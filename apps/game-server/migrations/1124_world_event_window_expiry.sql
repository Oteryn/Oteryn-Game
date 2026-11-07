-- Expiry scans only unfinished windows and retires at most eight per owner pass.
CREATE INDEX game_world_timed_events_unfinished_window
 ON game_world_timed_events(world_id,event_key,((state->>'end_ms')::BIGINT),cycle_id)
 WHERE state->>'phase' IN ('Active','Dormant');
