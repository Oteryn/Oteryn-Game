-- M51: retain command ordinals 1..u64::MAX; native STEP uses u64::MAX +
-- its compact movement occurrence. The complete actor/revision/epoch witness is
-- bound into the receipt payload, so a compact-id collision refuses replay.
ALTER TABLE game_arena_match_receipts
    DROP CONSTRAINT game_arena_match_receipts_command_id_check,
    DROP CONSTRAINT game_arena_match_receipts_payload_check,
    ADD CONSTRAINT game_arena_match_cause_ordinal_check
        CHECK(command_id BETWEEN 1 AND 36893488147419103230),
    ADD CONSTRAINT game_arena_match_cause_payload_check
        CHECK((command_id<=18446744073709551615 AND octet_length(payload)<=32)
           OR (command_id>18446744073709551615 AND octet_length(payload) BETWEEN 33 AND 64));
