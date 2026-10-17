-- SELECT ... FOR UPDATE OF a schedule requires UPDATE on at least one column.
-- Runtime drains lock the schedule together with its Item. These schedules are
-- append-only: the existing BEFORE UPDATE/DELETE and BEFORE TRUNCATE triggers
-- continue to reject mutation. Grant only the identity column, not the payload.
GRANT UPDATE (item_instance_id) ON game_spell_item_temporal_schedules
    TO oteryn_game_runtime;
GRANT UPDATE (item_instance_id) ON game_spell_field_temporal_schedules
    TO oteryn_game_runtime;
