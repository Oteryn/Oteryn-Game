-- Admit vacant personal properties from the active World's checked map catalogue.
-- Opening auctions and updating established properties are separate transitions.
CREATE FUNCTION game_world_house_properties_admit(w UUID,ch UUID,g NUMERIC,epoch NUMERIC,map TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE digest BYTEA;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra
   WHERE ra.scope_kind=1 AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
    AND ra.ownership_generation=g AND ra.holder_node_id=n AND ra.holder_registration_revision=nr FOR SHARE)
 THEN RETURN false; END IF;
 SELECT e.server_digest INTO digest FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 WHERE a.world_id=w AND a.reset_epoch=epoch AND e.map_revision=map FOR SHARE OF a,e;
 IF NOT FOUND OR p IS NULL OR jsonb_typeof(p)<>'array' OR jsonb_array_length(p)>1000
  OR octet_length(p::text)>1048576 THEN RETURN false; END IF;

 WITH admitted AS (
  INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
  SELECT w,x->>'house_key',x->>'kind',(x->>'rent_gold')::bigint,digest
  FROM jsonb_array_elements(p) x WHERE x->>'kind' IN('private_house','shop')
  ORDER BY x->>'house_key'
  ON CONFLICT(world_id,house_key) DO NOTHING
  RETURNING world_id,house_key
 )
 INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
 SELECT world_id,house_key,1,game_character_uuid_v7() FROM admitted;
 RETURN true;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_house_properties_admit(UUID,UUID,NUMERIC,NUMERIC,TEXT,JSONB,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_world_house_properties_admit(UUID,UUID,NUMERIC,NUMERIC,TEXT,JSONB,UUID,NUMERIC,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_house_properties_admit(UUID,UUID,NUMERIC,NUMERIC,TEXT,JSONB,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
