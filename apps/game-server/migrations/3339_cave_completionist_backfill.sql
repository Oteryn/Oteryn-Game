-- Backfill from the two immutable quest purchases. A transferred Character
-- still proves the seller's receipt, but its Character-owned progress travels
-- with the Character and stays untouched when it belongs to another account.
-- Extend the current owner guard so the retained boss-death proof stays active.
DO $migration$
DECLARE body TEXT;
    anchor CONSTANT TEXT := E'       OR NOT EXISTS (\n            SELECT 1 FROM game_account_achievements f';
BEGIN
    body := pg_get_functiondef('game_account_achievement_request_guard()'::regprocedure);
    IF (length(body) - length(replace(body, anchor, ''))) / length(anchor) <> 1 THEN
        RAISE EXCEPTION 'missing consumed achievement fact guard';
    END IF;
    EXECUTE replace(body, anchor, $proof$       AND NOT (
           NEW.source_kind = 'oteryn:cave-explorer-backfill'
           AND NEW.source_event_id = uuid_send(NEW.account_id)
           AND NEW.achievement_key = 'oteryn:achievement/cave_completionist'
           AND current_user = (
               SELECT pg_get_userbyid(relowner) FROM pg_class WHERE oid = TG_RELID
           )
           AND (
               EXISTS (
                   SELECT 1 FROM game_character_roots r
                    WHERE r.character_id = NEW.character_id AND r.account_id = NEW.account_id
               )
               OR EXISTS (
                   SELECT 1 FROM game_account_appearance_unlocks u
                   JOIN game_account_appearance_unlock_receipts r
                     USING (grant_occurrence_id, account_id, unlock_key)
                    WHERE u.account_id = NEW.account_id
                      AND u.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/2'
                      AND r.granting_character_id = NEW.character_id
                      AND r.source_kind = 'quest'
                      AND r.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
               )
           )
       )
$proof$ || anchor);
END $migration$;

-- Live achievement writers lock their Character before the request table.
-- Keep the locked lifecycle and ownership for the progress write as well.
CREATE TEMP TABLE cave_completionist_backfill_eligible ON COMMIT DROP AS
    SELECT second.account_id, second.granting_character_id AS character_id,
           holder.account_id = second.account_id AND holder.lifecycle <> 2 AS write_progress
    FROM game_account_appearance_unlock_receipts AS second
    JOIN game_account_appearance_unlocks AS second_unlock
      USING (grant_occurrence_id, account_id, unlock_key)
    JOIN game_account_appearance_unlocks AS first_unlock
      ON first_unlock.account_id = second.account_id
     AND first_unlock.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/1'
    JOIN game_account_appearance_unlock_receipts AS first
      ON first.grant_occurrence_id = first_unlock.grant_occurrence_id
    JOIN game_character_roots AS holder ON holder.character_id = second.granting_character_id
    WHERE second.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/2'
      AND first.source_kind = 'quest' AND second.source_kind = 'quest'
      AND first.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
      AND second.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
    FOR SHARE OF holder;

-- Only the migration writes historical requests against Bazaar-locked roots.
ALTER TABLE game_account_achievement_grant_requests
    DISABLE TRIGGER game_bazaar_frozen_character;

WITH requests AS (
    INSERT INTO game_account_achievement_grant_requests
      (source_kind, source_event_id, achievement_key, achievement_revision,
       account_id, character_id, earned_at)
    SELECT 'oteryn:cave-explorer-backfill', uuid_send(account_id),
           'oteryn:achievement/cave_completionist', '1',
           account_id, character_id, floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint
    FROM cave_completionist_backfill_eligible AS eligible
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
JOIN cave_completionist_backfill_eligible AS eligible USING (account_id, character_id)
WHERE eligible.write_progress
ON CONFLICT (character_id, achievement_key) DO UPDATE
SET progress = EXCLUDED.progress
WHERE progress.progress < EXCLUDED.progress;

SET CONSTRAINTS game_account_achievement_request_consumed IMMEDIATE;
ALTER TABLE game_account_achievement_grant_requests
    ENABLE TRIGGER game_bazaar_frozen_character;
