-- Found and EditRanks supply the rank key and name. Only PostgreSQL assigns
-- the identity that distinguishes a removed rank from its replacement.
REVOKE ALL ON game_guild_ranks FROM PUBLIC;
REVOKE INSERT, UPDATE ON game_guild_ranks FROM oteryn_game_runtime, oteryn_game_control;
GRANT SELECT, DELETE ON game_guild_ranks TO oteryn_game_runtime;
GRANT INSERT (guild_id, level, name), UPDATE (name) ON game_guild_ranks TO oteryn_game_runtime;
GRANT SELECT ON game_guild_ranks TO oteryn_game_control;

-- Keep retained rows readable. PostgreSQL checks the wire bounds on future
-- inserts and updates without scanning historical rows during deployment.
DO $$ BEGIN
 IF NOT EXISTS (
  SELECT 1 FROM pg_catalog.pg_constraint
  WHERE conrelid='game_house_acl_guild_ranks'::regclass
   AND conname='game_house_acl_guild_ranks_door_wire_range'
 ) THEN
  ALTER TABLE game_house_acl_guild_ranks
   ADD CONSTRAINT game_house_acl_guild_ranks_door_wire_range CHECK (
    CASE
     WHEN list_key IN ('guest','subowner') THEN true
     WHEN list_key ~ '^door:[0-9]{1,5}:[0-9]{1,5}:-?[0-9]{1,2}$' THEN
      split_part(list_key,':',2)::integer BETWEEN 0 AND 65535
      AND split_part(list_key,':',3)::integer BETWEEN 0 AND 65535
      AND split_part(list_key,':',4)::integer BETWEEN -15 AND 0
     ELSE false
    END
   ) NOT VALID;
 END IF;
END $$;
