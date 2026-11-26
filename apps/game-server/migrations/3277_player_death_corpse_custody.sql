-- A new death drop names its corpse placement and any replacement bag. The
-- nullable fields preserve the receipts and proofs of historical loose drops.
ALTER TABLE game_player_death_item_records
 ADD COLUMN destination_corpse_id UUID REFERENCES game_item_instances(item_instance_id),
 ADD COLUMN destination_ordinal NUMERIC(20,0),
 ADD COLUMN replacement_bag_id UUID UNIQUE,
 ADD CONSTRAINT game_player_death_corpse_destination CHECK (
  (destination_corpse_id IS NULL AND destination_ordinal IS NULL AND replacement_bag_id IS NULL)
  OR (operation=1 AND destination_corpse_id IS NOT NULL AND destination_corpse_id<>item_instance_id
      AND destination_ordinal IS NOT NULL AND destination_ordinal>0
      AND (replacement_bag_id IS NOT NULL)=(source_slot=9)));
CREATE INDEX game_player_death_items_corpse_idx ON game_player_death_item_records(destination_corpse_id)
 WHERE destination_corpse_id IS NOT NULL;
CREATE UNIQUE INDEX game_player_death_corpse_once ON game_item_mint_receipts(loot_table_production_key)
 WHERE loot_table_family='PlayerDeath' AND loot_purpose_key='CORPSE_MATERIALIZATION';

CREATE FUNCTION game_player_death_corpse_custody(p_tx UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_player_death_item_records r
  JOIN game_character_death_receipts d USING(death_occurrence_id)
  JOIN game_item_corpse_container_entries e ON e.item_instance_id=r.item_instance_id
  JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
  WHERE r.transaction_id=p_tx AND r.operation=1 AND r.created_xact_id=pg_current_xact_id()
   AND (e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id,e.world_id)
       =(r.destination_corpse_id,r.destination_ordinal,p_tx,r.world_id)
   AND i.world_id=r.world_id AND i.lifecycle=1 AND i.quantity=1
   AND m.loot_table_family='PlayerDeath' AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
   AND m.loot_table_production_key='oteryn:death.'||encode(uuid_send(d.death_occurrence_id),'hex')
   AND m.transaction_id=i.minted_transaction_id AND m.draw_ordinal=0
   AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
       IS NOT DISTINCT FROM (r.world_id,r.scope_kind,r.channel_id,r.instance_id)
   AND g.runtime_scope_ownership_generation<=r.scope_generation
   AND (g.spatial_position,g.map_revision,g.content_revision)
       =(d.death_spatial_position,d.death_map_revision,d.content_revision)
   AND g.corpse_ref=uuid_send(d.death_occurrence_id)
   AND g.native_room_placement_context=g.corpse_ref
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations loose WHERE loose.item_instance_id=r.item_instance_id)
 )
$$;

CREATE FUNCTION game_player_death_corpse_mint_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_item_mint_receipts m
  JOIN game_item_mint_reservations v USING(transaction_id)
  JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  JOIN game_item_audit_outbox a ON a.event_id=m.event_id
  JOIN game_player_death_item_records r ON r.destination_corpse_id=m.item_instance_id
  JOIN game_character_death_receipts d USING(death_occurrence_id)
  WHERE m.item_instance_id=p_item AND m.loot_table_family='PlayerDeath'
   AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
   AND m.death_actor_local_id=0 AND m.death_actor_local_generation=1
   AND m.corpse_top_damage_character_id IS NULL AND m.destination_parent_item_instance_id IS NULL
   AND m.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
   AND (m.death_world_id,m.death_scope_kind,m.death_channel_id,m.death_instance_id,m.death_scope_ownership_generation)
       IS NOT DISTINCT FROM (r.world_id,r.scope_kind,r.channel_id,r.instance_id,r.scope_generation)
   AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id,g.runtime_scope_ownership_generation)
       IS NOT DISTINCT FROM (m.death_world_id,m.death_scope_kind,m.death_channel_id,m.death_instance_id,m.death_scope_ownership_generation)
   AND d.character_id=r.character_id AND r.occurred_at=m.occurred_at
   AND i.state_revision=1 AND i.last_transaction_id IS NULL
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =('Item','oteryn:item.tibia.i4240','definition-r1')
   AND (v.item_instance_id,v.event_id,v.intent_binding,v.occurred_at,v.death_scope_key,
        v.death_scope_ownership_generation,v.death_actor_local_id,v.death_actor_local_generation,
        v.loot_table_family,v.loot_table_production_key,v.loot_table_revision_ref,v.loot_purpose_key,v.draw_ordinal)
       =(m.item_instance_id,m.event_id,m.intent_binding,m.occurred_at,m.death_scope_key,
         m.death_scope_ownership_generation,m.death_actor_local_id,m.death_actor_local_generation,
         m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref,m.loot_purpose_key,m.draw_ordinal)
   AND a.created_xact_id=pg_current_xact_id() AND a.item_instance_id=i.item_instance_id
   AND a.transaction_id=m.transaction_id AND a.occurred_at=m.occurred_at
   AND a.envelope=v.envelope AND a.envelope_sha256=m.envelope_sha256
   AND game_player_death_corpse_custody(r.transaction_id)
 )
