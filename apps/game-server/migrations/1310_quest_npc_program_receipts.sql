-- Standalone QNPCFX bootstrap until QBASE 1200 is composed.
-- QBASE owns the canonical table; leave its definition untouched when present.
DO $qnpcfx_bootstrap$
BEGIN
IF to_regclass('game_quest_npc_program_receipts') IS NULL THEN
-- NPC storage-only responses share the exact physical quest receipt transaction.
CREATE TABLE game_quest_npc_program_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id uuid NOT NULL,
    command_id numeric(20,0) NOT NULL CHECK(command_id>=1),
    transition_key text NOT NULL CHECK(transition_key LIKE 'oteryn:quest-npc/%'),
    reply text[] NOT NULL,
    next_topic integer NOT NULL CHECK(next_topic>=0),
    PRIMARY KEY(character_id,game_session_id,command_id,transition_key),
    FOREIGN KEY(character_id,game_session_id,command_id,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
REVOKE ALL ON game_quest_npc_program_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_npc_program_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_quest_npc_program_receipts TO oteryn_game_control;

CREATE FUNCTION game_quest_npc_program_response_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF cardinality(NEW.reply)>32 OR EXISTS (
        SELECT 1 FROM unnest(NEW.reply) line
        WHERE line IS NULL OR octet_length(line)>2048
    ) OR NOT EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id
          AND r.cause_ordinal=NEW.command_id AND r.transition_key=NEW.transition_key
          AND r.cause_kind='command' AND r.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'NPC response requires bounded lines and its same-transaction quest receipt'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_npc_program_response_guard() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_npc_program_response_guard() FROM PUBLIC;
CREATE TRIGGER game_quest_npc_program_response_proven
BEFORE INSERT ON game_quest_npc_program_receipts
FOR EACH ROW EXECUTE FUNCTION game_quest_npc_program_response_guard();


END IF;
END
$qnpcfx_bootstrap$;
