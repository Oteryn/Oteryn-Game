-- An optional immutable Channel landing for a content-authored arena USE.
-- Other return reasons retain the original exit or the proven death temple.
ALTER TABLE game_activity_runtime_placements
    ADD COLUMN use_exit_cell INTEGER[] CHECK (use_exit_cell IS NULL OR (
        array_ndims(use_exit_cell) = 1
        AND array_lower(use_exit_cell, 1) = 1 AND array_upper(use_exit_cell, 1) = 3
        AND cardinality(use_exit_cell) = 3 AND array_position(use_exit_cell, NULL) IS NULL
        AND use_exit_cell[3] BETWEEN -32768 AND 32767
    ));

ALTER TABLE game_activity_runtime_returns
    ADD COLUMN native_use_exit BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CHECK (NOT native_use_exit OR (
        reason = 1 AND command_id IS NOT NULL AND native_step_exit IS NULL
    ));

-- Preserve all existing session, transport, security and death checks.
DO $migration$
DECLARE
    body TEXT;
    old_guard TEXT := 'AND (NEW.reason=5 OR (NEW.return_x,NEW.return_y,NEW.return_floor)=(q.exit_x,q.exit_y,q.exit_floor))';
    new_guard TEXT := $guard$
        AND (NEW.reason=5 OR
            (NEW.return_x,NEW.return_y,NEW.return_floor) =
            CASE WHEN NEW.native_use_exit
                THEN (q.use_exit_cell[1],q.use_exit_cell[2],q.use_exit_cell[3]::smallint)
                ELSE (q.exit_x,q.exit_y,q.exit_floor)
            END)
    $guard$;
BEGIN
    SELECT pg_get_functiondef('game_activity_runtime_return_proven()'::regprocedure)
        INTO STRICT body;
    IF strpos(body, old_guard) = 0 THEN
        RAISE EXCEPTION 'activity return placement guard was not found';
    END IF;
    EXECUTE replace(body, old_guard, new_guard);
END;
$migration$;
