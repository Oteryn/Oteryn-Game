-- QNPCFX / NPC-LEGACY-1: read-compatible definition activation, no durable repinning.
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
        -- Preserve main 1003's reward-only arm and its deferred source proof.
        AND (cardinality(track_keys) > 0 OR completes
             OR transition_key LIKE 'oteryn:quest-reward/%')
        AND (cardinality(track_keys) = 0
             OR (array_ndims(track_keys) = 1 AND array_lower(track_keys, 1) = 1
                 AND array_ndims(values_before) = 1 AND array_lower(values_before, 1) = 1
                 AND array_ndims(values_after) = 1 AND array_lower(values_after, 1) = 1))
        AND cardinality(values_before) = cardinality(track_keys)
        AND cardinality(values_after) = cardinality(track_keys)
        AND array_position(track_keys, NULL) IS NULL
        AND array_position(values_before, NULL) IS NULL
        AND array_position(values_after, NULL) IS NULL);
