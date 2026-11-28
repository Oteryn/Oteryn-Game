-- Backfill from the two immutable quest purchases. A transferred Character
-- still proves the seller's receipt, but its Character-owned progress travels
-- with the Character and stays untouched when it belongs to another account.
-- Extend the current owner guard so the retained boss-death proof stays active.
-- grant_occurrence_id hashes the source command, so sorting those UUIDs
-- cannot recover purchase order. Match each grant to its command receipt.
CREATE OR REPLACE VIEW game_cave_completionist_purchasers AS
WITH purchases AS (
    SELECT u.account_id, r.granting_character_id, u.unlock_key, q.committed_at
    FROM game_account_appearance_unlocks u
    JOIN game_account_appearance_unlock_receipts r
      USING (grant_occurrence_id, account_id, unlock_key)
    JOIN game_quest_reward_sources s ON s.character_id = r.granting_character_id
      AND u.unlock_key = ANY(s.grant_keys)
    JOIN game_character_quest_receipts q
      USING (character_id, cause_id, cause_ordinal, transition_key)
    WHERE u.unlock_key IN ('oteryn:outfit.cave_explorer/quest-grant/1',
                           'oteryn:outfit.cave_explorer/quest-grant/2')
      AND r.source_kind = 'quest'
      AND r.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
      AND r.source_event_id = convert_to('oteryn:quest-callback-outfit:v1','UTF8')
          || decode('00','hex') || uuid_send(s.character_id)
          || convert_to(replace(jsonb_build_array(
              (SELECT jsonb_agg(get_byte(uuid_send(s.cause_id),n) ORDER BY n)
               FROM generate_series(0,15) n), s.cause_ordinal, s.transition_key)::text,
              ' ', ''), 'UTF8') || convert_to(u.unlock_key,'UTF8')
)
SELECT later.account_id, later.granting_character_id AS character_id,
       later.committed_at <> earlier.committed_at OR
           later.granting_character_id = earlier.granting_character_id AS completion_known
FROM purchases later JOIN purchases earlier USING (account_id)
WHERE earlier.unlock_key <> later.unlock_key
  AND (later.committed_at > earlier.committed_at OR
       (later.committed_at = earlier.committed_at
        AND later.unlock_key > earlier.unlock_key));

GRANT SELECT ON game_cave_completionist_purchasers TO oteryn_game_runtime;

DO $migration$
DECLARE body TEXT;
    legacy CONSTANT TEXT := $sql$SELECT 1 FROM game_account_appearance_unlocks u
               JOIN game_account_appearance_unlock_receipts r
                 USING (grant_occurrence_id, account_id, unlock_key)
                WHERE u.account_id = NEW.account_id
                  AND u.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/2'
                  AND r.granting_character_id = NEW.character_id
                  AND r.source_kind = 'quest'
                  AND r.content_revision = 'crystal-9f5a72c-quest-rewards-v1'$sql$;
    replacement CONSTANT TEXT := $sql$SELECT 1 FROM game_cave_completionist_purchasers p
                WHERE p.account_id = NEW.account_id AND p.character_id = NEW.character_id$sql$;
BEGIN
    body := pg_get_functiondef('game_account_achievement_request_guard()'::regprocedure);
    IF strpos(body, legacy) > 0 THEN
        EXECUTE replace(body, legacy, replacement);
    ELSIF strpos(body, replacement) = 0 THEN
        RAISE EXCEPTION 'missing Cave Completionist purchase proof';
    END IF;
END $migration$;

-- Live achievement writers lock their Character before the request table.
-- Keep the locked lifecycle and ownership for the progress write as well.
CREATE TEMP TABLE cave_completionist_reconciliation ON COMMIT DROP AS
    SELECT p.account_id, p.character_id, p.completion_known,
           p.completion_known AND holder.account_id = p.account_id AND holder.lifecycle <> 2 AS write_progress
    FROM game_cave_completionist_purchasers p
    JOIN game_character_roots holder USING (character_id)
    FOR SHARE OF holder;

