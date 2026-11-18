-- Silencer chamber uses burn one carried item and optionally start a raid.
DO $$
DECLARE
 name TEXT;
 rule TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY[
   'game_item_use_records_cause_kind_check','game_item_use_cause_shape'
 ] LOOP
   SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT rule
     FROM pg_constraint c
    WHERE c.conrelid='game_item_use_records'::regclass AND c.conname=name;
   EXECUTE format(
     'ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind=2961 AND shape=1))',
     name,name,rule);
 END LOOP;
END $$;

ALTER TABLE game_world_raid_schedule_sources
 DROP CONSTRAINT game_world_raid_schedule_sources_source_kind_check,
 ADD CHECK (source_kind IN (1,2,3) OR (source_kind=4 AND raid_key IN (
   'raid:roshamuul.silencer-plateau',
   'raid:soul-war.claustrophobic-inferno-1',
   'raid:soul-war.claustrophobic-inferno-2',
   'raid:soul-war.claustrophobic-inferno-3'
 )));

CREATE TABLE game_silencer_chamber_uses (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id),
 firing_id UUID UNIQUE REFERENCES game_world_raid_firings(firing_id),
 CHECK (firing_id IS NULL OR firing_id=transaction_id)
);
CREATE TRIGGER game_silencer_chamber_uses_immutable
 BEFORE UPDATE OR DELETE ON game_silencer_chamber_uses
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_silencer_chamber_uses_no_truncate
 BEFORE TRUNCATE ON game_silencer_chamber_uses
 EXECUTE FUNCTION game_item_immutable();

REVOKE ALL ON game_silencer_chamber_uses FROM PUBLIC;
GRANT SELECT, INSERT ON game_silencer_chamber_uses TO oteryn_game_runtime;
