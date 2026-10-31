-- Extend the existing build receipt cause owner; preserve every existing cause rule.
ALTER TABLE game_character_build_receipts
    DROP CONSTRAINT game_character_build_receipts_cause_check,
    ADD CONSTRAINT game_character_build_receipts_cause_check CHECK (
        cause IN ('training','resting_stamina','vocation_choice','newhaven_vocation_choice',
                  'vocation_trial','promotion','logout','offline_settlement')),
    ADD CONSTRAINT game_character_build_receipts_newhaven_direction CHECK (
        cause <> 'newhaven_vocation_choice' OR
        (vocation_before = 'none' AND vocation_after IN ('sorcerer','druid','paladin','knight','monk')));
-- Existing direction CASE returns NULL for this new cause; the exact direction is enforced
-- above, while the Rust build owner still verifies cumulative-progress conversion.
