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

-- QUESTPC / NPC-LEGACY-1: read-compatible definition activation, no durable repinning.
-- Existing states/receipts/track rows are immutable. The catalogue retains each
-- exact pre-activation hash and absent-track default; only those hashes are read
-- compatibly by the fenced quest owner. Unknown hashes still require migration.
-- Father acceptance is one source operation: eleven tracks, one Character
-- revision and receipt. Every other receipt keeps the existing eight-track cap.
-- Rollback retains this additive constraint and the compatibility reader; an old
-- artifact unable to read the new definition must be rejected before activation.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
        (cardinality(track_keys) BETWEEN 0 AND 8
             OR (cause_kind = 'command'
                 AND NOT completes
                 AND quest_key = 'oteryn:quest.a_father_s_burden_quest'
                 AND transition_key = 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'
                 AND track_keys = ARRAY['oteryn:quest-progress/quest/u8_6/afathers_burden/cloth','oteryn:quest-progress/quest/u8_6/afathers_burden/crystal','oteryn:quest-progress/quest/u8_6/afathers_burden/iron','oteryn:quest-progress/quest/u8_6/afathers_burden/progress','oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log','oteryn:quest-progress/quest/u8_6/afathers_burden/root','oteryn:quest-progress/quest/u8_6/afathers_burden/scale','oteryn:quest-progress/quest/u8_6/afathers_burden/silk','oteryn:quest-progress/quest/u8_6/afathers_burden/sinew','oteryn:quest-progress/quest/u8_6/afathers_burden/status','oteryn:quest-progress/quest/u8_6/afathers_burden/wood']::text[]
                 AND values_after = ARRAY[1,1,1,0,1,1,1,1,1,1,1]::bigint[]))
        AND (cardinality(track_keys) > 0 OR completes)
        AND (cardinality(track_keys) = 0
             OR (array_ndims(track_keys) = 1 AND array_lower(track_keys, 1) = 1
                 AND array_ndims(values_before) = 1 AND array_lower(values_before, 1) = 1
                 AND array_ndims(values_after) = 1 AND array_lower(values_after, 1) = 1))
        AND cardinality(values_before) = cardinality(track_keys)
        AND cardinality(values_after) = cardinality(track_keys)
        AND array_position(track_keys, NULL) IS NULL
        AND array_position(values_before, NULL) IS NULL
        AND array_position(values_after, NULL) IS NULL);
