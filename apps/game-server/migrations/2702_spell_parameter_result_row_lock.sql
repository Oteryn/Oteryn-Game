-- Parameter-result replay uses SELECT ... FOR SHARE, which requires UPDATE on
-- at least one column. Grant only an identity column; the existing immutable
-- trigger still rejects every UPDATE, including a no-op identity write.
GRANT UPDATE (game_session_id) ON game_spell_parameter_results
    TO oteryn_game_runtime;
