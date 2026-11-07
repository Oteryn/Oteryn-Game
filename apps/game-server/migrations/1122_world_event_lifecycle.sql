-- WEVENTS reusable lifecycle and native effect provenance. Existing receipts stay immutable.
ALTER TABLE game_world_timed_events DROP CONSTRAINT game_world_timed_events_event_key_check;
ALTER TABLE game_world_timed_events ADD CONSTRAINT game_world_timed_events_event_key_check
 CHECK(event_key IN ('flower_month','mummys_curse','chyllfroest','spring_into_life',
 'rise_of_devovorga','devovorga_s_essence','kingsday','bank_robbery','insectoid_invasion'));
ALTER TABLE game_world_timed_events DROP CONSTRAINT game_world_timed_events_state_check;
ALTER TABLE game_world_timed_events ADD CONSTRAINT game_world_timed_events_state_check
 CHECK(jsonb_typeof(state)='object' AND octet_length(state::text)<=32768);
ALTER TABLE game_world_timed_participants ADD COLUMN reward_ready BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE game_world_timed_receipts ADD COLUMN origin JSONB NULL
 CHECK(origin IS NULL OR (jsonb_typeof(origin)='object' AND octet_length(origin::text)<=1024));
CREATE TABLE game_world_timed_raid_origins (
 firing_id UUID PRIMARY KEY REFERENCES game_world_raid_firings(firing_id),
 origin JSONB NOT NULL CHECK(jsonb_typeof(origin)='object' AND octet_length(origin::text)<=1024)
);
CREATE TRIGGER game_world_timed_raid_origins_immutable BEFORE UPDATE OR DELETE
 ON game_world_timed_raid_origins FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_world_timed_raid_origins FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_timed_raid_origins TO oteryn_game_runtime;
GRANT SELECT ON game_world_timed_raid_origins TO oteryn_game_control;
