-- Execute in an isolated PostgreSQL17.6 qualification schema after0001..0033.
-- Administrative seeding below creates a Character fixture ONLY; every item,
-- cast/expiry receipt, custody change and failure uses the actual live guards.
\set ON_ERROR_STOP on
SET search_path=spell_items_asset_audit,pg_catalog;
SET session_replication_role=replica;
INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name)
VALUES('10000000-0000-7000-8000-000000000001','10000000-0000-7000-8000-000000000002','10000000-0000-7000-8000-000000000003',1,1,'r1','r1','r1','r1','Fixture Item');
SET session_replication_role=origin;

CREATE FUNCTION fixture_spell_mint(command BIGINT,delay_ms BIGINT DEFAULT NULL,with_audit BOOLEAN DEFAULT true,origin_case INTEGER DEFAULT 0) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7(); i UUID:=game_character_uuid_v7();
 now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 cost BYTEA; intent BYTEA;
BEGIN
 cost:=decode('7b2261667465725f7265766973696f6e223a312c226265666f72655f7265766973696f6e223a302c226361737465725f6166746572223a5b322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c322c325d2c226361737465725f6265666f7265223a5b312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c312c315d2c22636f6f6c646f776e735f6166746572223a5b5b227370656c6c3a66697874757265222c313030303030305d5d2c22636f6f6c646f776e735f6265666f7265223a5b5d2c226d616e615f6166746572223a35302c226d616e615f6265666f7265223a3130302c22736f756c5f6166746572223a342c22736f756c5f6265666f7265223a352c2276657273696f6e223a317d','hex');
 intent:=convert_to(jsonb_build_object('fixture',command,'cost',to_jsonb(ARRAY(SELECT get_byte(cost,n) FROM generate_series(0,octet_length(cost)-1)n)))::text,'UTF8');
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
 RETURN i;
END $$;

CREATE FUNCTION fixture_spell_expire(item UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7();
BEGIN
 INSERT INTO game_spell_item_expiry_receipts(item_instance_id,transaction_id,event_id,source_transaction_id,source_ordinal,state_revision_before,source_stack_ordinal,world_id,channel_id,ownership_generation,expires_at_unix_ms,occurred_at_unix_ms)
 SELECT item,t,e,l.transaction_id,l.ordinal,i.state_revision,g.stack_ordinal,g.world_id,g.channel_id,2,l.expires_at_unix_ms,floor(extract(epoch FROM statement_timestamp())*1000)::bigint
 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_spell_item_lines l USING(item_instance_id) WHERE i.item_instance_id=item AND l.operation_kind=1;
 UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=t WHERE item_instance_id=item;
 DELETE FROM game_item_ground_locations WHERE item_instance_id=item;
END $$;

SELECT fixture_spell_mint(1,1) AS due \gset
SELECT fixture_spell_mint(2,60000) AS future \gset
SELECT fixture_spell_mint(4,NULL,true,1) AS attributed \gset
SELECT pg_sleep(0.005);
SELECT fixture_spell_expire(:'due');
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_instances WHERE lifecycle=2 AND quantity=0 AND state_revision=2) THEN RAISE EXCEPTION 'real expiry successor absent'; END IF;
 IF EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=2) THEN RAISE EXCEPTION 'retired item still on Ground'; END IF;
END $$;

DO $$ BEGIN
 BEGIN
  PERFORM fixture_spell_mint(3,NULL,false); SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'missing audit was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=3) THEN RAISE EXCEPTION 'failed source was not rolled back'; END IF;
 BEGIN
  PERFORM fixture_spell_mint(5,NULL,true,2); SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'missing immutable caster origin was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=5) THEN RAISE EXCEPTION 'mismatched origin source did not roll back'; END IF;
 BEGIN
  PERFORM fixture_spell_expire((SELECT item_instance_id FROM game_spell_item_lines WHERE transaction_id=(SELECT transaction_id FROM game_spell_item_receipts WHERE command_id=2)));
  SET CONSTRAINTS ALL IMMEDIATE; RAISE EXCEPTION 'early expiry was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  UPDATE game_spell_item_receipts SET binding=decode(repeat('00',32),'hex');
  RAISE EXCEPTION 'receipt rewrite was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO game_spell_item_lines SELECT transaction_id,2,operation_kind,game_character_uuid_v7(),definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,source_stack_ordinal,destination_parent_item_instance_id,destination_ordinal,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid,expires_at_unix_ms,pg_current_xact_id()
   FROM game_spell_item_lines LIMIT 1;
  SET CONSTRAINTS ALL IMMEDIATE; RAISE EXCEPTION 'historical transaction source was accepted';
 EXCEPTION WHEN check_violation OR foreign_key_violation THEN NULL; END;
 IF game_spell_cost_binding_valid(convert_to('{}','UTF8')) THEN RAISE EXCEPTION 'missing cost fields accepted'; END IF;
 -- Each negative changes one cost dimension, keeping all other actual bytes valid.
 DECLARE cost JSONB:=(SELECT convert_from(r.cost,'UTF8')::jsonb FROM game_spell_item_receipts r LIMIT 1);
 BEGIN
  IF game_spell_cost_binding_valid(convert_to(jsonb_set(cost,'{mana_before}','4294967296')::text,'UTF8')) THEN RAISE EXCEPTION 'mana u32 overflow accepted'; END IF;
  IF game_spell_cost_binding_valid(convert_to(jsonb_set(cost,'{after_revision}','0')::text,'UTF8')) THEN RAISE EXCEPTION 'non-successor revision accepted'; END IF;
  IF game_spell_cost_binding_valid(convert_to(jsonb_set(cost,'{cooldowns_after}','[["spell:fixture",1000000],["spell:fixture",1000001]]')::text,'UTF8')) THEN RAISE EXCEPTION 'duplicate cooldown key accepted'; END IF;
  IF game_spell_cost_binding_valid(convert_to(jsonb_set(cost,'{cooldowns_after}','[["spell:fixture",1.5]]')::text,'UTF8')) THEN RAISE EXCEPTION 'fractional semantic micros accepted'; END IF;
 END;
END $$;
SELECT 'SPELL_ITEM_GUARDS_PASS' AS result;
