-- L2: an eleven-track quest write remains one receipt and one Character revision.
-- Extend RL-02 without changing any receipt, authority or consistency guard.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipt_effects_shape,
    ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
        cardinality(track_keys) BETWEEN 0 AND 16
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
