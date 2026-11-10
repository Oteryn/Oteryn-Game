\connect crs_srx3_claustrophobic_world_1794017011580834183
\set ON_ERROR_STOP on
\set VERBOSITY verbose
\timing on
BEGIN;
CREATE TEMP TABLE srx3_source_raid_action_probe
(LIKE public.game_world_raid_effects INCLUDING DEFAULTS INCLUDING CONSTRAINTS)
ON COMMIT DROP;
INSERT INTO srx3_source_raid_action_probe(firing_id,channel_id,step_ordinal,owner_generation,action_kind,cleanup)
VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',0,1,3,false);
-- A fixed source cue is neither a textual announcement nor a fabricated wave.
INSERT INTO srx3_source_raid_action_probe(firing_id,channel_id,step_ordinal,owner_generation,action_kind)
VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',1,1,5);
ROLLBACK;
