\set ON_ERROR_STOP on
SET search_path=spell_house_privacy_audit,pg_catalog;
-- Persistence fixture only: these historical rows do not mint a current Node
-- incarnation, recovery seal, map proof, or runtime initialization permission.
SET session_replication_role=replica;
INSERT INTO game_runtime_scope_assignments(scope_key,world_id,channel_id,ownership_generation,state,holder_node_id,holder_registration_revision,source_revision,decision_identity,operation_key,decided_at) VALUES(
 decode('01','hex')||uuid_send('019a0000-0000-7000-8000-000000000002')||uuid_send('019a0000-0000-7000-8000-000000000102'),
 '019a0000-0000-7000-8000-000000000002','019a0000-0000-7000-8000-000000000102',1,1,'019a0000-0000-7000-8000-000000000103',1,900,'map-fixture',decode(repeat('aa',32),'hex'),0);
INSERT INTO game_durability_admission_runtime_guards VALUES(
 decode('01','hex')||uuid_send('019a0000-0000-7000-8000-000000000002')||uuid_send('019a0000-0000-7000-8000-000000000102'),
 1,true,900,'fixture',900,'map-fixture',0,0,'{}');
SET session_replication_role=origin;
CREATE FUNCTION test_map_row(n int,missing_audit bool DEFAULT false) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_native_map_item_receipts%ROWTYPE;
BEGIN
 r.transaction_id:=game_character_uuid_v7(); r.event_id:=game_character_uuid_v7(); r.item_instance_id:=game_character_uuid_v7();
 r.world_id:='019a0000-0000-7000-8000-000000000002';r.channel_id:='019a0000-0000-7000-8000-000000000102';r.scope_generation:=1;
 r.holder_node_id:='019a0000-0000-7000-8000-000000000103';r.holder_registration_revision:=1;
 r.content_digest:=decode(repeat('11',32),'hex');r.map_digest:=decode(repeat('22',32),'hex');r.frame_digest:=decode(repeat('33',32),'hex');
 r.placement_key:='source-cell:ordinal'||n;r.definition_family:='Item';r.definition_key:='oteryn:content.item.test';r.definition_revision:='r1';r.quantity:=1;
 r.spatial_position:=decode('0000000100000002fff9','hex');r.map_revision:='sha256:'||encode(r.map_digest,'hex');r.content_revision:='sha256:'||encode(r.content_digest,'hex');r.placement_context:=r.frame_digest;
 r.source_binding:=convert_to('{"fixture":"source-binding"}','UTF8');r.source_attributes:='{}';r.blocks_movement:=false;r.blocks_projectile:=false;r.immovable_block_solid:=false;
 r.source_intent:=convert_to(jsonb_build_object('schema','OTERYN_NATIVE_MAP_ITEM_INIT/v1','world',encode(uuid_send(r.world_id),'hex'),'channel',encode(uuid_send(r.channel_id),'hex'),'scope_generation',r.scope_generation::text,'content_digest',encode(r.content_digest,'hex'),'map_digest',encode(r.map_digest,'hex'),'frame_digest',encode(r.frame_digest,'hex'),'placement_key',r.placement_key,'definition_family',r.definition_family,'definition_key',r.definition_key,'definition_revision',r.definition_revision,'quantity',r.quantity,'spatial_position',encode(r.spatial_position,'hex'),'source_binding',encode(r.source_binding,'hex'),'source_attributes',r.source_attributes,'blocks_movement',r.blocks_movement,'blocks_projectile',r.blocks_projectile,'immovable_block_solid',r.immovable_block_solid,'owner_kind',r.owner_kind)::text,'UTF8');
 r.binding:=sha256(r.source_intent);r.created_xact_id:=pg_current_xact_id();
 INSERT INTO game_native_map_item_receipts SELECT r.*;
 INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id,last_transaction_id,state_revision) VALUES(r.item_instance_id,r.world_id,r.definition_family,r.definition_key,r.definition_revision,r.quantity,1,r.transaction_id,NULL,1);
 INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(r.item_instance_id,r.world_id,r.channel_id,r.scope_generation,r.spatial_position,convert_to('native-source-map-init-r21','UTF8'),r.map_revision,r.content_revision,r.placement_context);
 IF NOT missing_audit THEN INSERT INTO game_native_map_item_audit VALUES(r.event_id,r.transaction_id,r.item_instance_id,r.source_intent,sha256(r.source_intent),pg_current_xact_id());END IF;
