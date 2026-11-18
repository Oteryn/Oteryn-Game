-- A source STEP can arm one return against the same Character's active stage.
-- Crystal 201189f8 mission06_run_like_wolf.lua:137-163.
CREATE TABLE game_quest_personal_returns (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    return_key TEXT NOT NULL CHECK (octet_length(return_key) BETWEEN 1 AND 128),
    occurrence UUID NOT NULL UNIQUE DEFAULT game_character_uuid_v7()
        CHECK (game_character_is_uuid_v7(occurrence)),
    world_id UUID NOT NULL,
    map_revision TEXT NOT NULL CHECK (map_revision ~ '^sha256:[0-9a-f]{64}$'),
    binding BYTEA NOT NULL CHECK (octet_length(binding) = 32),
    due_at TIMESTAMPTZ NOT NULL,
    committed_revision NUMERIC(20,0),
    settled_xact_id xid8,
    PRIMARY KEY (character_id, return_key),
    CHECK ((committed_revision IS NULL) = (settled_xact_id IS NULL))
);

-- Preserve the existing shared-switch proof and admit the independent durable
-- personal timer only when it settled in the receipt's own transaction.
CREATE OR REPLACE FUNCTION game_quest_timer_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind = 'server_timer' THEN
        IF NEW.transition_key LIKE 'oteryn:quest-personal-return/%' THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_quest_personal_returns p
                WHERE p.character_id = NEW.character_id AND p.occurrence = NEW.cause_id
                    AND p.return_key = NEW.transition_key AND p.due_at <= clock_timestamp()
                    AND p.committed_revision = NEW.committed_character_revision
                    AND p.settled_xact_id = NEW.created_xact_id
            ) THEN
                RAISE EXCEPTION 'personal quest return receipt lacks its same-XID timer proof'
                    USING ERRCODE = '23514';
            END IF;
        ELSE
            IF NOT EXISTS (
                SELECT 1 FROM game_quest_timer_observations o
                WHERE o.character_id = NEW.character_id AND o.lease_id = NEW.cause_id
                    AND o.recipe = NEW.transition_key AND NEW.cause_ordinal = 0
                    AND o.committed_revision = NEW.committed_character_revision
                    AND o.created_xact_id = NEW.created_xact_id
            ) THEN
                RAISE EXCEPTION 'server timer receipt lacks its original lease proof'
                    USING ERRCODE = '23514';
            END IF;
            PERFORM game_quest_timer_complete(NEW.cause_id, NEW.transition_key);
        END IF;
    END IF;
    RETURN NULL;
END $$;

REVOKE ALL ON game_quest_personal_returns FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_quest_personal_returns TO oteryn_game_runtime;
GRANT SELECT ON game_quest_personal_returns TO oteryn_game_control;
