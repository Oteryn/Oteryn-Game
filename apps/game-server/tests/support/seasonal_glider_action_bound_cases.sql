-- Evaluate the installed CHECK itself over the same full-row JSON shapes used
-- by consume_transform::persist_action; this is not a substitute action writer.
CREATE FUNCTION pg_temp.srv7_glider_bound(row_json JSONB,wanted BOOLEAN,label TEXT) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE expression TEXT; actual BOOLEAN;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
 WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_extra_input_bound';
 EXECUTE 'SELECT ('||expression||') IS TRUE FROM jsonb_populate_record(NULL::game_item_consume_transform_records,$1) exact_serializer_row'
 INTO actual USING row_json;
 IF actual IS DISTINCT FROM wanted THEN RAISE EXCEPTION 'GLIDER_BOUND_% expected % got %',label,wanted,actual;END IF;
 RAISE NOTICE 'GLIDER_BOUND_PASS %',label;
END $$;
DO $$ DECLARE primary_item JSONB; inputs JSONB; candidate JSONB; changed JSONB;
BEGIN
 primary_item:=to_jsonb(jsonb_populate_record(NULL::game_item_instances,jsonb_build_object(
  'item_instance_id','00000000-0000-7000-8000-000000000071','world_id','00000000-0000-7000-8000-000000000042',
  'definition_family','Item','definition_production_key','oteryn:item.tibia.i21897','definition_revision_ref','definition-r1',
  'quantity',1,'lifecycle',1,'state_revision',1,'minted_transaction_id','00000000-0000-7000-8000-000000000082')));
 SELECT jsonb_agg(jsonb_build_object('item',('00000000-0000-7000-8000-'||lpad((100+ordinal)::text,12,'0'))::uuid,
  'before',primary_item||jsonb_build_object('item_instance_id',('00000000-0000-7000-8000-'||lpad((100+ordinal)::text,12,'0'))::uuid,
   'definition_production_key','oteryn:item.tibia.i'||item::text),
  'parent','00000000-0000-7000-8000-000000000070','ordinal',(ordinal+5)::text,'count',1,'source_kind',1) ORDER BY ordinal)
 INTO inputs FROM unnest(ARRAY[21898,21906,21902,21899,21905,21901]) WITH ORDINALITY AS actual_parts(item,ordinal);
 candidate:=jsonb_build_object('recipe','oteryn:item-cosmetic.i21897.v1','consume_source',true,'source_count',1,'source_kind',1,
  'before_item',primary_item,'outputs','[]'::jsonb,'extra_inputs',inputs,
  'effects',jsonb_build_object('kind','cosmetic','source_item_id',21897,'require_head',false,'require_player_target',false,
   'cues','[]'::jsonb,'speech',NULL,'donor','crystal:data-global/scripts/actions/mounts/mounts.lua:20-36,111-140',
   'cosmetic',jsonb_build_object('source_item_id',21897,'consumed',true,'provenance','crystal-item-cosmetics-201189f-v1',
    'grants',jsonb_build_array(jsonb_build_object('grant_key','oteryn:taming.glooth_glider','cosmetic_key','oteryn:content.mount.glooth_glider','kind','mount','addon',0)),
    'progress',NULL,'achievements',jsonb_build_array('oteryn:achievement/fabled_construction','oteryn:achievement/natural_born_cowboy'),
    'achievement_progress','[]'::jsonb,'feedback','You successfully constructed a Glooth Glider!')));
 PERFORM pg_temp.srv7_glider_bound(candidate,TRUE,'exact_six_real_parts');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('source_kind',4),TRUE,'same_six_nested_parts');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('recipe','oteryn:planting.preserve'),FALSE,'other_six_recipe');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('recipe','oteryn:seasonal.dracoyle'),FALSE,'other_six_transform');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('consume_source',false,'source_count',0),FALSE,'nonconsuming_six');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('source_kind',3),FALSE,'unqualified_ground_six');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('extra_inputs',inputs||jsonb_build_array(inputs->0)),FALSE,'seven_extras');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('extra_inputs',jsonb_set(inputs,'{0,count}','0')),FALSE,'wrong_count');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('extra_inputs',jsonb_set(inputs,'{0}',(inputs->0)-'count')),FALSE,'missing_count');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('extra_inputs',jsonb_set(inputs,'{0,source_kind}','3')),FALSE,'wrong_extra_custody');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('extra_inputs',jsonb_set(inputs,'{0,before,definition_production_key}','"oteryn:item.tibia.i21906"')),FALSE,'duplicate_part');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('before_item',primary_item||jsonb_build_object('definition_production_key','oteryn:item.tibia.i2995')),FALSE,'wrong_primary');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('effects','{}'::jsonb),FALSE,'missing_effect');
 PERFORM pg_temp.srv7_glider_bound(candidate||jsonb_build_object('recipe','other','extra_inputs',inputs-5),TRUE,'general_five_unchanged');
END $$;
