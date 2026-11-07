\connect crs_srx3_cruelty_native_1794008352899988192
\set ON_ERROR_STOP on
\set VERBOSITY verbose
\timing on
BEGIN;
CREATE TEMP TABLE srx3_source_raid_kind_probe
(LIKE public.game_world_raid_schedule_sources INCLUDING DEFAULTS INCLUDING CONSTRAINTS)
ON COMMIT DROP;
INSERT INTO srx3_source_raid_kind_probe(world_id,raid_key,source_kind)
VALUES('00000000-0000-7000-8000-000000000001','raid:legacy-positive-control',2);
-- Source4 is a fixed STEP encounter, not adaptive Lua scheduling. It is
-- accepted only for the three closed source keys by the proposed forward fix.
INSERT INTO srx3_source_raid_kind_probe(world_id,raid_key,source_kind)
VALUES('00000000-0000-7000-8000-000000000001','raid:soul-war.claustrophobic-inferno-1',4);
ROLLBACK;