END $$;
BEGIN; SELECT test_map_row(1); SET CONSTRAINTS ALL IMMEDIATE; COMMIT;
DO $$ DECLARE r game_native_map_item_receipts%ROWTYPE; BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_native_map_item_receipts JOIN game_item_instances USING(item_instance_id) JOIN game_item_ground_locations USING(item_instance_id) JOIN game_native_map_item_audit USING(item_instance_id)) THEN RAISE EXCEPTION 'map Item cause not materialized'; END IF;
 BEGIN PERFORM test_map_row(2,true);SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'missing private map audit accepted';EXCEPTION WHEN check_violation THEN NULL;END;
 BEGIN UPDATE game_native_map_item_receipts SET quantity=2;RAISE EXCEPTION 'immutable source replaced';EXCEPTION WHEN check_violation THEN NULL;END;
 BEGIN DELETE FROM game_native_map_item_audit;RAISE EXCEPTION 'map audit erased';EXCEPTION WHEN check_violation THEN NULL;END;
 SELECT * INTO r FROM game_native_map_item_receipts LIMIT 1;
 BEGIN
  r.transaction_id:=game_character_uuid_v7();r.event_id:=game_character_uuid_v7();r.item_instance_id:=game_character_uuid_v7();r.placement_key:='different';r.quantity:=2;
  INSERT INTO game_native_map_item_receipts SELECT r.*; RAISE EXCEPTION 'intent-column substitution accepted';
 EXCEPTION WHEN check_violation THEN NULL;END;
 BEGIN
  r.source_intent:=convert_to(replace(convert_from(r.source_intent,'UTF8'),'"scope_generation": "1"','"scope_generation": "2"'),'UTF8');
  r.scope_generation:=2;r.binding:=sha256(r.source_intent);
  -- Repair unrelated substitutions, leaving just the stale current generation.
  r.quantity:=1;r.placement_key:='source-cell:ordinal1';
  INSERT INTO game_native_map_item_receipts SELECT r.*;RAISE EXCEPTION 'stale map generation accepted';
 EXCEPTION WHEN check_violation THEN NULL;END;
END $$;
-- Simulate a separately accepted newer assignment as historical fixture rows;
-- no real runtime/recovery authority is constructed by this SQL setup.
SET session_replication_role=replica;
UPDATE game_runtime_scope_assignments SET ownership_generation=2 WHERE decision_identity='map-fixture';
UPDATE game_durability_admission_runtime_guards SET ownership_generation=2 WHERE decision_identity='map-fixture';
SET session_replication_role=origin;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
INSERT INTO game_native_map_scope_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before)
 SELECT r.transaction_id,2,r.holder_node_id,r.holder_registration_revision,to_jsonb(g) FROM game_native_map_item_receipts r JOIN game_item_ground_locations g USING(item_instance_id);
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 BEGIN UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2;
  RAISE EXCEPTION 'runtime unexpectedly has direct Ground UPDATE privilege';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 IF (SELECT count(*) FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts))<>1 OR
 NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_receipts r USING(item_instance_id)
 WHERE i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL AND g.runtime_scope_ownership_generation=2 AND r.scope_generation=1) THEN
 RAISE EXCEPTION 'map custody adoption reminted or rewrote original Item';END IF;
END $$;
COMMIT;
DO $$ BEGIN
 BEGIN UPDATE game_item_ground_locations SET spatial_position=decode('0000000200000002fff9','hex');RAISE EXCEPTION 'map handoff changed position';EXCEPTION WHEN check_violation THEN NULL;END;
 BEGIN UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=3;RAISE EXCEPTION 'unproven Ground generation advanced';EXCEPTION WHEN check_violation THEN NULL;END;
 BEGIN UPDATE game_native_map_scope_adoptions SET ownership_generation=3;RAISE EXCEPTION 'adoption historical scope rewritten';EXCEPTION WHEN check_violation THEN NULL;END;
