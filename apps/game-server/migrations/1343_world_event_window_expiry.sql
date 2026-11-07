-- Expiry scans only unfinished windows and retires at most eight per owner pass.
CREATE INDEX game_world_timed_events_unfinished_window
 ON game_world_timed_events(world_id,event_key,((state->>'end_ms')::BIGINT),cycle_id)
 WHERE state->>'phase' IN ('Active','Dormant');


-- Completed firing history is immutable. Retirement stops retained effects
-- through a separate immutable receipt instead of rewriting that history.
CREATE TABLE game_world_raid_retirements (
    firing_id UUID PRIMARY KEY REFERENCES game_world_raid_firings(firing_id),
    recorded_at_ms BIGINT NOT NULL CHECK(recorded_at_ms>=0),
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 96)
);
CREATE TRIGGER game_world_raid_retirements_immutable BEFORE UPDATE OR DELETE
 ON game_world_raid_retirements FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_world_raid_retirements FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_raid_retirements TO oteryn_game_runtime;
GRANT SELECT ON game_world_raid_retirements TO oteryn_game_control;