$$;

CREATE FUNCTION game_player_death_bag_proven(p_tx UUID, p_bag UUID) RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
BEGIN
 IF EXISTS (
  SELECT 1 FROM game_player_death_item_records r
  JOIN game_item_instances i ON i.item_instance_id=r.replacement_bag_id
  JOIN game_item_container_slots s ON s.item_instance_id=i.item_instance_id
  JOIN game_player_death_item_removals rm USING(transaction_id)
  WHERE r.transaction_id=p_tx AND r.source_slot=9 AND r.operation=1
   AND r.replacement_bag_id=p_bag AND r.item_instance_id<>p_bag
   AND r.created_xact_id=pg_current_xact_id() AND rm.created_xact_id=pg_current_xact_id()
   AND i.minted_transaction_id=p_tx AND i.world_id=r.world_id
   AND i.state_revision=1 AND i.quantity=1 AND i.lifecycle=1 AND i.last_transaction_id IS NULL
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =('Item','oteryn:item.tibia.i2853','definition-r1')
   AND (s.character_id,s.world_id,s.placed_transaction_id)=(r.character_id,r.world_id,p_tx)
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=p_bag)
   AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=p_bag)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=p_bag)
 ) THEN
  PERFORM game_item_location_exclusive(p_bag);
  RETURN TRUE;
 END IF;
 RETURN FALSE;
END
$$;

