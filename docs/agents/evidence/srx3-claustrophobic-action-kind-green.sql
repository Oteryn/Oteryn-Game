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
ALTER TABLE srx3_source_raid_action_probe DROP CONSTRAINT game_world_raid_effects_action_kind_check;
ALTER TABLE srx3_source_raid_action_probe ADD COLUMN source_cue JSONB;
ALTER TABLE srx3_source_raid_action_probe ADD CHECK(action_kind IN(1,2,3,4,5));
ALTER TABLE srx3_source_raid_action_probe ADD CHECK((action_kind=5)=(source_cue IS NOT NULL) AND (source_cue IS NULL OR (jsonb_typeof(source_cue)='object' AND octet_length(source_cue::text)<=2048)));
INSERT INTO srx3_source_raid_action_probe(firing_id,channel_id,step_ordinal,owner_generation,action_kind,source_cue)
VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',1,1,5,'{"raid":"First","id":1,"due":1000,"effect":"Advance"}');
DO $$ BEGIN
 BEGIN INSERT INTO srx3_source_raid_action_probe(firing_id,channel_id,step_ordinal,owner_generation,action_kind) VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',2,1,5); RAISE EXCEPTION 'missing cue accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN INSERT INTO srx3_source_raid_action_probe(firing_id,channel_id,step_ordinal,owner_generation,action_kind,source_cue) VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',3,1,6,'{}'); RAISE EXCEPTION 'unknown kind accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
ROLLBACK;
