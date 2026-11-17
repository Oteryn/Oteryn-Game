-- M37: free source NPC teleport responses have their own receipt, not fake quest progress.
CREATE TABLE game_quest_npc_teleport_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id uuid NOT NULL,
    command_id numeric(20,0) NOT NULL CHECK(command_id>=1),
    transition_key text NOT NULL CHECK(transition_key LIKE 'oteryn:quest-npc/%'),
    request_binding bytea NOT NULL CHECK(octet_length(request_binding)=33),
    program_digest bytea NOT NULL CHECK(octet_length(program_digest)=32),
    occurrence_id uuid NOT NULL,
    world_id uuid NOT NULL,
    tile_x integer NOT NULL CHECK(tile_x BETWEEN 0 AND 65535),
    tile_y integer NOT NULL CHECK(tile_y BETWEEN 0 AND 65535),
    tile_floor smallint NOT NULL CHECK(tile_floor BETWEEN -15 AND 0),
    reply text[] NOT NULL CHECK(cardinality(reply)<=32),
    next_topic integer NOT NULL CHECK(next_topic>=0),
    end_conversation boolean NOT NULL,
    end_delay_ms integer NOT NULL CHECK(end_delay_ms BETWEEN 0 AND 60000),
    quest_changed boolean NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,game_session_id,command_id),
    UNIQUE(character_id,occurrence_id)
);
REVOKE ALL ON game_quest_npc_teleport_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_npc_teleport_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_quest_npc_teleport_receipts TO oteryn_game_control;
ALTER TABLE game_character_pending_arrivals ADD COLUMN source_npc_teleport boolean NOT NULL DEFAULT false;

CREATE FUNCTION game_quest_npc_teleport_receipt_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR EXISTS (
        SELECT 1 FROM unnest(NEW.reply) line WHERE line IS NULL OR octet_length(line)>2048
    ) OR NOT EXISTS (
        SELECT 1 FROM game_character_pending_arrivals a WHERE a.character_id=NEW.character_id
          AND a.occurrence_id=NEW.occurrence_id AND a.world_id=NEW.world_id
          AND a.tile_x=NEW.tile_x AND a.tile_y=NEW.tile_y AND a.tile_floor=NEW.tile_floor
          AND a.source_npc_teleport
    ) OR NEW.quest_changed IS DISTINCT FROM EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        JOIN game_quest_npc_program_receipts n ON n.character_id=r.character_id
          AND n.game_session_id=r.cause_id AND n.command_id=r.cause_ordinal AND n.transition_key=r.transition_key
        WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id
          AND r.cause_ordinal=NEW.command_id AND r.transition_key=NEW.transition_key
          AND r.cause_kind='command' AND r.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'NPC teleport response requires its atomic arrival and exact quest-write disposition' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION game_quest_npc_teleport_arrival_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='UPDATE' AND OLD.source_npc_teleport THEN
        IF NEW IS NOT DISTINCT FROM OLD THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'Source NPC arrival is immutable until consumed' USING ERRCODE='23514';
    END IF;
    IF NEW.source_npc_teleport AND NOT EXISTS (
        SELECT 1 FROM game_quest_npc_teleport_receipts r WHERE r.character_id=NEW.character_id
          AND r.occurrence_id=NEW.occurrence_id AND r.world_id=NEW.world_id
          AND r.tile_x=NEW.tile_x AND r.tile_y=NEW.tile_y AND r.tile_floor=NEW.tile_floor
          AND r.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'Source NPC arrival requires its same-transaction response receipt' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_npc_teleport_receipt_guard() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_npc_teleport_arrival_guard() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_npc_teleport_receipt_guard() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_npc_teleport_arrival_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_quest_npc_teleport_response_proven
AFTER INSERT ON game_quest_npc_teleport_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_quest_npc_teleport_receipt_guard();
CREATE CONSTRAINT TRIGGER game_quest_npc_teleport_arrival_proven
AFTER INSERT OR UPDATE ON game_character_pending_arrivals DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_quest_npc_teleport_arrival_guard();