END $$;
-- Restart/retry observes the same immutable adoption; no second mutation/mint.
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
INSERT INTO game_native_map_scope_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before)
 SELECT transaction_id,2,holder_node_id,holder_registration_revision,NULL FROM game_native_map_item_receipts
 ON CONFLICT(source_transaction_id,ownership_generation) DO NOTHING;
DO $$ BEGIN
 IF (SELECT count(*) FROM game_native_map_scope_adoptions)<>1 THEN RAISE EXCEPTION 'restart duplicated handoff'; END IF;
END $$;
COMMIT;
-- A separately accepted fixture owner changes both scope generation and holder.
SET session_replication_role=replica;
UPDATE game_runtime_scope_assignments SET ownership_generation=3,holder_node_id='019a0000-0000-7000-8000-000000000104',holder_registration_revision=2 WHERE decision_identity='map-fixture';
UPDATE game_durability_admission_runtime_guards SET ownership_generation=3 WHERE decision_identity='map-fixture';
SET session_replication_role=origin;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
DO $$ DECLARE r game_native_map_item_receipts%ROWTYPE;g JSONB; BEGIN
 SELECT * INTO STRICT r FROM game_native_map_item_receipts;
 SELECT to_jsonb(x) INTO STRICT g FROM game_item_ground_locations x WHERE item_instance_id=r.item_instance_id;
 BEGIN
  INSERT INTO game_native_map_scope_adoptions VALUES(r.transaction_id,3,r.holder_node_id,2,g,pg_current_xact_id());
  RAISE EXCEPTION 'previous holder acquired current map custody';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO game_native_map_scope_adoptions VALUES(r.transaction_id,3,'019a0000-0000-7000-8000-000000000104',1,g,pg_current_xact_id());
  RAISE EXCEPTION 'stale holder revision acquired map custody';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO game_native_map_scope_adoptions VALUES(r.transaction_id,2,'019a0000-0000-7000-8000-000000000104',2,g,pg_current_xact_id());
  RAISE EXCEPTION 'stale generation acquired map custody';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO game_native_map_scope_adoptions VALUES(r.transaction_id,3,'019a0000-0000-7000-8000-000000000104',2,jsonb_set(g,'{spatial_position}','"substituted"'),pg_current_xact_id());
  RAISE EXCEPTION 'substituted Ground acquired map custody';
 EXCEPTION WHEN check_violation THEN NULL; END;
 INSERT INTO game_native_map_scope_adoptions VALUES(r.transaction_id,3,'019a0000-0000-7000-8000-000000000104',2,g,pg_current_xact_id());
END $$;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE runtime_scope_ownership_generation=3)
 OR (SELECT count(*) FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts))<>1
 THEN RAISE EXCEPTION 'future-generation handoff failed or reminted'; END IF;
END $$;
COMMIT;
-- Historical moved-away custody must remain absent after a later owner adopts.
SET session_replication_role=replica;
DELETE FROM game_item_ground_locations WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts);
UPDATE game_runtime_scope_assignments SET ownership_generation=4 WHERE decision_identity='map-fixture';
UPDATE game_durability_admission_runtime_guards SET ownership_generation=4 WHERE decision_identity='map-fixture';
SET session_replication_role=origin;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
INSERT INTO game_native_map_scope_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before)
 SELECT transaction_id,4,'019a0000-0000-7000-8000-000000000104',2,NULL FROM game_native_map_item_receipts;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts))
 THEN RAISE EXCEPTION 'handoff recreated moved-away Ground'; END IF;
END $$;
COMMIT;
DROP FUNCTION test_map_row(int,bool);
SELECT 'Map initialization exact Item/Ground/audit, immutable provenance, intent substitution, current-generation guards: PASS' AS result;
