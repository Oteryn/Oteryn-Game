-- Candidate row-guard tests: synthetic 1ms source stages, not runtime admission.
\set ON_ERROR_STOP on
SET search_path=spell_items_asset_audit,pg_catalog;
CREATE FUNCTION fixture_chain_mint(command BIGINT,broken BOOLEAN DEFAULT FALSE) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE delay_ms BIGINT:=NULL; with_audit BOOLEAN:=true; origin_case INTEGER:=0; t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i UUID:=game_character_uuid_v7();
 now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 cost BYTEA; intent BYTEA; chain JSONB; item_key TEXT:='field:first'; amount INTEGER:=1;
BEGIN
 cost:=decode('7b2261667465725f7265766973696f6e223a312c226265666f72655f7265766973696f6e223a302c226361737465725f6166746572223a5b322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c325d2c226361737465725f6265666f7265223a5b312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c315d2c22636f6f6c646f776e735f6166746572223a5b5b227370656c6c3a66697874757265222c313030303030305d5d2c22636f6f6c646f776e735f6265666f7265223a5b5d2c226d616e615f6166746572223a35302c226d616e615f6265666f7265223a3130302c22736f756c5f6166746572223a342c22736f756c5f6265666f7265223a352c2276657273696f6e223a317d','hex');
 chain:=jsonb_build_array(jsonb_build_object('definition',jsonb_build_object('family','Item','key','field:first','revision','r1'),'duration_millis',1,'target',jsonb_build_object('family','Item','key','field:second','revision','r1'),'blocks_movement',false,'blocks_projectile',false,'immovable_block_solid',false),jsonb_build_object('definition',jsonb_build_object('family','Item','key','field:second','revision','r1'),'duration_millis',1,'target',NULL,'blocks_movement',false,'blocks_projectile',false,'immovable_block_solid',false));
 intent:=convert_to(jsonb_build_object('operations',jsonb_build_array(jsonb_build_object('kind','mint_ground','definition',jsonb_build_object('family','Item','key','field:first','revision','r1'),'decay_chain',chain)),'fixture',command,'cost',to_jsonb(ARRAY(SELECT get_byte(cost,n) FROM generate_series(0,octet_length(cost)-1)n)))::text,'UTF8');
 IF origin_case=1 THEN intent:=convert_to((convert_from(intent,'UTF8')::jsonb||jsonb_build_object('caster_origin',jsonb_build_object('lease',1,'placement',to_jsonb(array_fill(1,ARRAY[16])))))::text,'UTF8'); END IF;
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms,caster_lease_generation,caster_placement_digest)
 VALUES(t,e,'10000000-0000-7000-8000-000000000004',command,'10000000-0000-7000-8000-000000000001','10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',1,'Spell','fixture:barrier','r1',decode(repeat('01',32),'hex'),sha256(intent),intent,cost,sha256(cost),1,now_ms,CASE WHEN origin_case=0 THEN NULL ELSE 1 END,CASE WHEN origin_case=0 THEN NULL ELSE decode(repeat('01',16),'hex') END);
 INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,expires_at_unix_ms)
 VALUES(t,1,1,i,'Item',item_key,'r1',0,amount,0,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',decode('00000001000000020000','hex'),'map-r1','content-r1',decode('01','hex'),decode(repeat('02',32),'hex'),true,true,true,CASE WHEN delay_ms IS NULL THEN NULL ELSE now_ms+delay_ms END);
 INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES(i,'10000000-0000-7000-8000-000000000003','Item',item_key,'r1',amount,1,t);
 INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
 VALUES(i,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',1,decode('00000001000000020000','hex'),decode('01','hex'),'map-r1','content-r1',decode('01','hex'));
 IF with_audit THEN INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
 VALUES(e,t,now_ms,now_ms+7776000000,intent,sha256(intent)); END IF;
 INSERT INTO game_spell_field_temporal_schedules(item_instance_id,source_stage,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,blocks_movement,blocks_projectile,immovable_block_solid)
 VALUES(i,1,t,1,1,now_ms+1,'Item','field:first','r1','Item','field:second','r1',decode(repeat('02',32),'hex'),false,false,false),
 (i,2,t,1,1,now_ms+CASE WHEN broken THEN 20 ELSE 2 END,'Item','field:second','r1',NULL,NULL,NULL,decode(repeat('02',32),'hex'),false,false,false);
 RETURN i;
END $$;

CREATE FUNCTION fixture_chain_drain(item UUID,stage INTEGER,with_audit BOOLEAN DEFAULT TRUE) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7();e UUID:=game_character_uuid_v7();r game_spell_field_temporal_receipts%ROWTYPE;p game_spell_field_temporal_schedules%ROWTYPE;body BYTEA;
BEGIN
 SELECT * INTO STRICT p FROM game_spell_field_temporal_schedules WHERE item_instance_id=item AND source_stage=stage;
 INSERT INTO game_spell_field_temporal_receipts(source_stage,transaction_id,event_id,item_instance_id,world_id,channel_id,ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,source_stack_ordinal)
 SELECT stage,t,e,item,i.world_id,g.channel_id,g.runtime_scope_ownership_generation,i.state_revision,i.quantity,floor(extract(epoch FROM statement_timestamp())*1000)::BIGINT,1,g.stack_ordinal
 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=item;
 IF p.target_family IS NULL THEN
  UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=t WHERE item_instance_id=item;
  DELETE FROM game_item_ground_locations WHERE item_instance_id=item;
 ELSE
  UPDATE game_item_instances SET definition_family=p.target_family,definition_production_key=p.target_production_key,definition_revision_ref=p.target_revision,last_transaction_id=t WHERE item_instance_id=item;
 END IF;
 IF with_audit THEN
  SELECT * INTO STRICT r FROM game_spell_field_temporal_receipts WHERE transaction_id=t;
  body:=convert_to(jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p))::TEXT,'UTF8');
  INSERT INTO game_spell_field_temporal_audit(event_id,transaction_id,envelope,envelope_sha256) VALUES(e,t,body,sha256(body));
 END IF;
 RETURN t;
