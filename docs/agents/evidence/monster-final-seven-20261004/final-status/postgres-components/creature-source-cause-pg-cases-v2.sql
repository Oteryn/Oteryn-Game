-- Actual PostgreSQL17.6 physical/deferred guard tests, component fixture only.
-- Run after real0001..0077 migrations in an OWNED isolated test schema.
-- Existing Player regression script must run in its separate existing schema.
-- This SQL does not claim the native Creature/fence producer has been qualified.
CREATE FUNCTION fixture_creature_source_field(occurrence TEXT,actor_generation BIGINT DEFAULT 1,scope_generation BIGINT DEFAULT 1,bad INTEGER DEFAULT 0) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7();e UUID:=game_character_uuid_v7();i UUID:=game_character_uuid_v7();
 now_ms BIGINT:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 placement BYTEA:=decode(repeat('01',16),'hex'); content BYTEA:=decode(repeat('02',32),'hex');
 map BYTEA:=decode(repeat('03',32),'hex');frame BYTEA:=decode(repeat('04',32),'hex');
 body BYTEA:=decode(repeat('05',32),'hex');cast_digest BYTEA:=decode(repeat('06',32),'hex');p JSONB;intent BYTEA;
BEGIN
 p:=jsonb_build_object('schema','OTERYN_CREATURE_GROUND_CAUSE/v1','player',NULL,'cost',NULL,'caster_origin',NULL,
 'world','10000000000070008000000000000003','channel','10000000000070008000000000000005','ownership_generation',scope_generation::text,
 'creature',jsonb_build_object('key','oteryn:creature.angry_demon','revision','definition-r1','placement',encode(placement,'hex'),'generation',actor_generation::text),
 'ability',jsonb_build_object('family','Ability','key','oteryn:ability.creature.angry_demon.attack-4','revision','definition-r1'),
 'occurrence',occurrence,'content',encode(content,'hex'),'map',encode(map,'hex'),'frame',encode(frame,'hex'),'source_body',encode(body,'hex'),'source_cast',encode(cast_digest,'hex'));
 IF bad=1 THEN p:=jsonb_set(p,'{creature,generation}','"2"'); END IF;
 IF bad=2 THEN p:=jsonb_set(p,'{world}','"20000000000070008000000000000003"'); END IF;
 intent:=convert_to(p::text,'UTF8');
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,operation_count,occurred_at_unix_ms,cause_kind,creature_placement,creature_actor_generation,creature_definition_key,creature_definition_revision,creature_occurrence,creature_map_digest,creature_frame_digest,creature_source_body_digest,creature_cast_digest)
 VALUES(t,e,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',scope_generation,CASE WHEN bad=3 THEN 'Spell' ELSE 'Ability' END,'oteryn:ability.creature.angry_demon.attack-4','definition-r1',content,sha256(intent),intent,1,now_ms,1,placement,CASE WHEN bad=4 THEN NULL ELSE actor_generation END,'oteryn:creature.angry_demon','definition-r1',occurrence,map,frame,body,cast_digest);
 INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid)
 VALUES(t,1,1,i,'Item','oteryn:item.tibia.i2118','definition-r1',0,1,0,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',decode('00000001000000020000','hex'),'map-r1','content-r1',decode('01','hex'),content,false,false,false);
 INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES(i,'10000000-0000-7000-8000-000000000003','Item','oteryn:item.tibia.i2118','definition-r1',1,1,t);
 INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
 VALUES(i,'10000000-0000-7000-8000-000000000003','10000000-0000-7000-8000-000000000005',scope_generation,decode('00000001000000020000','hex'),decode('01','hex'),'map-r1','content-r1',decode('01','hex'));
 INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
 VALUES(e,t,now_ms,now_ms+7776000000,intent,sha256(intent));
 RETURN i;
END $$;
SELECT fixture_creature_source_field('A');
SELECT fixture_creature_source_field('B');
DO $$ DECLARE before_count BIGINT; failure INTEGER;BEGIN
 SELECT count(*) INTO before_count FROM game_item_instances;
 BEGIN PERFORM fixture_creature_source_field('A');SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'duplicate source A after B accepted';EXCEPTION WHEN unique_violation THEN NULL;END;
 IF (SELECT count(*) FROM game_item_instances)<>before_count THEN RAISE EXCEPTION 'duplicate source partially minted';END IF;
 FOR failure IN 1..4 LOOP
  BEGIN PERFORM fixture_creature_source_field('bad-'||failure,1,1,failure);SET CONSTRAINTS ALL IMMEDIATE;RAISE EXCEPTION 'bad typed Creature cause accepted %',failure;EXCEPTION WHEN check_violation THEN NULL;END;
 END LOOP;
 IF (SELECT count(*) FROM game_item_instances)<>before_count THEN RAISE EXCEPTION 'bad source partially minted';END IF;
END $$;
-- A new independently qualified scope/actor epoch is a distinct legitimate cause.
-- Current-fence proof is the Rust producer's responsibility, not this component SQL fixture.
SELECT fixture_creature_source_field('A',1,2);
SELECT fixture_creature_source_field('A',2,1);
DO $$ BEGIN
 IF (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=1)<>4 THEN RAISE EXCEPTION 'exact source epoch cases missing';END IF;
 IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE cause_kind=1 AND (game_session_id IS NOT NULL OR character_id IS NOT NULL OR command_id IS NOT NULL OR cost IS NOT NULL OR cost_binding IS NOT NULL)) THEN RAISE EXCEPTION 'Creature impersonated Player/cost';END IF;
 BEGIN UPDATE game_spell_item_receipts SET creature_cast_digest=decode(repeat('07',32),'hex') WHERE cause_kind=1;RAISE EXCEPTION 'source receipt mutable';EXCEPTION WHEN check_violation THEN NULL;END;
END $$;
