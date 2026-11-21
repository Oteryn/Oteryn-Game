-- Source sand-timer raids are valid before the first World reset (0148).
-- Keep the legacy scheduler's positive-epoch constraint for every other key.
ALTER TABLE game_world_raid_schedules
 DROP CONSTRAINT game_world_raid_schedules_reset_epoch_check,
 ADD CONSTRAINT game_world_raid_schedules_reset_epoch_check CHECK (
  reset_epoch BETWEEN 1 AND 18446744073709551615 OR
  (reset_epoch=0 AND raid_key IN (
   'raid:soul-war.claustrophobic-inferno-1',
   'raid:soul-war.claustrophobic-inferno-2',
   'raid:soul-war.claustrophobic-inferno-3')));

ALTER TABLE game_world_raid_firings
 DROP CONSTRAINT game_world_raid_firings_reset_epoch_check,
 ADD CONSTRAINT game_world_raid_firings_reset_epoch_check CHECK (
  reset_epoch BETWEEN 1 AND 18446744073709551615 OR
  (reset_epoch=0 AND raid_key IN (
   'raid:soul-war.claustrophobic-inferno-1',
   'raid:soul-war.claustrophobic-inferno-2',
   'raid:soul-war.claustrophobic-inferno-3')));
