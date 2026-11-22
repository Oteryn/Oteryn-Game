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
