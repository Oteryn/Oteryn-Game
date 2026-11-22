-- The pre-achievement Cave Explorer purchases already recorded both account
-- appearance grants. Award their capped progress and account fact once, without
-- replaying any quest transition, charging a fee, or consuming Spike fame.
WITH eligible AS (
    SELECT second.account_id, holder.character_id
    FROM game_account_appearance_unlock_receipts AS second
    JOIN game_account_appearance_unlocks AS second_unlock
      USING (grant_occurrence_id, account_id, unlock_key)
    JOIN game_account_appearance_unlocks AS first_unlock
      ON first_unlock.account_id = second.account_id
     AND first_unlock.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/1'
    JOIN game_account_appearance_unlock_receipts AS first
      ON first.grant_occurrence_id = first_unlock.grant_occurrence_id
    JOIN LATERAL (
        SELECT root.character_id
        FROM game_character_roots AS root
        WHERE root.account_id = second.account_id AND root.lifecycle = 1
        ORDER BY (root.character_id = second.granting_character_id) DESC, root.character_id
        LIMIT 1
    ) AS holder ON true
    WHERE second.unlock_key = 'oteryn:outfit.cave_explorer/quest-grant/2'
      AND first.source_kind = 'quest' AND second.source_kind = 'quest'
      AND first.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
      AND second.content_revision = 'crystal-9f5a72c-quest-rewards-v1'
      AND NOT EXISTS (
          SELECT 1 FROM game_account_achievements AS fact
          WHERE fact.account_id = second.account_id
            AND fact.achievement_key = 'oteryn:achievement/cave_completionist'
      )
), requests AS (
    INSERT INTO game_account_achievement_grant_requests
      (source_kind, source_event_id, achievement_key, achievement_revision,
       account_id, character_id, earned_at)
    SELECT 'oteryn:cave-explorer-backfill', uuid_send(account_id),
           'oteryn:achievement/cave_completionist', '1',
           account_id, character_id, floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint
    FROM eligible
    ON CONFLICT (source_kind, source_event_id, achievement_key) DO NOTHING
    RETURNING source_kind, source_event_id, achievement_key, account_id, character_id
), grants AS (
    INSERT INTO game_account_achievements
      (account_id, achievement_key, source_kind, source_event_id)
    SELECT account_id, achievement_key, source_kind, source_event_id FROM requests
    ON CONFLICT (account_id, achievement_key) DO NOTHING
)
INSERT INTO game_character_achievement_progress AS progress (character_id, achievement_key, progress)
SELECT character_id, achievement_key, 2 FROM requests
ON CONFLICT (character_id, achievement_key) DO UPDATE
SET progress = EXCLUDED.progress
WHERE progress.progress < EXCLUDED.progress;
