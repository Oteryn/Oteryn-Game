\connect crs_srx3_cruelty_native_1794008352899988192
\set ON_ERROR_STOP on
\set VERBOSITY verbose
\timing on
BEGIN;
CREATE TEMP TABLE srx3_source_raid_kind_probe
(LIKE public.game_world_raid_schedule_sources INCLUDING DEFAULTS INCLUDING CONSTRAINTS)
ON COMMIT DROP;
-- SRX-3 W11: fixed STEP-triggered Soul War Encounters are not XML chance,
-- adaptive Lua chance or calendar callbacks. The existing Raid owner keeps
-- their firing/run/placement receipts; only these literal source keys use4.
ALTER TABLE srx3_source_raid_kind_probe
    DROP CONSTRAINT game_world_raid_schedule_sources_source_kind_check;
ALTER TABLE srx3_source_raid_kind_probe
    ADD CONSTRAINT game_world_raid_schedule_sources_source_kind_check CHECK (
        source_kind IN (1, 2, 3)
        OR (source_kind = 4 AND raid_key IN (
            'raid:soul-war.claustrophobic-inferno-1',
            'raid:soul-war.claustrophobic-inferno-2',
            'raid:soul-war.claustrophobic-inferno-3'
        ))
    );

INSERT INTO srx3_source_raid_kind_probe(world_id,raid_key,source_kind)
VALUES('00000000-0000-7000-8000-000000000001','raid:legacy-positive-control',2),
('00000000-0000-7000-8000-000000000001','raid:soul-war.claustrophobic-inferno-1',4),
('00000000-0000-7000-8000-000000000001','raid:soul-war.claustrophobic-inferno-2',4),
('00000000-0000-7000-8000-000000000001','raid:soul-war.claustrophobic-inferno-3',4);
DO $$
BEGIN
  BEGIN
    INSERT INTO srx3_source_raid_kind_probe VALUES('00000000-0000-7000-8000-000000000001','raid:unreviewed',4);
    RAISE EXCEPTION 'unreviewed source4 was accepted';
  EXCEPTION WHEN check_violation THEN RAISE NOTICE 'SRX3_UNREVIEWED_SOURCE4_REFUSED SQLSTATE=%',SQLSTATE;
  END;
  BEGIN
    INSERT INTO srx3_source_raid_kind_probe VALUES('00000000-0000-7000-8000-000000000001','raid:soul-war.claustrophobic-inferno-1',5);
    RAISE EXCEPTION 'unknown source5 was accepted';
  EXCEPTION WHEN check_violation THEN RAISE NOTICE 'SRX3_UNKNOWN_SOURCE5_REFUSED SQLSTATE=%',SQLSTATE;
  END;
  IF (SELECT count(*) FROM srx3_source_raid_kind_probe) <> 4 THEN
    RAISE EXCEPTION 'exact positive source count mismatch';
  END IF;
END $$;
ROLLBACK;
