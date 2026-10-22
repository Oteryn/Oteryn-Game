-- TUTORIAL-1 fix: admission reads the pending arrival with SELECT ... FOR UPDATE, and
-- PostgreSQL requires UPDATE privilege on at least one column for a row lock. Runtime
-- held only SELECT, INSERT, DELETE, so every fresh admission was refused under the
-- least-privilege role. Grant one column so the lock is allowed, and reject every real
-- UPDATE so the row stays insert/delete-only as 0136 intended.
GRANT UPDATE (occurrence_id) ON game_character_pending_arrivals TO oteryn_game_runtime;

CREATE FUNCTION game_character_pending_arrival_reject_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'pending arrivals are insert/delete-only' USING ERRCODE = '23514';
END;
$$;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_pending_arrival_reject_update() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END;
$$;
CREATE TRIGGER game_character_pending_arrival_immutable BEFORE UPDATE
    ON game_character_pending_arrivals FOR EACH ROW
    EXECUTE FUNCTION game_character_pending_arrival_reject_update();
REVOKE ALL ON FUNCTION game_character_pending_arrival_reject_update() FROM PUBLIC;