DO $migration$
DECLARE body TEXT; amended TEXT; signature TEXT; anchor TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_player_death_corpse_custody(uuid)','game_player_death_corpse_mint_proven(uuid)',
  'game_player_death_bag_proven(uuid,uuid)'
 ] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
 -- pg_get_functiondef emits CREATE OR REPLACE and retains the installed signature,
 -- exact Channel/Instance scope guard, search path and privileges.
 body:=pg_get_functiondef('game_player_death_item_complete(uuid)'::regprocedure);
 anchor:='OR (r.operation=1 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations';
 amended:=replace(body,anchor,$arm$
  OR (r.destination_corpse_id IS NOT NULL AND (
    NOT game_player_death_corpse_custody(p_tx)
    OR (r.source_slot=9 AND NOT game_player_death_bag_proven(p_tx,r.replacement_bag_id))))
  OR (r.destination_corpse_id IS NULL AND r.operation=1 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations$arm$);
 IF body=amended THEN RAISE EXCEPTION 'player death completion Ground guard changed'; END IF;
 body:=amended;
 amended:=replace(body,'ON d.death_occurrence_id=r.death_occurrence_id WHERE g.item_instance_id=r.item_instance_id',
  'ON d.death_occurrence_id=r.death_occurrence_id WHERE d.item_selection_root IS NULL AND g.item_instance_id=r.item_instance_id');
 IF body=amended THEN RAISE EXCEPTION 'player death legacy receipt lookup changed'; END IF;
 EXECUTE amended;

 body:=pg_get_functiondef('game_item_corpse_container_entry_proven()'::regprocedure);
 anchor:='IF NOT EXISTS (';
 amended:=replace(body,anchor,'IF NOT game_player_death_corpse_custody(NEW.placed_transaction_id) AND NOT EXISTS (');
 IF body=amended THEN RAISE EXCEPTION 'corpse entry insertion guard changed'; END IF;
 EXECUTE amended;

 body:=pg_get_functiondef('game_source_corpse_mint_proven()'::regprocedure);
 anchor:='IF TG_TABLE_NAME=''game_item_mint_receipts'' THEN';
 amended:=replace(body,anchor,anchor||$arm$
  IF NEW.loot_table_family='PlayerDeath' THEN
   IF NOT game_player_death_corpse_mint_proven(NEW.item_instance_id) THEN
    RAISE EXCEPTION 'player corpse requires its current player death loss proof' USING ERRCODE='23514';
   END IF;
   RETURN NULL;
  END IF;
 $arm$);
 IF body=amended THEN RAISE EXCEPTION 'corpse mint source guard changed'; END IF;
 EXECUTE amended;

 FOREACH signature IN ARRAY ARRAY['game_item_ground_insertion_guard()', 'game_item_mint_consistency_guard()', 'game_item_placement_proven()'] LOOP
  body:=pg_get_functiondef(signature::regprocedure);
  anchor:=CASE signature
    WHEN 'game_item_ground_insertion_guard()' THEN $arm$
    IF game_player_death_corpse_mint_proven(NEW.item_instance_id) THEN RETURN NULL; END IF;
    $arm$
    WHEN 'game_item_mint_consistency_guard()' THEN $arm$
    IF game_player_death_bag_proven(NEW.minted_transaction_id,NEW.item_instance_id) THEN
      RETURN NULL;
    END IF;
    $arm$
    ELSE $arm$
    IF TG_TABLE_NAME='game_item_container_slots'
     AND game_player_death_bag_proven(NEW.placed_transaction_id,NEW.item_instance_id) THEN
      RETURN NULL;
    END IF;
    $arm$ END;
  IF position(E'\nBEGIN\n' IN body)=0 THEN RAISE EXCEPTION 'missing placement proof: %',signature; END IF;
  EXECUTE overlay(body placing E'\nBEGIN\n'||anchor FROM position(E'\nBEGIN\n' IN body) FOR 7);
 END LOOP;
END
$migration$;

-- A previously looted ItemInstance can become a death loss again. The proof
-- is unique in its physical transaction, the key all existing consumers use.
ALTER TABLE game_item_corpse_entry_removal_evidence
 DROP CONSTRAINT game_item_corpse_entry_removal_evidence_pkey,
 ADD PRIMARY KEY(item_instance_id,created_xact_id);

DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 -- Global manual §controls: the PvE damage-priority window excludes player bodies.
 body:=pg_get_functiondef('game_item_transfer_consistency_guard()'::regprocedure);
 amended:=replace(body,'IF from_corpse THEN' || E'\n        SELECT r.corpse_top_damage_character_id',
  $arm$IF from_corpse AND NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts player
        WHERE player.item_instance_id=corpse_ev.parent_item_instance_id
         AND player.loot_table_family='PlayerDeath' AND player.loot_purpose_key='CORPSE_MATERIALIZATION'
         AND player.materialized_at IS NOT NULL
    ) THEN
        SELECT r.corpse_top_damage_character_id$arm$);
 IF body=amended THEN RAISE EXCEPTION 'corpse transfer exclusivity guard changed'; END IF;
 EXECUTE amended;

 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 amended:=replace(body,'OR cr.corpse_top_damage_character_id=r.character_id',
  'OR cr.loot_table_family=''PlayerDeath'' OR cr.corpse_top_damage_character_id=r.character_id');
 IF body=amended THEN RAISE EXCEPTION 'quick loot exclusivity guard changed'; END IF;
 EXECUTE amended;
END
$migration$;

-- Keep the shared scheduler's unknown-corpse interval. Player corpse durations
-- in Global are unconfirmed. The UUID source key uses the existing fallback.
-- A partially settled loss cannot expire before the receipt has finished.
DO $migration$
DECLARE body TEXT; amended TEXT; anchor TEXT;
BEGIN
 body:=pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure);
 anchor:='AND m.draw_ordinal=0';
 amended:=replace(body,anchor,anchor||$arm$
 AND CASE WHEN m.loot_table_family='PlayerDeath' THEN
  game_player_death_items_complete(substring(m.loot_table_production_key from 14)::uuid)
 ELSE TRUE END$arm$);
 IF body=amended THEN RAISE EXCEPTION 'corpse retirement deadline lookup changed'; END IF;
 EXECUTE amended;
END
$migration$;

