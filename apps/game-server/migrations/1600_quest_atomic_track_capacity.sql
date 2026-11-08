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
