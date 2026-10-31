-- HOUSE-ITEM-WIRE-1. The admitted single-cell pair is Canary items.xml:17549-17552
-- (oven 6355 wrapableto 6371). Other pairs are UNKNOWN for this bounded slice.
-- Immutable same-CommandRef receipts preserve lineage and prove all changed rows.
CREATE TABLE game_house_furniture_wrap_receipts (
 transaction_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7() CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 intent BYTEA NOT NULL CHECK(octet_length(intent)=45),
 action SMALLINT NOT NULL CHECK(action IN(1,2)),
 before_item JSONB NOT NULL,
 after_key TEXT NOT NULL,
 house_location JSONB NOT NULL,
 inventory_location JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_house_furniture_wrap_character ON game_house_furniture_wrap_receipts(character_id);
CREATE INDEX game_house_furniture_wrap_item ON game_house_furniture_wrap_receipts(item_instance_id);

CREATE FUNCTION game_house_furniture_wrap_complete(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_wrap_receipts%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_tx;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR i.last_transaction_id IS DISTINCT FROM p_tx
  OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
  OR i.definition_production_key IS DISTINCT FROM r.after_key
  OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
   IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])
  OR (r.action=1 AND ((r.before_item->>'definition_production_key')<>'oteryn:item.tibia.i6355' OR r.after_key<>'oteryn:item.tibia.i6371'))
  OR (r.action=2 AND ((r.before_item->>'definition_production_key')<>'oteryn:item.tibia.i6371' OR r.after_key<>'oteryn:item.tibia.i6355'))
 THEN RAISE EXCEPTION 'incomplete house wrap item' USING ERRCODE='23514'; END IF;
 IF r.action=1 THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=r.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE to_jsonb(e)=r.inventory_location)
   THEN RAISE EXCEPTION 'incomplete house wrap custody' USING ERRCODE='23514'; END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE to_jsonb(h)=r.house_location)
   OR NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=r.item_instance_id
     AND p.reclaim_subject_character_id=r.character_id AND p.placement_transaction_id=r.transaction_id
     AND p.provenance_revision=1 AND p.written_xact_id=pg_current_xact_id())
   THEN RAISE EXCEPTION 'incomplete house unwrap custody' USING ERRCODE='23514'; END IF;
 END IF;
END $$;
CREATE FUNCTION game_house_furniture_wrap_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_house_furniture_wrap_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_house_furniture_wrap_proven AFTER INSERT ON game_house_furniture_wrap_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_furniture_wrap_proven();
CREATE TRIGGER game_house_furniture_wrap_immutable BEFORE UPDATE OR DELETE ON game_house_furniture_wrap_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_house_furniture_wrap_no_truncate BEFORE TRUNCATE ON game_house_furniture_wrap_receipts
 EXECUTE FUNCTION game_item_reject_truncate();

-- This owner-only writer is the sole INSERT authority for the evidence consumed below.
-- The current-session lock remains held until commit, including while replaying a receipt.
CREATE FUNCTION game_house_furniture_wrap(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,
 p_command NUMERIC,p_item UUID,p_revision NUMERIC,p_acl NUMERIC,p_action SMALLINT,p_position BYTEA,
 p_intent BYTEA,p_after TEXT,p_capacity INTEGER,p_backpack TEXT) RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; r game_house_furniture_wrap_receipts%ROWTYPE;
 h game_item_house_interior_locations%ROWTYPE; e game_item_container_entries%ROWTYPE;
 a RECORD; guild UUID; parent UUID; ordinal NUMERIC; tx UUID;