-- An intact backpack follows its own death-corpse entry. Enumerate its
-- existing bounded subtree in postorder so every descendant uses the same
-- corpse retirement owner before its immediate parent.
CREATE FUNCTION game_corpse_retirement_entries(p_corpse UUID)
RETURNS TABLE(item_instance_id UUID,parent_item_instance_id UUID,
 corpse_ordinal NUMERIC,source_ordinal NUMERIC,depth INTEGER)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.placement_ordinal,0
 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=p_corpse
 UNION ALL
 SELECT t.item_instance_id,t.parent_item_instance_id,e.placement_ordinal,t.placement_ordinal,t.depth
 FROM game_item_mint_receipts m
 JOIN game_item_corpse_container_entries e ON e.parent_item_instance_id=m.item_instance_id
 CROSS JOIN LATERAL game_item_loop_subtree_edges(e.item_instance_id) t
 WHERE m.item_instance_id=p_corpse AND m.loot_table_family='PlayerDeath'
  AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
  AND t.depth>0 AND t.source_kind=2 AND t.world_id=e.world_id AND NOT t.cyclic
$$;

-- Capture the actual old nested edge while proving the reservation's live
-- corpse, exact scope, direct-root ordinal and immediate parent.
CREATE FUNCTION game_player_corpse_remove_nested(p_tx UUID, p_parent UUID)
RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE SECURITY DEFINER AS $$
DECLARE v game_item_decay_retire_reservations%ROWTYPE;
BEGIN
 SELECT * INTO STRICT v FROM game_item_decay_retire_reservations WHERE transaction_id=p_tx;
 INSERT INTO game_item_corpse_entry_removal_evidence
  (item_instance_id,parent_item_instance_id,placement_ordinal,world_id,channel_id,scope_kind,instance_id)
 SELECT n.item_instance_id,g.item_instance_id,v.placement_ordinal,g.world_id,g.channel_id,g.scope_kind,g.instance_id
 FROM game_item_ground_locations g
 JOIN game_item_instances corpse ON corpse.item_instance_id=g.item_instance_id AND corpse.lifecycle=1
 CROSS JOIN LATERAL game_corpse_retirement_entries(g.item_instance_id) e
 JOIN game_item_nested_entries n ON n.item_instance_id=e.item_instance_id
 WHERE g.item_instance_id=coalesce(v.corpse_item_instance_id,v.ground_root_item_instance_id)
  AND v.cause_kind IN('CORPSE_DECAY','WORLD_RESET','ACTIVITY_RETIREMENT')
  AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
      IS NOT DISTINCT FROM (v.world_id,v.scope_kind,v.channel_id,v.instance_id)
  AND n.world_id=g.world_id AND e.depth>0 AND n.item_instance_id=v.item_instance_id
  AND n.parent_item_instance_id=p_parent AND e.parent_item_instance_id=p_parent
  AND n.placement_ordinal=e.source_ordinal AND e.corpse_ordinal=v.placement_ordinal
  AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries child WHERE child.parent_item_instance_id=n.item_instance_id);
 IF NOT FOUND THEN RETURN FALSE; END IF;
 DELETE FROM game_item_nested_entries WHERE item_instance_id=v.item_instance_id AND parent_item_instance_id=p_parent;
 IF NOT FOUND THEN RAISE EXCEPTION 'player corpse nested custody changed before retirement' USING ERRCODE='23514'; END IF;
 RETURN TRUE;
END
$$;

DO $migration$
DECLARE body TEXT; signature TEXT; begin_at INTEGER;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_corpse_retirement_entries(uuid)','game_player_corpse_remove_nested(uuid,uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'nested entry removal guard changed'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(
  SELECT 1 FROM game_item_corpse_entry_removal_evidence e
  JOIN game_item_decay_retire_receipts r USING(item_instance_id)
  JOIN game_item_mint_receipts m ON m.item_instance_id=e.parent_item_instance_id
  WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=OLD.world_id
   AND e.created_xact_id=pg_current_xact_id() AND r.created_xact_id=e.created_xact_id
   AND r.cause_kind IN('CORPSE_DECAY','WORLD_RESET','ACTIVITY_RETIREMENT')
   AND e.parent_item_instance_id=coalesce(r.corpse_item_instance_id,r.ground_root_item_instance_id)
   AND e.placement_ordinal=r.placement_ordinal AND m.loot_table_family='PlayerDeath'
   AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
 ) THEN RETURN NULL; END IF;
 $arm$ FROM begin_at FOR 5);
END
$migration$;
