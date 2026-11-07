-- WEVENTS first native bindings. May 1 is included by the retained Chyllfroest gate.
ALTER TABLE game_world_timed_events DROP CONSTRAINT game_world_timed_events_event_key_check;
ALTER TABLE game_world_timed_events ADD CONSTRAINT game_world_timed_events_event_key_check
 CHECK(event_key IN ('flower_month','mummys_curse','chyllfroest'));
-- Failed canopic-jar attempts are per jar, not a cooldown shared across all jars.
ALTER TABLE game_world_timed_participants ADD COLUMN ready_object INTEGER NOT NULL DEFAULT 0
 CHECK (ready_object BETWEEN 0 AND 65535);
-- Correct foundation snapshots to the existing Berlin save calendar (23/25h DST
-- days are not flattened to 24h). Immutable reset provenance supplies the boundary.
UPDATE game_world_timed_events t SET state=jsonb_set(t.state,'{end_ms}',to_jsonb(
 (extract(epoch FROM ((date_trunc('day',r.activated_at AT TIME ZONE 'Europe/Berlin'-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin'))*1000)::bigint))
 FROM game_world_reset_epochs r
 WHERE t.event_key='mummys_curse' AND t.world_id=r.world_id AND t.cycle_id=r.reset_epoch;
UPDATE game_world_timed_participants p SET ready_object=50006+(t.state->>'objects')::integer
 FROM game_world_timed_events t
 WHERE (p.world_id,p.event_key,p.cycle_id)=(t.world_id,t.event_key,t.cycle_id)
 AND p.event_key='mummys_curse' AND p.ready_ms>0;


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
