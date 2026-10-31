-- W15c: a native room visit is an obligation of its ordinary QuestState receipt.
-- Quest storage remains the sole entitlement/progress owner. Cleanup may free an
-- expired room while its offline Character's return remains durable.
CREATE TABLE game_character_grizzly_boss_entries (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK (cause_ordinal BETWEEN 1 AND 18446744073709551615),
    transition_key TEXT NOT NULL,
    boss_id SMALLINT NOT NULL CHECK (boss_id BETWEEN 1 AND 30),
    boss_track TEXT NOT NULL CHECK (octet_length(boss_track) BETWEEN 1 AND 128),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation BETWEEN 1 AND 18446744073709551615),
    exit_x INTEGER NOT NULL CHECK (exit_x BETWEEN 0 AND 65535),
    exit_y INTEGER NOT NULL CHECK (exit_y BETWEEN 0 AND 65535),
    exit_floor SMALLINT NOT NULL CHECK (exit_floor BETWEEN 0 AND 15),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^sha256:[0-9a-f]{64}$'),
    frame_digest BYTEA NOT NULL CHECK (octet_length(frame_digest) = 32),
    entered_at BIGINT NOT NULL CHECK (entered_at >= 0),
    deadline_at BIGINT NOT NULL CHECK (deadline_at = entered_at + 600000),
    state TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (state IN ('ACTIVE','RETURN_PENDING','CLOSED')),
    PRIMARY KEY(character_id,cause_id,cause_ordinal),
    CHECK (transition_key = 'oteryn:quest-transition/grizzly/enter-boss-' || boss_id::text),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
CREATE UNIQUE INDEX game_grizzly_one_occupied_room
    ON game_character_grizzly_boss_entries(world_id,channel_id,boss_id) WHERE state='ACTIVE';
CREATE UNIQUE INDEX game_grizzly_one_open_visit
    ON game_character_grizzly_boss_entries(character_id) WHERE state IN ('ACTIVE','RETURN_PENDING');
REVOKE ALL ON game_character_grizzly_boss_entries FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_grizzly_boss_entries TO oteryn_game_runtime;

CREATE FUNCTION game_grizzly_boss_entry_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
    IF TG_OP='INSERT' AND NEW.state='ACTIVE' AND EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)
            = (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
          AND r.quest_key='oteryn:quest.killing_in_the_name_of_quest'
          AND r.cause_kind='step' AND r.created_xact_id=pg_current_xact_id()
          AND r.values_before[array_position(r.track_keys,NEW.boss_track)]=1
          AND r.values_after[array_position(r.track_keys,NEW.boss_track)]=2
    ) THEN
        RETURN NEW;
    ELSIF TG_OP='UPDATE' AND (to_jsonb(NEW)-'state')=(to_jsonb(OLD)-'state')
        AND ((OLD.state='ACTIVE' AND NEW.state IN ('RETURN_PENDING','CLOSED'))
             OR (OLD.state='RETURN_PENDING' AND NEW.state='CLOSED')) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Grizzly visit requires its entry receipt and immutable return identity'
        USING ERRCODE='23514';
END;
$$;
REVOKE ALL ON FUNCTION game_grizzly_boss_entry_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_grizzly_boss_entry_guard() TO oteryn_game_runtime;
CREATE TRIGGER game_grizzly_boss_entry_guard
BEFORE INSERT OR UPDATE OR DELETE ON game_character_grizzly_boss_entries
FOR EACH ROW EXECUTE FUNCTION game_grizzly_boss_entry_guard();
