-- Historical receipts remain replayable. Every new receipt binds the original
-- admitted transport generation, and the existing immutable trigger protects it.
ALTER TABLE game_house_management_receipts ADD COLUMN connection_generation NUMERIC(20,0);
ALTER TABLE game_house_management_receipts ADD CONSTRAINT game_house_management_receipt_generation
 CHECK(connection_generation IS NOT NULL AND connection_generation BETWEEN 1 AND 18446744073709551615) NOT VALID;

CREATE OR REPLACE FUNCTION game_house_management_guild_acl_advance(s UUID,c NUMERIC,generation NUMERIC,rules JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_management_receipts%ROWTYPE; p game_house_properties%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_management_receipts WHERE game_session_id=s AND command_id=c;
 IF NOT FOUND OR r.connection_generation IS DISTINCT FROM generation
  OR NOT game_house_acl_session_current(s,r.character_id,r.world_id,r.connection_generation) THEN
  RAISE EXCEPTION 'House guild ACL needs its original current session' USING ERRCODE='23514';
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=r.world_id AND house_key=r.house_key FOR UPDATE;
 PERFORM game_house_management_acl_advance(s,c);
 INSERT INTO game_house_acl_guild_ranks(world_id,house_key,list_key,ownership_generation,
  acl_revision,previous_acl_revision,connection_generation,rules,last_session_id,last_command_id)
 VALUES(r.world_id,r.house_key,r.list_key,p.ownership_generation,r.result_revision,p.acl_revision,generation,rules,s,c)
 ON CONFLICT(world_id,house_key,list_key) DO UPDATE SET
  ownership_generation=excluded.ownership_generation,acl_revision=excluded.acl_revision,
  previous_acl_revision=excluded.previous_acl_revision,connection_generation=excluded.connection_generation,
  rules=excluded.rules,last_session_id=excluded.last_session_id,last_command_id=excluded.last_command_id;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- Preserve the named-House role separately from exact door lists. A door
-- selector does not grant entry to the House or access to its services.
CREATE FUNCTION game_house_door_access(w UUID,h TEXT,actor UUID)
RETURNS TABLE(all_doors BOOLEAN,lists TEXT[]) LANGUAGE sql SECURITY DEFINER AS $$
 SELECT a.role IN('OWNER','SUBOWNER'),ARRAY(
  SELECT list_key FROM game_house_acl_member_lists(w,h,actor)
  WHERE list_key LIKE 'door:%' ORDER BY list_key
 )
 FROM game_house_access(w,h,actor) a WHERE NOT a.content_fenced
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_door_access(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_door_access(UUID,TEXT,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_door_access(UUID,TEXT,UUID) TO oteryn_game_runtime;
