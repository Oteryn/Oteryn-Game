-- Preserve the ledger checksum of 2975 and tolerate reapplication of this migration.
ALTER TABLE game_guild_ranks
 ADD COLUMN IF NOT EXISTS house_acl_identity BIGINT GENERATED ALWAYS AS IDENTITY;

ALTER TABLE game_house_acl_guild_ranks
 DROP CONSTRAINT IF EXISTS game_house_acl_guild_ranks_list_key_check,
 ADD CONSTRAINT game_house_acl_guild_ranks_list_key_check CHECK(
  CASE
   WHEN list_key IN ('guest', 'subowner') THEN true
   WHEN list_key ~ '^door:[0-9]{1,5}:[0-9]{1,5}:-?[0-9]{1,2}$' THEN
    split_part(list_key, ':', 2)::integer BETWEEN 0 AND 65535
    AND split_part(list_key, ':', 3)::integer BETWEEN 0 AND 65535
    AND split_part(list_key, ':', 4)::integer BETWEEN -15 AND 0
   ELSE false
  END);

-- The rank mutation trigger still verifies the matching same-transaction Guild operation.
-- Only these columns enter the insert, so PostgreSQL always supplies the ACL identity.
CREATE OR REPLACE FUNCTION game_guild_write_rank(
 p_guild UUID,p_level SMALLINT,p_name TEXT,p_rename_existing BOOLEAN
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF p_guild IS NULL OR p_level IS NULL OR p_level NOT BETWEEN 1 AND 20
    OR p_name IS NULL OR p_name='' OR p_rename_existing IS NULL THEN
  RAISE EXCEPTION 'Guild rank requires a guild, level and name' USING ERRCODE='23514';
 END IF;
 IF p_rename_existing THEN
  INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(p_guild,p_level,p_name)
  ON CONFLICT(guild_id,level) DO UPDATE SET name=EXCLUDED.name;
 ELSE
  INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(p_guild,p_level,p_name);
 END IF;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_guild_write_rank(UUID,SMALLINT,TEXT,BOOLEAN) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_guild_write_rank(UUID,SMALLINT,TEXT,BOOLEAN)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_guild_write_rank(UUID,SMALLINT,TEXT,BOOLEAN) TO oteryn_game_runtime;
REVOKE INSERT ON game_guild_ranks FROM oteryn_game_runtime;
