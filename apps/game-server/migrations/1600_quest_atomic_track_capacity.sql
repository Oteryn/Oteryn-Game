-- ENGNPC: reuse QUESTL2's sixteen-effect bound for every primary quest receipt.
-- Preserve the canonical empty-reward/world-change arms and array shape guards.
-- All source, fencing, revision and deferred track/receipt guards remain active.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
        cardinality(track_keys) BETWEEN 0 AND 16
        AND (cardinality(track_keys) > 0 OR completes
            OR transition_key LIKE 'oteryn:quest-reward/%'
            OR (transition_key = 'oteryn:quest-transition/worldchange/benevola/handover'
                AND quest_key = 'oteryn:quest/worldchange/benevola'
                AND cause_kind = 'command' AND NOT completes))
        AND (cardinality(track_keys) = 0
            OR (array_ndims(track_keys) = 1 AND array_lower(track_keys, 1) = 1
                AND array_ndims(values_before) = 1 AND array_lower(values_before, 1) = 1
                AND array_ndims(values_after) = 1 AND array_lower(values_after, 1) = 1))
        AND cardinality(values_before) = cardinality(track_keys)
        AND cardinality(values_after) = cardinality(track_keys)
        AND array_position(track_keys, NULL) IS NULL
        AND array_position(values_before, NULL) IS NULL
        AND array_position(values_after, NULL) IS NULL);

-- Extend the primary/combined source guards without replacing their evolved
-- proof bodies. Foreign participants still have their independent eight-track
-- table CHECK and an explicit caller bound on the shared effect validator.
DO $migration$
DECLARE v_function TEXT; v_old TEXT; v_new TEXT; v_body TEXT;
BEGIN
    FOR v_function, v_old, v_new IN
        SELECT * FROM (VALUES
            ('game_quest_reward_participants_complete()',
             'p),0)>8', 'p),0)>16'),
            ('game_quest_reward_state_ops_complete()',
             'IF v_count>8 THEN', 'IF v_count>16 THEN'),
            ('game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])',
             'cardinality(v_keys) NOT BETWEEN 1 AND 8',
             'cardinality(v_keys) NOT BETWEEN 1 AND 16'),
            ('game_quest_reward_participant_complete()',
             'OR NOT game_quest_reward_effects_valid(NEW.plan,NEW.track_keys',
             'OR cardinality(NEW.track_keys)>8 OR NOT game_quest_reward_effects_valid(NEW.plan,NEW.track_keys')
        ) AS patches(function_name, old_text, new_text)
    LOOP
        v_body := pg_get_functiondef(v_function::regprocedure);
        IF (length(v_body)-length(replace(v_body,v_old,'')))/length(v_old) <> 1 THEN
            RAISE EXCEPTION 'missing or ambiguous quest capacity guard: %', v_function;
        END IF;
        EXECUTE replace(v_body,v_old,v_new);
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',
                       v_function, current_schema());
    END LOOP;
END $migration$;
