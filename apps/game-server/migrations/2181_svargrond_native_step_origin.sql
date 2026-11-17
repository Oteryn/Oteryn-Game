-- M51: Crystal201189f data-global/scripts/quests/svargrond_arena/
-- movements_arena_pit.lua:52-64 is a native STEP, not a client arena command.
-- Keep the full native binding next to its compact receipt id. A hash collision
-- must refuse rather than replay another step or a client command's result.
ALTER TABLE game_arena_match_receipts
    ADD COLUMN native_step_binding BYTEA
    CHECK(native_step_binding IS NULL OR octet_length(native_step_binding) BETWEEN 1 AND 256);
-- Existing immutable/stamp triggers, same-transaction state proof and privileges
-- remain authoritative; no historical migration or receipt is rewritten.
