-- Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:20-32.
-- An exact retained tool and a current native corpse can drive the existing quest receipt.
-- Crystal 201189f8 actions_mission03_the_keeper.lua:17-21 uses a corpse
-- without a carried tool or an additional age constraint.
ALTER TABLE game_quest_corpse_samples
 ALTER COLUMN source_item DROP NOT NULL,
 ALTER COLUMN maximum_age_ms DROP NOT NULL,
 ADD CONSTRAINT game_quest_corpse_sample_age_requires_tool
 CHECK(maximum_age_ms IS NULL OR source_item IS NOT NULL);

CREATE OR REPLACE FUNCTION game_quest_corpse_sample_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source_row RECORD; sample_row RECORD; input_row RECORD; ingress_json JSONB; proof JSONB;
BEGIN
 SELECT * INTO source_row FROM game_quest_reward_sources
 WHERE (character_id,cause_id,cause_ordinal,transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
 IF NOT FOUND THEN RAISE EXCEPTION 'corpse sample lacks quest source' USING ERRCODE='23514'; END IF;
 IF NOT (source_row.plan ? 'corpse_target') THEN
  IF TG_TABLE_NAME='game_quest_corpse_samples' THEN RAISE EXCEPTION 'corpse sample lacks authored predicate' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 SELECT * INTO sample_row FROM game_quest_corpse_samples
 WHERE (character_id,cause_id,cause_ordinal,transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
 IF NOT FOUND OR sample_row.created_xact_id<>pg_current_xact_id() OR source_row.created_xact_id<>pg_current_xact_id()
  OR source_row.plan #>> '{corpse_target,appearance}' IS DISTINCT FROM sample_row.appearance::TEXT
  OR source_row.plan #>> '{corpse_target,maximum_age_ms}' IS DISTINCT FROM sample_row.maximum_age_ms::TEXT
  OR source_row.plan->'capture_creature'='true'::JSONB
 THEN RAISE EXCEPTION 'corpse sample predicate differs from quest source' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i
  JOIN game_item_ground_locations g USING(item_instance_id,world_id)
  JOIN game_corpse_source_races p USING(item_instance_id)
  JOIN game_item_mint_receipts r ON r.transaction_id=p.mint_transaction_id AND r.item_instance_id=i.item_instance_id
  WHERE i.item_instance_id=sample_row.corpse_item AND i.world_id=sample_row.world_id
   AND g.channel_id=sample_row.channel_id AND g.runtime_scope_ownership_generation=sample_row.scope_generation
   AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
   AND i.definition_production_key='oteryn:item.tibia.i'||sample_row.appearance::TEXT
   AND i.minted_transaction_id=p.mint_transaction_id
   AND (p.race_key=source_row.plan->>'creature_target'
    OR coalesce(source_row.plan #> '{corpse_target,additional_creatures}','[]'::jsonb) ? p.race_key)
   AND r.loot_purpose_key='CORPSE_MATERIALIZATION'
   AND (sample_row.maximum_age_ms IS NULL OR floor(extract(epoch FROM transaction_timestamp())*1000)::BIGINT-r.occurred_at BETWEEN 0 AND sample_row.maximum_age_ms-1)
   AND to_jsonb(i)=sample_row.before_item AND to_jsonb(g)=sample_row.before_ground
   AND game_quest_corpse_adjacent(g.spatial_position,sample_row.actor_position))
 THEN RAISE EXCEPTION 'corpse sample lacks unchanged fresh native corpse and adjacency' USING ERRCODE='23514'; END IF;
 IF sample_row.source_item IS NULL THEN
  IF source_row.plan ? 'carried_item'
   OR source_row.plan->'take' IS DISTINCT FROM '[]'::jsonb
   OR coalesce(source_row.plan->'give','null'::jsonb)='null'::jsonb
  THEN RAISE EXCEPTION 'direct corpse sample has incompatible quest source' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 IF source_row.plan #> '{corpse_target,retain_tool}' ='true'::jsonb THEN
  IF source_row.plan->'take' IS DISTINCT FROM '[]'::jsonb
   OR NOT EXISTS(
    SELECT 1 FROM game_quest_callback_carried(sample_row.character_id,sample_row.world_id) carried
     JOIN game_item_instances i ON i.item_instance_id=carried.item
     WHERE i.item_instance_id=sample_row.source_item AND i.lifecycle=1 AND i.quantity>=1
      AND i.definition_family='Item' AND i.definition_production_key=source_row.plan->>'carried_item'
   )
  THEN RAISE EXCEPTION 'corpse sample lacks retained selected tool' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 IF sample_row.maximum_age_ms IS NULL THEN
  RAISE EXCEPTION 'consumed corpse tool requires a freshness window' USING ERRCODE='23514';
 END IF;
 SELECT * INTO input_row FROM game_quest_callback_consumptions c
 WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
  AND c.item_instance_id=sample_row.source_item AND c.quantity=1 AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR input_row.before_item->>'world_id' IS DISTINCT FROM sample_row.world_id::TEXT
  OR input_row.before_item->>'definition_production_key' IS DISTINCT FROM source_row.plan->>'carried_item'
 THEN RAISE EXCEPTION 'corpse sample lacks exact carried tool' USING ERRCODE='23514'; END IF;
 proof:=convert_from(input_row.proof,'UTF8')::JSONB;
 SELECT jsonb_agg(get_byte(sample_row.ingress,n) ORDER BY n) INTO ingress_json
 FROM generate_series(0,octet_length(sample_row.ingress)-1) bytes(n);
 IF proof->'reward_callback' IS DISTINCT FROM source_row.plan OR proof #> '{carried_use,intent}' IS DISTINCT FROM ingress_json
 THEN RAISE EXCEPTION 'corpse sample ingress differs from tool receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_corpse_sample_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

-- Crystal 201189f8 movements_herald_red_floors.lua:42–56. The Ground owner
-- burns only current, field-minted items inside the authored StepIn sweep.
CREATE FUNCTION game_quest_dynamic_field_placement(i game_item_instances,g game_item_ground_locations)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT coalesce(i.item_instance_id=g.item_instance_id AND i.world_id=g.world_id
 AND EXISTS(
  SELECT 1 FROM game_spell_item_lines l
  WHERE l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id
   AND l.operation_kind=1 AND l.world_id=g.world_id AND l.channel_id=g.channel_id
   AND l.spatial_position=g.spatial_position AND l.map_revision=g.map_revision
   AND l.content_revision=g.content_revision AND l.placement_context=g.native_room_placement_context
   AND (l.expires_at_unix_ms IS NOT NULL OR EXISTS(
    SELECT 1 FROM game_spell_field_temporal_schedules t
    WHERE t.item_instance_id=i.item_instance_id AND t.source_transaction_id=l.transaction_id))
 ),false)
$$;

CREATE FUNCTION game_quest_field_sweep_matches(
 r game_quest_callback_consumptions,i game_item_instances,g game_item_ground_locations)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
DECLARE p JSONB; area JSONB;
BEGIN
 IF r.location_kind<>'ground' OR r.cause_ordinal<1 OR r.quantity<>1 OR r.replacement IS NOT NULL
  OR NOT game_quest_dynamic_field_placement(i,g)
 THEN RETURN false; END IF;
 p:=convert_from(r.proof,'UTF8')::jsonb;
 IF p->>'schema' IS DISTINCT FROM 'OTERYN_QUEST_FIELD_SWEEP/v1' THEN RETURN false; END IF;
 area:=p#>'{callback,effects,sweep_fields}';
 RETURN coalesce(
  p#>>'{callback,key}'=r.transition_key AND p#>>'{callback,trigger}'='step_in'
  AND i.item_instance_id=r.item_instance_id AND i.lifecycle=1 AND i.quantity=1
  AND i.definition_family='Item' AND to_jsonb(i)=r.before_item AND to_jsonb(g)=r.before_ground
  AND EXISTS(SELECT 1 FROM jsonb_array_elements_text(area->'appearances') appearance(id)
   WHERE i.definition_production_key='oteryn:item.tibia.i'||appearance.id)
  AND octet_length(g.spatial_position)=10
  AND substring(g.spatial_position from 9 for 2)=int2send((-(area#>>'{minimum,floor}')::integer)::smallint)
  AND area#>>'{maximum,floor}'=area#>>'{minimum,floor}'
  AND substring(g.spatial_position from 1 for 4)
    BETWEEN int4send((area#>>'{minimum,x}')::integer) AND int4send((area#>>'{maximum,x}')::integer)
  AND substring(g.spatial_position from 5 for 4)
    BETWEEN int4send((area#>>'{minimum,y}')::integer) AND int4send((area#>>'{maximum,y}')::integer)
  AND g.map_revision=p#>>'{context,map_revision}' AND g.content_revision=p#>>'{context,content_revision}'
  AND EXISTS(
   SELECT 1 FROM game_durability_reconnect_sessions s
    JOIN game_durability_admission_character_guards a ON a.character_id=s.character_id
   WHERE s.game_session_id=r.cause_id AND s.character_id=r.character_id AND s.session_state=2
    AND a.eligible AND a.holder_game_session_id=s.game_session_id
    AND a.lease_generation=s.character_lease_generation
    AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_channel_id=g.channel_id
    AND s.scope_ownership_generation=g.runtime_scope_ownership_generation
  ),false);
END $$;

DO $migration$
DECLARE body TEXT; needle TEXT:='game_remorse_source_step_input_matches(NEW,i,g)';
BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_dynamic_field_placement(game_item_instances,game_item_ground_locations) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_quest_field_sweep_matches(game_quest_callback_consumptions,game_item_instances,game_item_ground_locations) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 body:=pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'unexpected current quest Ground admit predicate';
 END IF;
 EXECUTE replace(body,needle,'('||needle||' OR game_quest_field_sweep_matches(NEW,i,g))');
END $migration$;
