-- 0562 moved the World owner to Berlin 06:00 but left the history check at 10:00.
-- Keep the civil-day proof aligned with the owner, including both DST offsets.
ALTER TABLE game_world_reset_history
    DROP CONSTRAINT game_world_reset_history_check1,
    ADD CONSTRAINT game_world_reset_history_check1
    CHECK (reset_day = ((to_timestamp(reset_at_unix_ms / 1000.0)
        AT TIME ZONE 'Europe/Berlin') - INTERVAL '6 hours')::date - DATE '1970-01-01');
