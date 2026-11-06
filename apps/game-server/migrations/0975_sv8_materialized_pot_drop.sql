-- GAP-SV8 W19b: Crystal 201189f movements/seeds.lua:3-17.
-- AddItem is a consequence of the existing native move, not a second transaction.
-- Existing USE/cosmetic guards remain intact. A Ground extra input is admitted only
-- by the exact same-XID move line, its current Ground custody, and its empty pot.
-- Forward rollback: disable native seed AddItem admission; retain all receipts.
CREATE FUNCTION game_sv8_seed_drop_shape(p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,
 p_before JSONB,p_effects JSONB,p_extra JSONB,p_outputs JSONB,p_recipe TEXT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(p_kind=3 AND p_consume AND p_count=1
  AND p_before->>'definition_family'='Item'
  AND p_before->>'definition_production_key'='oteryn:item.tibia.i306'
  AND (p_before->>'quantity')::bigint=1
  AND jsonb_array_length(p_extra)=1
  AND (p_extra->0->>'source_kind')::integer=3
  AND (p_extra->0->>'count')::integer=1
  AND p_extra->0->'parent'='null'::jsonb
  AND jsonb_typeof(p_extra->0->'drop')='object'
  AND jsonb_typeof(p_extra->0->'drop'->'ground')='object'
  AND game_character_is_uuid_v7((p_extra->0->'drop'->>'move')::uuid)
  AND p_extra->0->'before'->>'definition_family'='Item'
  AND jsonb_array_length(p_outputs)=1 AND (p_outputs->0->>'count')::bigint=1
  AND ((p_extra->0->'before'->>'definition_production_key'='oteryn:item.tibia.i13844'
        AND p_recipe='oteryn:planting.ice-flower' AND p_outputs->0->>'key'='oteryn:item.tibia.i14031'
        AND p_effects->>'source_item_id'='13844'
        AND p_effects->>'donor'='crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/ice_flower_seeds.lua')
    OR (p_extra->0->'before'->>'definition_production_key'='oteryn:item.tibia.i647'
        AND p_recipe='oteryn:planting.seed-drop' AND p_outputs->0->>'key'='oteryn:item.tibia.i324'
        AND p_effects->>'source_item_id'='647'
        AND p_effects->>'donor'='crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/movements/seeds.lua'))
  AND jsonb_typeof(p_effects->'ground')='object'
  AND p_effects-ARRAY['ground','source_item_id','donor']=jsonb_build_object(
    'kind','transform','cues','[{"binding":"appearance:effect/poff","effect_id":3}]'::jsonb,
    'private_speech',FALSE,'require_head',FALSE,'require_player_target',FALSE,
    'achievement_progress','[]'::jsonb),FALSE)
$$;

CREATE FUNCTION game_sv8_seed_drop_custody(r game_item_consume_transform_records,e JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT coalesce(game_sv8_seed_drop_shape(r.source_kind,r.consume_source,r.source_count,
    r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe)
  AND e=r.extra_inputs->0 AND e->>'item'<>r.source_item_instance_id::text
  AND (e->>'ordinal')::bigint=(e->'drop'->'ground'->>'stack_ordinal')::bigint
  AND EXISTS(SELECT 1 FROM game_item_loop_move_receipts m
    JOIN game_item_loop_move_lines l USING(transaction_id)
    JOIN game_item_ground_locations g ON g.item_instance_id=l.item_instance_id
    WHERE m.transaction_id=(e->'drop'->>'move')::uuid
      AND m.created_xact_id=pg_current_xact_id() AND m.game_session_id=r.game_session_id
      AND m.command_id=r.command_id AND m.character_id=r.character_id AND m.world_id=r.world_id
      AND m.intent=r.intent AND l.item_instance_id=(e->>'item')::uuid AND l.to_kind=3
      AND l.quantity_after=(e->'before'->>'quantity')::bigint
      AND to_jsonb(g)=e->'drop'->'ground'
      AND g.world_id=r.world_id AND g.channel_id=m.channel_id
      AND g.runtime_scope_ownership_generation=m.scope_generation
      AND g.spatial_position=l.to_spatial_position
      AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(m.transaction_id)
      AND (to_jsonb(g)-ARRAY['item_instance_id','stack_ordinal','corpse_ref','native_room_placement_context'])=
          (r.source_location-ARRAY['item_instance_id','stack_ordinal','corpse_ref','native_room_placement_context','qualified_house'])
      AND (to_jsonb(g)->'native_room_placement_context'=r.source_location->'native_room_placement_context'
        OR game_item_action_ground_source(r.source_item_instance_id)
        OR EXISTS(SELECT 1 FROM game_map_item_mint_receipts pot_mint
          WHERE pot_mint.item_instance_id=r.source_item_instance_id
            AND pot_mint.transaction_id=(r.before_item->>'minted_transaction_id')::uuid
            AND pot_mint.world_id=r.world_id AND pot_mint.channel_id=m.channel_id
            AND pot_mint.runtime_scope_ownership_generation=m.scope_generation
            AND pot_mint.spatial_position=g.spatial_position
            AND pot_mint.map_revision=g.map_revision AND pot_mint.content_revision=g.content_revision
            AND ('\x'||encode(pot_mint.base_bundle_digest,'hex'))=r.source_location->>'native_room_placement_context'
            AND r.source_location->>'corpse_ref'=('\x'||encode(convert_to('map-item-materialization-v1','UTF8'),'hex'))))
      AND ((l.from_kind=0 AND (e->'before'->>'minted_transaction_id')::uuid=m.transaction_id
             AND e->'before'->>'last_transaction_id' IS NULL AND (e->'before'->>'state_revision')::numeric=1)
        OR (l.from_kind<>0 AND (e->'before'->>'last_transaction_id')::uuid=m.transaction_id
             AND (e->'before'->>'state_revision')::numeric=l.state_revision_before+1))
      AND (e->'before'->>'world_id')::uuid=r.world_id),FALSE)
$$;

-- Native move receipts retain their original bounded internal intent bytes.
-- Ordinary USE keeps its existing 529-byte admission arm unchanged.
DO $migration$
DECLARE constraint_name TEXT; definition TEXT;
BEGIN
 SELECT conname,pg_get_constraintdef(oid) INTO STRICT constraint_name,definition
 FROM pg_constraint WHERE conrelid='game_item_consume_transform_records'::regclass
   AND contype='c' AND position('octet_length(intent)' IN pg_get_constraintdef(oid))>0;
 IF left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing bounded consume intent proof'; END IF;
 EXECUTE format('ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT %I',constraint_name);
 EXECUTE format('ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT %I CHECK (%s OR (octet_length(intent) BETWEEN 1 AND 4096 AND game_sv8_seed_drop_shape(source_kind,consume_source,source_count,before_item,effects,extra_inputs,outputs,recipe)))',constraint_name,substring(definition FROM 7));
END $migration$;

DO $migration$
DECLARE body TEXT; original TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_action_extra_custody(game_item_consume_transform_records,jsonb)'::regprocedure);
 original:='ELSE FALSE END';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing native action extra custody arms'; END IF;
 EXECUTE replace(body,original,'WHEN 3 THEN game_sv8_seed_drop_custody(r,e) ELSE FALSE END');

 -- Keep all plain-input, exact predecessor, one-unit and alias checks. Only the
 -- custody helper above gains this new independently proven Ground source kind.
 body:=pg_get_functiondef('game_item_action_extra_complete(game_item_consume_transform_records)'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing native action extra completion'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 FOR e IN SELECT value FROM jsonb_array_elements(p_record.extra_inputs) LOOP
  IF (e->>'source_kind')::integer=3 AND EXISTS(
    SELECT 1 FROM game_item_instances seed JOIN game_item_ground_locations g USING(item_instance_id)
    WHERE seed.item_instance_id=(e->>'item')::uuid AND seed.quantity=0)
  THEN RAISE EXCEPTION 'consumed native seed retains Ground custody' USING ERRCODE='23514'; END IF;
 END LOOP;
$arm$ from begin_at for 5);

 body:=pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing native Ground removal proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,
   LATERAL jsonb_array_elements(r.extra_inputs) e,game_item_instances seed
   WHERE game_sv8_seed_drop_shape(r.source_kind,r.consume_source,r.source_count,
       r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe)
     AND (e->>'item')::uuid=OLD.item_instance_id AND e->'drop'->'ground'=to_jsonb(OLD)
     AND seed.item_instance_id=OLD.item_instance_id AND seed.quantity=0 AND seed.lifecycle=2
     AND seed.last_transaction_id=r.transaction_id AND r.created_xact_id=pg_current_xact_id())
 THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);

 -- Existing Source USE shape stays byte-for-byte intact as the first SQL arm.
 SELECT prosrc INTO body FROM pg_proc
   WHERE oid='game_event_pot_planting(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text)'::regprocedure;
 IF position('p_recipe=''oteryn:planting.ice-flower''' IN body)=0 THEN
  RAISE EXCEPTION 'missing exact pot planting shape'; END IF;
 body:=rtrim(body,E' \n\r\t;')||' OR game_sv8_seed_drop_shape(p_kind,p_consume,p_count,p_before,p_effects,p_extra,p_outputs,p_recipe)';
 EXECUTE format('CREATE OR REPLACE FUNCTION game_event_pot_planting(p_kind integer,p_consume boolean,p_count bigint,p_before jsonb,p_effects jsonb,p_extra jsonb,p_outputs jsonb,p_recipe text) RETURNS boolean LANGUAGE sql IMMUTABLE AS %L',body);

 -- Source XML ordinary plant has the same two absolute twenty-hour stages as
 -- the existing ice plant. The remaining same-XID source/stage/hash checks stay.
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 original:='IF initial=''oteryn:item.tibia.i14031'' THEN';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing event plant timer source'; END IF;
 EXECUTE replace(body,original,$arm$
 IF initial='oteryn:item.tibia.i324' THEN
  expected_count:=2; expected_duration:=72000000;
  expected_source:=CASE NEW.source_stage WHEN 1 THEN initial WHEN 2 THEN 'oteryn:item.tibia.i330' END;
  expected_target:=CASE NEW.source_stage WHEN 1 THEN 'oteryn:item.tibia.i330' WHEN 2 THEN 'oteryn:item.tibia.i306' END;
  IF NOT game_sv8_seed_drop_shape(r.source_kind,r.consume_source,r.source_count,
    r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe)
  THEN RAISE EXCEPTION 'ordinary plant timer requires its native AddItem receipt' USING ERRCODE='23514'; END IF;
 ELSIF initial='oteryn:item.tibia.i14031' THEN
$arm$);

 -- W07's physical bundle frame is +z; captured source rooms retain native -z.
 -- All staging coordinates, roster/current-session and core guards are retained.
 body:=pg_get_functiondef('game_soulpit_participant_admit()'::regprocedure);
 original:='p.entry_floor=-8';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing Soulpit staging floor proof'; END IF;
 EXECUTE replace(body,original,'p.entry_floor IN(-8,8)');
END $migration$;

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_sv8_seed_drop_shape(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text)',
  'game_sv8_seed_drop_custody(game_item_consume_transform_records,jsonb)',
  'game_event_pot_planting(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $migration$;
