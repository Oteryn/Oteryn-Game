-- PostgreSQL guard qualification only. Run after the isolated Item guard harness
-- and migration0051. The Character fixture is administratively seeded; this is
-- not evidence of SourceAdmission, client dispatch or physical owner composition.
\set ON_ERROR_STOP on
SET search_path=spell_items_asset_audit,pg_catalog;
CREATE OR REPLACE FUNCTION spell_items_asset_audit.fixture_description_mint(command bigint, delay_ms bigint DEFAULT NULL::bigint, with_audit boolean DEFAULT true, origin_case integer DEFAULT 0, description_case integer DEFAULT 0)
 RETURNS uuid
 LANGUAGE plpgsql
AS $function$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i UUID:=game_character_uuid_v7();
 now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 cost BYTEA; intent BYTEA; attribution TEXT;
BEGIN
 cost:=decode('7b2261667465725f7265766973696f6e223a312c226265666f72655f7265766973696f6e223a302c226361737465725f6166746572223a5b322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c325d2c226361737465725f6265666f7265223a5b312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c315d2c22636f6f6c646f776e735f6166746572223a5b5b227370656c6c3a66697874757265222c313030303030305d5d2c22636f6f6c646f776e735f6265666f7265223a5b5d2c226d616e615f6166746572223a35302c226d616e615f6265666f7265223a3130302c22736f756c5f6166746572223a342c22736f756c5f6265666f7265223a352c2276657273696f6e223a317d','hex');
 attribution:=(SELECT 'Casted by: '||name FROM game_character_roots WHERE character_id='10000000-0000-7000-8000-000000000001');
 IF description_case=1 THEN attribution:='Casted by: Forged Name';END IF;
 intent:=convert_to(jsonb_build_object('fixture',command,'cost',to_jsonb(ARRAY(SELECT get_byte(cost,n) FROM generate_series(0,octet_length(cost)-1)n)),
 'operations',jsonb_build_array(jsonb_build_object('kind','mint_ground','id',to_jsonb(ARRAY(SELECT get_byte(uuid_send(CASE WHEN description_case=4 THEN t ELSE i END),n) FROM generate_series(0,15)n)), 'description',attribution)))::text,'UTF8');
 IF origin_case=1 THEN intent:=convert_to((convert_from(intent,'UTF8')::jsonb||jsonb_build_object('caster_origin',jsonb_build_object('lease',1,'placement',to_jsonb(array_fill(1,ARRAY[16])))))::text,'UTF8'); END IF;
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms,caster_lease_generation,caster_placement_digest)
 VALUES(t,e,'10000000-0000-7000-8000-000000000004',command,'10000000-0000-7000-8000-000000000001','10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',1,'Spell','fixture:barrier','r1',decode(repeat('01',32),'hex'),sha256(intent),intent,cost,sha256(cost),1,now_ms,CASE WHEN origin_case=0 THEN NULL ELSE 1 END,CASE WHEN origin_case=0 THEN NULL ELSE decode(repeat('01',16),'hex') END);
 INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,expires_at_unix_ms)
 VALUES(t,1,1,i,'Item','oteryn:item.tibia.i2128','r1',0,1,0,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',decode('00000001000000020000','hex'),'map-r1','content-r1',decode('01','hex'),decode(repeat('02',32),'hex'),true,true,true,CASE WHEN delay_ms IS NULL THEN NULL ELSE now_ms+delay_ms END);
 INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES(i,'10000000-0000-7000-8000-000000000003','Item','oteryn:item.tibia.i2128','r1',1,1,t);
 INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
 VALUES(i,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',1,decode('00000001000000020000','hex'),decode('01','hex'),'map-r1','content-r1',decode('01','hex'));
 IF with_audit THEN INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
 VALUES(e,t,now_ms,now_ms+7776000000,intent,sha256(intent)); END IF;
 IF description_case<>2 THEN
  INSERT INTO game_spell_item_source_descriptions(item_instance_id,transaction_id,ordinal,description)
  VALUES(i,t,1,CASE WHEN description_case=3 THEN 'Casted by: Changed Text' ELSE attribution END);
 END IF;
 RETURN i;
END $function$;


BEGIN;
SELECT fixture_description_mint(501,NULL,true,0,0) AS item \gset
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_item_source_descriptions d
 JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_character_roots c USING(character_id)
 JOIN game_item_instances i USING(item_instance_id)
 WHERE d.description='Casted by: '||c.name AND i.minted_transaction_id=d.transaction_id) THEN
 RAISE EXCEPTION 'exact original caster attribution was not stored'; END IF;
END $$;
COMMIT;
DO $$ DECLARE case_no INTEGER; BEGIN
 FOR case_no IN 1..4 LOOP
  BEGIN
   SET CONSTRAINTS ALL DEFERRED;
   PERFORM fixture_description_mint(501+case_no,NULL,true,0,case_no);
   SET CONSTRAINTS ALL IMMEDIATE;
   RAISE EXCEPTION 'forged/omitted/changed attribution accepted: %',case_no;
  EXCEPTION WHEN check_violation THEN NULL;
  END;
  IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=501+case_no) THEN
   RAISE EXCEPTION 'failed attribution leaked its MINT or receipt'; END IF;
 END LOOP;
 BEGIN UPDATE game_spell_item_source_descriptions SET description='Casted by: Changed Text';
 RAISE EXCEPTION 'immutable description changed'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN DELETE FROM game_spell_item_source_descriptions;
 RAISE EXCEPTION 'immutable description deleted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN TRUNCATE game_spell_item_source_descriptions;
 RAISE EXCEPTION 'immutable description truncated'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
SELECT 'SOURCE_ITEM_DESCRIPTION_REAL_GUARDS_PASS';