BEGIN
 IF p_action IS NULL OR p_action NOT IN(1,2) OR p_revision IS NULL OR p_revision NOT BETWEEN 1 AND 18446744073709551614
  OR p_revision<>trunc(p_revision) OR p_acl IS NULL OR p_acl NOT BETWEEN 1 AND 18446744073709551615
  OR p_acl<>trunc(p_acl) OR p_command IS NULL OR p_command NOT BETWEEN 1 AND 18446744073709551615 OR p_command<>trunc(p_command)
  OR p_intent IS NULL OR octet_length(p_intent)<>45 OR p_position IS NULL OR octet_length(p_position)<>12
  OR p_after IS NULL THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 -- Carried-item writers lock Character before House and Item rows.
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 SELECT owner_guild_id INTO guild FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF guild IS NOT NULL THEN PERFORM 1 FROM game_guilds WHERE guild_id=guild AND world_id=p_world FOR SHARE; END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'WRAP_NO_ACCESS'; END IF;
 IF (SELECT owner_guild_id FROM game_house_properties WHERE world_id=p_world AND house_key=p_house) IS DISTINCT FROM guild
  THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
 -- Serialize every House placement writer, including missing-row occupancy checks.
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 SELECT q.*,count(*) OVER() AS grant_count INTO a FROM game_house_access(p_world,p_house,p_character) q;
 IF NOT FOUND OR a.grant_count<>1 OR a.content_fenced IS DISTINCT FROM false OR a.role IS NULL OR a.role NOT IN('OWNER','SUBOWNER')
  THEN RAISE EXCEPTION 'WRAP_NO_ACCESS'; END IF;
 IF a.acl_revision IS DISTINCT FROM p_acl THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
 SELECT * INTO r FROM game_house_furniture_wrap_receipts WHERE game_session_id=p_session AND command_id=p_command;
 IF FOUND THEN
  IF r.intent IS DISTINCT FROM p_intent OR r.character_id<>p_character OR r.world_id<>p_world OR r.house_key<>p_house
   OR r.item_instance_id<>p_item OR r.action<>p_action THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
  RETURN (r.before_item->>'state_revision')::numeric+1;
 END IF;
 SELECT b.item_instance_id INTO parent FROM game_item_container_slots b JOIN game_item_instances backpack USING(item_instance_id,world_id)
  WHERE b.character_id=p_character AND b.world_id=p_world AND backpack.lifecycle=1
   AND backpack.definition_family='Item' AND backpack.definition_revision_ref='definition-r1'
   AND backpack.definition_production_key=p_backpack;
 IF parent IS NULL THEN RAISE EXCEPTION 'WRAP_INVENTORY_FULL'; END IF;
 PERFORM 1 FROM game_item_instances WHERE item_instance_id IN(p_item,parent) ORDER BY item_instance_id FOR UPDATE;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=p_item AND world_id=p_world AND lifecycle=1;
 IF NOT FOUND OR i.state_revision<>p_revision THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
 IF i.quantity<>1 OR i.definition_family<>'Item' OR i.definition_revision_ref<>'definition-r1'
  OR (p_action=1 AND (i.definition_production_key<>'oteryn:item.tibia.i6355' OR p_after<>'oteryn:item.tibia.i6371'))
  OR (p_action=2 AND (i.definition_production_key<>'oteryn:item.tibia.i6371' OR p_after<>'oteryn:item.tibia.i6355'))
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=p_item)
  THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 tx:=game_character_uuid_v7();
 IF p_action=1 THEN
  SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=p_item AND world_id=p_world AND house_key=p_house FOR UPDATE;
  IF NOT FOUND OR game_house_access_tile(p_world,p_house,h.spatial_position) IS DISTINCT FROM true THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  IF p_capacity IS NULL OR p_capacity NOT BETWEEN 1 AND 20 OR
   (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=parent)>=p_capacity
   THEN RAISE EXCEPTION 'WRAP_INVENTORY_FULL'; END IF;
  SELECT coalesce(max(placement_ordinal),0)+1 INTO ordinal FROM game_item_container_entries WHERE parent_item_instance_id=parent;
  e.item_instance_id:=p_item;e.world_id:=p_world;e.character_id:=p_character;e.parent_item_instance_id:=parent;
  e.placement_ordinal:=ordinal;e.placed_transaction_id:=tx;
 ELSE
  SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id=p_item AND world_id=p_world
   AND character_id=p_character AND parent_item_instance_id=parent FOR UPDATE;
  IF NOT FOUND OR NOT EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts prior
    WHERE prior.item_instance_id=p_item AND prior.action=1 AND prior.after_key=i.definition_production_key
     AND prior.before_item->>'definition_production_key'=p_after) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  IF game_house_access_tile(p_world,p_house,p_position) IS DISTINCT FROM true
   OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=p_world AND house_key=p_house AND spatial_position=p_position)
   OR (SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=p_world AND house_key=p_house)>=2000
   THEN RAISE EXCEPTION 'WRAP_BLOCKED'; END IF;
  h.item_instance_id:=p_item;h.world_id:=p_world;h.house_key:=p_house;h.spatial_position:=p_position;
  h.stack_ordinal:=1;h.placed_transaction_id:=tx;
 END IF;
 INSERT INTO game_house_furniture_wrap_receipts(transaction_id,game_session_id,command_id,character_id,item_instance_id,
  world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location)
 VALUES(tx,p_session,p_command,p_character,p_item,p_world,p_house,p_intent,p_action,to_jsonb(i),p_after,to_jsonb(h),to_jsonb(e));
 IF p_action=1 THEN
  DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=p_item;
  DELETE FROM game_item_house_interior_locations WHERE item_instance_id=p_item;
  INSERT INTO game_item_container_entries SELECT(e).*;
 ELSE
  DELETE FROM game_item_container_entries WHERE item_instance_id=p_item;
  INSERT INTO game_item_house_interior_locations SELECT(h).*;
  INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
   VALUES(p_item,p_world,p_house,p_character,tx,1);
 END IF;
 UPDATE game_item_instances SET definition_production_key=p_after,last_transaction_id=tx,state_revision=state_revision+1
  WHERE item_instance_id=p_item;
 RETURN p_revision+1;
END $$;

-- Add exact wrap evidence arms without replacing any previous mechanic's guard chain.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_container_entry_removal_proven','game_item_placement_proven','game_house_location_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=OLD.item_instance_id
     AND r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<>OLD.quantity
      OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
       (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
      THEN RAISE EXCEPTION 'wrap preserves item lineage' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=NEW.item_instance_id
     AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_house_furniture_wrap_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.action=2
     AND r.inventory_location=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.action=1
      AND r.inventory_location=to_jsonb(NEW) AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='INSERT' THEN
     IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.action=2
      AND r.house_location=to_jsonb(NEW) AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    ELSIF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.action=1
      AND r.house_location=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_house_furniture_wrap_receipts FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_wrap_receipts TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_complete(UUID),game_house_furniture_wrap_proven(),
 game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)
 TO oteryn_game_runtime;

-- The existing deferred Item-change trigger is invoker-security and calls this read-only proof.
GRANT EXECUTE ON FUNCTION game_house_furniture_wrap_complete(UUID) TO oteryn_game_runtime;