END $$;
SELECT fixture_chain_mint(301) AS chain \gset
SELECT pg_sleep(0.005);
BEGIN;
SELECT fixture_chain_drain(:'chain',1);
SET CONSTRAINTS game_spell_field_temporal_receipt_proven IMMEDIATE;
SET CONSTRAINTS game_spell_field_temporal_receipt_proven DEFERRED;
SELECT fixture_chain_drain(:'chain',2);
COMMIT;
DO $$ DECLARE i UUID:=(SELECT item_instance_id FROM game_spell_field_temporal_schedules LIMIT 1); BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=i AND state_revision=3 AND lifecycle=2 AND quantity=0) THEN RAISE EXCEPTION 'chain did not transform and retire actual same instance'; END IF;
 IF EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i) OR (SELECT count(*) FROM game_spell_field_temporal_receipts WHERE item_instance_id=i)<>2 THEN RAISE EXCEPTION 'chain custody/history incomplete';END IF;
 BEGIN PERFORM fixture_chain_mint(302,TRUE);SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'wrong cumulative deadline accepted';EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=302) THEN RAISE EXCEPTION 'rejected chain source did not rollback';END IF;
END $$;
SELECT fixture_chain_mint(303) AS wrongstage \gset
SELECT pg_sleep(0.005);
DO $$ DECLARE i UUID:=(SELECT item_instance_id FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.command_id=303);BEGIN
 BEGIN PERFORM fixture_chain_drain(i,2);SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'skipped source stage accepted';EXCEPTION WHEN check_violation THEN NULL;END;
 IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts WHERE item_instance_id=i) OR (SELECT state_revision FROM game_item_instances WHERE item_instance_id=i)<>1 THEN RAISE EXCEPTION 'failed chain stage did not rollback';END IF;
 BEGIN PERFORM fixture_chain_drain(i,1,FALSE);SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'unaudited chain stage accepted';EXCEPTION WHEN check_violation THEN NULL;END;
 IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts WHERE item_instance_id=i) OR (SELECT state_revision FROM game_item_instances WHERE item_instance_id=i)<>1 THEN RAISE EXCEPTION 'failed chain audit did not rollback';END IF;
END $$;
SELECT 'SPELL_FIELD_CHAIN_REAL_SOURCE_GUARDS_PASS';
