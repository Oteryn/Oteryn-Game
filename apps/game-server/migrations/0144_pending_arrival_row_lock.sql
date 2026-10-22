-- TUTORIAL-1 fix: admission reads the pending arrival with SELECT ... FOR UPDATE, and
-- PostgreSQL requires UPDATE privilege on at least one column for a row lock. Runtime
-- held only SELECT, INSERT, DELETE, so every fresh admission was refused under the
-- least-privilege role. Grant UPDATE on the occurrence column only, so the lock is
-- allowed; runtime code still never updates this row (it inserts and deletes it).
GRANT UPDATE (occurrence_id) ON game_character_pending_arrivals TO oteryn_game_runtime;
