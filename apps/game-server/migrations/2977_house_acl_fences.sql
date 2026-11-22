-- Historical receipts remain replayable. On both the deployed and new writer
-- paths, only the database may stamp the admitted connection into a new receipt.
ALTER TABLE game_house_management_receipts ADD COLUMN connection_generation NUMERIC(20,0);
ALTER TABLE game_house_management_receipts ADD CONSTRAINT game_house_management_receipt_generation
 CHECK(connection_generation IS NOT NULL AND connection_generation BETWEEN 1 AND 18446744073709551615) NOT VALID;

REVOKE INSERT ON game_house_management_receipts FROM oteryn_game_runtime;
GRANT INSERT(game_session_id,command_id,character_id,world_id,house_key,request_binding,
 action_kind,list_key,result_revision) ON game_house_management_receipts TO oteryn_game_runtime;

CREATE FUNCTION game_house_management_stamp_receipt() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 SELECT sess.current_generation INTO NEW.connection_generation
 FROM game_durability_reconnect_sessions sess WHERE sess.game_session_id=NEW.game_session_id;
 IF NOT FOUND OR NOT game_house_acl_session_current(
  NEW.game_session_id,NEW.character_id,NEW.world_id,NEW.connection_generation
 ) THEN
  RAISE EXCEPTION 'House management receipt needs an admitted session' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_management_stamp_receipt BEFORE INSERT ON game_house_management_receipts
 FOR EACH ROW EXECUTE FUNCTION game_house_management_stamp_receipt();

CREATE FUNCTION game_house_management_guild_acl_advance(s UUID,c NUMERIC,rules JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_management_receipts%ROWTYPE; p game_house_properties%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_management_receipts WHERE game_session_id=s AND command_id=c;
 IF NOT FOUND OR r.connection_generation IS NULL
  OR NOT game_house_acl_session_current(s,r.character_id,r.world_id,r.connection_generation) THEN
  RAISE EXCEPTION 'House guild ACL needs its original current session' USING ERRCODE='23514';
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=r.world_id AND house_key=r.house_key FOR UPDATE;
 PERFORM game_house_management_acl_advance(s,c);
 INSERT INTO game_house_acl_guild_ranks(world_id,house_key,list_key,ownership_generation,
  acl_revision,previous_acl_revision,connection_generation,rules,last_session_id,last_command_id)
 VALUES(r.world_id,r.house_key,r.list_key,p.ownership_generation,r.result_revision,p.acl_revision,r.connection_generation,rules,s,c)
 ON CONFLICT(world_id,house_key,list_key) DO UPDATE SET
  ownership_generation=excluded.ownership_generation,acl_revision=excluded.acl_revision,
  previous_acl_revision=excluded.previous_acl_revision,connection_generation=excluded.connection_generation,
  rules=excluded.rules,last_session_id=excluded.last_session_id,last_command_id=excluded.last_command_id;
END $$;

-- The pre-deploy node still calls the original signature. Its generation is a
-- comparison only. The same database-stamped receipt governs either signature.
CREATE OR REPLACE FUNCTION game_house_management_guild_acl_advance(s UUID,c NUMERIC,generation NUMERIC,rules JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_management_receipts r
  WHERE r.game_session_id=s AND r.command_id=c AND r.connection_generation=generation) THEN
  RAISE EXCEPTION 'House guild ACL needs its original current session' USING ERRCODE='23514';
 END IF;
 PERFORM game_house_management_guild_acl_advance(s,c,rules);
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_stamp_receipt() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_management_stamp_receipt(),
 game_house_management_guild_acl_advance(UUID,NUMERIC,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,JSONB) TO oteryn_game_runtime;

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

-- An opaque address prevents proving that a specific creature's cell has no
-- field. Index just these exceptional placements so the probe stays local.
CREATE INDEX game_item_ground_opaque_scope_lookup ON game_item_ground_locations
 (world_id,scope_kind,(COALESCE(channel_id,instance_id))) WHERE octet_length(spatial_position)<>10;
