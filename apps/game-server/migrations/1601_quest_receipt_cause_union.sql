-- Preserve the union of independently owned trade and timer continuations.
-- Ordinal checks and deferred source-proof guards remain unchanged.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipts_cause_kind_check;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_receipts_cause_kind_check
    CHECK (cause_kind IN ('command','use','step','claim_obligation','creature_death','trade_obligation','server_timer'));
