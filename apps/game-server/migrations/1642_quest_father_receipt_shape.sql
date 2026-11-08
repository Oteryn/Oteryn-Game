-- Keep ENGNPC's generic sixteen-track capacity and all its source guards.
-- Restore 1349's exact higher-arity Father exception as an additional CHECK;
-- migration 1600 remains immutable and every other quest keeps its full bound.
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_father_receipt_shape CHECK (
        quest_key <> 'oteryn:quest.a_father_s_burden_quest'
        OR cardinality(track_keys) <= 8
        OR (cause_kind = 'command'
            AND NOT completes
            AND transition_key = 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'
            AND track_keys = ARRAY[
                'oteryn:quest-progress/quest/u8_6/afathers_burden/cloth',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/crystal',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/iron',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/progress',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/root',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/scale',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/silk',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/sinew',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/status',
                'oteryn:quest-progress/quest/u8_6/afathers_burden/wood'
            ]::text[]
            AND values_after = ARRAY[1,1,1,0,1,1,1,1,1,1,1]::bigint[])
    );
