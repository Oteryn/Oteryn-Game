CREATE TABLE game_boss_death_achievement_pending (
    scope_key BYTEA NOT NULL CHECK (octet_length(scope_key) = 33),
    death BYTEA PRIMARY KEY CHECK (octet_length(death) IN (52, 53)),
    generation NUMERIC(20, 0) NOT NULL CHECK (generation BETWEEN 1 AND 18446744073709551615),
    boss SMALLINT NOT NULL CHECK (boss BETWEEN 1 AND 3),
    recipients JSONB NOT NULL CHECK (jsonb_typeof(recipients) = 'array')
);
CREATE INDEX game_boss_death_achievement_pending_scope
    ON game_boss_death_achievement_pending (scope_key, death);
REVOKE ALL ON game_boss_death_achievement_pending FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_boss_death_achievement_pending TO oteryn_game_runtime;
-- PostgreSQL requires a column-level UPDATE grant to lock a row for consumption.
GRANT UPDATE (death) ON game_boss_death_achievement_pending TO oteryn_game_runtime;
GRANT SELECT ON game_boss_death_achievement_pending TO oteryn_game_control;

ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN boss_achievement_explicit_logout BOOLEAN NOT NULL DEFAULT FALSE;

-- A sold Character can still prove its original earning account via the
-- locked retained roster and the original session. Ordinary grants continue
-- to require the active Character root to belong to the receiving account.
DO $migration$
DECLARE body TEXT;
    anchor CONSTANT TEXT := 'AND r.lifecycle = 1)';
BEGIN
    body := pg_get_functiondef('game_account_achievement_request_guard()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'missing original achievement account guard';
    END IF;
    EXECUTE replace(body,anchor,anchor || $proof$
       AND NOT EXISTS (
           SELECT 1
           FROM game_boss_death_achievement_pending p
           CROSS JOIN LATERAL jsonb_array_elements(p.recipients) recipient
           JOIN game_durability_reconnect_sessions s
             ON s.game_session_id::text=recipient->>1
            AND s.character_lease_generation::text=recipient->>2
           JOIN game_character_roots r ON r.character_id=s.character_id
           WHERE s.character_id=NEW.character_id AND s.account_id=NEW.account_id
             AND recipient->>0=NEW.character_id::text AND recipient->>3=NEW.account_id::text
             AND NOT s.boss_achievement_explicit_logout AND r.lifecycle=1
             AND NEW.source_event_id=sha256(uuid_send(NEW.character_id)||p.death)
             AND CASE p.boss
                 WHEN 1 THEN NEW.source_kind='oteryn:oberon-death'
                    AND NEW.achievement_key IN ('oteryn:achievement/master_debater',
                                               'oteryn:achievement/millennial_falcon')
                 WHEN 2 THEN NEW.source_kind='oteryn:boss-death'
                    AND NEW.achievement_key='oteryn:achievement/a_study_in_scarlett'
                 WHEN 3 THEN NEW.source_kind='oteryn:boss-death'
                    AND NEW.achievement_key='oteryn:achievement/museum_goer'
                 ELSE FALSE END
       )
$proof$);
END $migration$;
