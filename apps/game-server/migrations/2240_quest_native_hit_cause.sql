-- Successful native damage has its own occurrence namespace, independent of USE/death.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipts_cause_kind_check;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_receipts_cause_kind_check
    CHECK (cause_kind IN ('command', 'use', 'step', 'claim_obligation',
                         'creature_death', 'server_timer', 'native_hit'));

ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_quest_timer_cause_ordinal;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_quest_native_hit_cause_ordinal
    CHECK ((cause_kind IN ('creature_death', 'server_timer', 'native_hit')) = (cause_ordinal = 0));