-- 3339 bound the account request and the completed counter to addon 2.
-- Repair only facts derived from that migration. Preserve earned_at and the
-- account fact's immutable source, and leave independent completions alone.
CREATE TEMP TABLE cave_completionist_previous ON COMMIT DROP AS
    SELECT r.account_id, r.character_id AS previous_character, e.character_id,
           e.write_progress, holder.account_id = r.account_id AND holder.lifecycle <> 2 AS can_clear
    FROM cave_completionist_reconciliation e
    JOIN game_account_achievements f USING (account_id)
    JOIN game_account_achievement_grant_requests r
      USING (account_id, achievement_key, source_kind, source_event_id)
    JOIN game_character_roots holder ON holder.character_id = r.character_id
    WHERE f.achievement_key = 'oteryn:achievement/cave_completionist'
      AND f.source_kind = 'oteryn:cave-explorer-backfill'
      AND f.source_event_id = uuid_send(f.account_id)
      AND (r.character_id <> e.character_id OR NOT e.completion_known)
    FOR SHARE OF holder;

DELETE FROM game_character_achievement_progress p
USING cave_completionist_previous old
WHERE old.can_clear AND p.character_id = old.previous_character
  AND p.achievement_key = 'oteryn:achievement/cave_completionist' AND p.progress = 2
  AND NOT EXISTS (
      SELECT 1 FROM game_account_achievement_grant_requests r
      WHERE r.character_id = p.character_id AND r.achievement_key = p.achievement_key
        AND (r.account_id <> old.account_id OR r.source_kind <> 'oteryn:cave-explorer-backfill')
  );

-- Only the migration writes historical requests against Bazaar-locked roots.
ALTER TABLE game_account_achievement_grant_requests
    DISABLE TRIGGER game_bazaar_frozen_character;

ALTER TABLE game_account_achievement_grant_requests
    DISABLE TRIGGER game_account_achievement_request_immutable;

UPDATE game_account_achievement_grant_requests r
SET character_id = old.character_id
FROM cave_completionist_previous old
WHERE r.account_id = old.account_id AND r.character_id <> old.character_id
  AND r.achievement_key = 'oteryn:achievement/cave_completionist'
  AND r.source_kind = 'oteryn:cave-explorer-backfill'
  AND r.source_event_id = uuid_send(old.account_id);

ALTER TABLE game_account_achievement_grant_requests
    ENABLE TRIGGER game_account_achievement_request_immutable;

WITH requests AS (
    INSERT INTO game_account_achievement_grant_requests
      (source_kind, source_event_id, achievement_key, achievement_revision,
       account_id, character_id, earned_at)
    SELECT 'oteryn:cave-explorer-backfill', uuid_send(account_id),
           'oteryn:achievement/cave_completionist', '1',
           account_id, character_id, floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint
    FROM cave_completionist_reconciliation AS eligible
    WHERE NOT EXISTS (
        SELECT 1 FROM game_account_achievements AS fact
        WHERE fact.account_id = eligible.account_id
          AND fact.achievement_key = 'oteryn:achievement/cave_completionist'
    )
    ON CONFLICT (source_kind, source_event_id, achievement_key) DO NOTHING
    RETURNING source_kind, source_event_id, achievement_key, account_id, character_id
), grants AS (
    INSERT INTO game_account_achievements
      (account_id, achievement_key, source_kind, source_event_id)
    SELECT account_id, achievement_key, source_kind, source_event_id FROM requests
    ON CONFLICT (account_id, achievement_key) DO NOTHING
)
INSERT INTO game_character_achievement_progress AS progress (character_id, achievement_key, progress)
SELECT requests.character_id, requests.achievement_key, 2
FROM requests
JOIN cave_completionist_reconciliation AS eligible USING (account_id, character_id)
WHERE eligible.write_progress
UNION
SELECT character_id, 'oteryn:achievement/cave_completionist', 2
FROM cave_completionist_previous WHERE write_progress
ON CONFLICT (character_id, achievement_key) DO UPDATE
SET progress = EXCLUDED.progress
WHERE progress.progress < EXCLUDED.progress;

SET CONSTRAINTS game_account_achievement_request_consumed IMMEDIATE;
ALTER TABLE game_account_achievement_grant_requests
    ENABLE TRIGGER game_bazaar_frozen_character;
