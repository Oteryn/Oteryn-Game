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
