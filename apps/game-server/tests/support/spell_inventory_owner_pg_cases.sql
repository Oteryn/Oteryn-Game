-- Real source guard fixture; run after spell_item_owner_pg_cases.sql.
-- No Item seeding or invented creature death.
\set ON_ERROR_STOP on
SET search_path=spell_items_asset_audit,pg_catalog;
CREATE FUNCTION fixture_inventory_mint(command BIGINT,item_key TEXT,amount INTEGER) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE delay_ms BIGINT:=NULL; with_audit BOOLEAN:=true; origin_case INTEGER:=0; t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i UUID:=game_character_uuid_v7();
 now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 cost BYTEA; intent BYTEA;
BEGIN
 cost:=decode('7b2261667465725f7265766973696f6e223a312c226265666f72655f7265766973696f6e223a302c226361737465725f6166746572223a5b322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c325d2c226361737465725f6265666f7265223a5b312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c315d2c22636f6f6c646f776e735f6166746572223a5b5b227370656c6c3a66697874757265222c313030303030305d5d2c22636f6f6c646f776e735f6265666f7265223a5b5d2c226d616e615f6166746572223a35302c226d616e615f6265666f7265223a3130302c22736f756c5f6166746572223a342c22736f756c5f6265666f7265223a352c2276657273696f6e223a317d','hex');
 intent:=convert_to(jsonb_build_object('fixture',command,'cost',to_jsonb(ARRAY(SELECT get_byte(cost,n) FROM generate_series(0,octet_length(cost)-1)n)))::text,'UTF8');
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
 RETURN i;
END $$;

CREATE FUNCTION fixture_inventory_transfer(command BIGINT,item UUID,parent UUID DEFAULT NULL) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i game_item_instances%ROWTYPE;
 n BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::BIGINT; ordinal BIGINT:=1;
 w UUID:='10000000-0000-7000-8000-000000000003'; ch UUID:='10000000-0000-7000-8000-000000000005';
 c UUID:='10000000-0000-7000-8000-000000000001'; s UUID:='10000000-0000-7000-8000-000000000004';
 intent BYTEA:=decode(repeat('01',33),'hex'); envelope BYTEA:=convert_to('guard-fixture-transfer','UTF8');
BEGIN
 SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=item FOR UPDATE;
 INSERT INTO game_item_transfer_reservations VALUES(s,command,c,w,ch,item,CASE WHEN parent IS NULL THEN 1 ELSE 2 END,intent,t,e,n,0,n);
 INSERT INTO game_item_transfer_receipts(game_session_id,command_id,character_id,intent_binding,transaction_id,event_id,shape,source_item_instance_id,source_quantity_before,source_quantity_after,destination_parent_item_instance_id,destination_ordinal,occurred_at,envelope_sha256,committed_at)
 VALUES(s,command,c,intent,t,e,CASE WHEN parent IS NULL THEN 1 ELSE 2 END,item,i.quantity,i.quantity,parent,CASE WHEN parent IS NULL THEN NULL ELSE ordinal END,n,sha256(envelope),n);
 INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state)
 VALUES(e,t,1,1,2,1,'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1',item,n,n+7776000000,envelope,sha256(envelope),1);
 UPDATE game_item_instances SET last_transaction_id=t WHERE item_instance_id=item;
 DELETE FROM game_item_ground_locations WHERE item_instance_id=item;
 IF parent IS NULL THEN INSERT INTO game_item_container_slots VALUES(c,item,w,t);
 ELSE INSERT INTO game_item_container_entries VALUES(item,w,c,parent,ordinal,t); END IF;
END $$;

CREATE FUNCTION fixture_inventory_consume(command BIGINT,item UUID,remaining INTEGER,wrong_ordinal BOOLEAN DEFAULT FALSE,missing_ordinal BOOLEAN DEFAULT FALSE) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i game_item_instances%ROWTYPE; entry game_item_container_entries%ROWTYPE;
 r game_spell_item_receipts%ROWTYPE; cost BYTEA; intent BYTEA; n BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::BIGINT;
BEGIN
 SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=item FOR UPDATE;
 SELECT * INTO STRICT entry FROM game_item_container_entries WHERE item_instance_id=item FOR UPDATE;
 SELECT * INTO STRICT r FROM game_spell_item_receipts WHERE command_id=1;
 cost:=r.cost; intent:=convert_to(jsonb_build_object('fixture',command,'cost',to_jsonb(ARRAY(SELECT get_byte(cost,j) FROM generate_series(0,octet_length(cost)-1)j)))::TEXT,'UTF8');
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms)
 VALUES(t,e,r.game_session_id,command,r.character_id,r.world_id,r.channel_id,1,'Spell','fixture:consume','r1',r.catalog_digest,sha256(intent),intent,cost,sha256(cost),1,n);
 INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,source_parent_item_instance_id,source_placement_ordinal)
 VALUES(t,1,5,item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,remaining,i.state_revision,r.world_id,r.channel_id,decode('00000001000000020000','hex'),'map-r1','content-r1',decode('01','hex'),decode(repeat('02',32),'hex'),false,false,false,entry.parent_item_instance_id,CASE WHEN missing_ordinal THEN NULL WHEN wrong_ordinal THEN entry.placement_ordinal+1 ELSE entry.placement_ordinal END);
 UPDATE game_item_instances SET quantity=remaining,lifecycle=CASE WHEN remaining=0 THEN 2 ELSE 1 END,last_transaction_id=t WHERE item_instance_id=item;
 IF remaining=0 THEN DELETE FROM game_item_container_entries WHERE item_instance_id=item; END IF;
 INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256) VALUES(e,t,n,n+7776000000,intent,sha256(intent));
 RETURN t;
END $$;
SELECT fixture_inventory_mint(101,'oteryn:item.tibia.i1988',1) AS bag \gset
SELECT fixture_inventory_transfer(102,:'bag');
SELECT fixture_inventory_mint(103,'oteryn:item.tibia.i3582',95) AS food \gset
SELECT fixture_inventory_transfer(104,:'food',:'bag');
SELECT fixture_inventory_consume(105,:'food',94);
SELECT fixture_inventory_consume(106,:'food',93);
DO $$ DECLARE item UUID:=(SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3582'); BEGIN
 BEGIN
  PERFORM fixture_inventory_consume(107,item,92,true); SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'wrong actual inventory custody was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=107) THEN RAISE EXCEPTION 'wrong custody failure was not rolled back'; END IF;
 BEGIN
  PERFORM fixture_inventory_consume(108,item,92,false,true); SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'NULL source ordinal was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=108) THEN RAISE EXCEPTION 'NULL custody failure was not rolled back'; END IF;
 IF (SELECT quantity FROM game_item_instances WHERE item_instance_id=item)<>93 THEN RAISE EXCEPTION 'rejected consumption changed real quantity'; END IF;
 PERFORM fixture_inventory_consume(109,item,0); SET CONSTRAINTS ALL IMMEDIATE;
 IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=item) THEN RAISE EXCEPTION 'retired source retained custody'; END IF;
END $$;
SELECT 'SPELL_INVENTORY_SOURCE_GUARDS_PASS';
