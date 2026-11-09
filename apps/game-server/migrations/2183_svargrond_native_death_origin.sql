-- M51: both real native callbacks retain the full source binding.
-- Crystal201189f creaturescripts_arena_kill.lua:2-38 and
-- movements_arena_pit.lua:24-64. Preserve existing STEP receipts and all guards.
ALTER TABLE game_arena_match_receipts
    RENAME COLUMN native_step_binding TO native_origin_binding;
