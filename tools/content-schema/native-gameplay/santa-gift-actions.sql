-- Closed Santa gift leaves; all other consume/transform contracts remain unchanged.
CREATE FUNCTION game_santa_gift_record(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT r.recipe IN('oteryn:novelty-item.i2995.v1','oteryn:novelty-item.i5080.v1','oteryn:novelty-item.i6511.v1','oteryn:novelty-item.i5791.v1','oteryn:novelty-item.i6566.v1')
  AND r.before_item->>'definition_production_key'='oteryn:item.tibia.i'||(r.effects->>'source_item_id')
  AND r.recipe='oteryn:novelty-item.i'||(r.effects->>'source_item_id')||'.v1'
$$;
CREATE FUNCTION game_santa_gift_custody(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN r.source_kind=5 THEN game_item_action_house_custody(r)
 WHEN r.source_kind IN(1,4) THEN game_item_action_furniture_custody(r)
 WHEN r.source_kind=3 THEN game_item_action_custody_current(r) ELSE FALSE END
$$;
-- Actual House actor/source positions are sealed by its current Node, not supplied
-- by a later runtime receipt which could choose a convenient actor_position.
CREATE TABLE game_santa_house_source_context(
 backend_pid INTEGER PRIMARY KEY,transaction_xid XID8 NOT NULL,node_id UUID NOT NULL,registration NUMERIC NOT NULL,
 character_id UUID NOT NULL,game_session_id UUID NOT NULL,command_id NUMERIC NOT NULL,connection NUMERIC NOT NULL,lease NUMERIC NOT NULL,generation NUMERIC NOT NULL,
 world_id UUID NOT NULL,house_key TEXT NOT NULL,source_item UUID NOT NULL,actor_position JSONB NOT NULL,source_position JSONB NOT NULL);
REVOKE ALL ON game_santa_house_source_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_santa_arm_house_source(p_node UUID,p_registration NUMERIC,p_secret BYTEA,p_character UUID,p_session UUID,p_command NUMERIC,
 p_connection NUMERIC,p_lease NUMERIC,p_generation NUMERIC,p_world UUID,p_house TEXT,p_source UUID,p_actor JSONB,p_at JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
  OR p_command IS NULL OR p_command<>trunc(p_command) OR p_command NOT BETWEEN 1 AND 18446744073709551615
  OR p_source IS NULL OR p_house IS NULL OR octet_length(p_house) NOT BETWEEN 1 AND 256
  OR (jsonb_typeof(p_actor)='array' AND jsonb_array_length(p_actor)=3 AND jsonb_typeof(p_at)='array' AND jsonb_array_length(p_at)=3) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM jsonb_array_elements(p_actor||p_at) value WHERE jsonb_typeof(value)<>'number' OR value::text !~ '^-?[0-9]{1,10}$'
   OR value::text::bigint NOT BETWEEN -2147483648 AND 2147483647)
  OR (p_actor->>2)::integer IS DISTINCT FROM (p_at->>2)::integer
  OR abs((p_actor->>0)::bigint-(p_at->>0)::bigint)>1 OR abs((p_actor->>1)::bigint-(p_at->>1)::bigint)>1 THEN
  RAISE EXCEPTION 'Santa House source lacks current private Node and actual actor adjacency' USING ERRCODE='23514'; END IF;
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_generation,p_node,p_registration,p_world,p_house);
 IF EXISTS(SELECT 1 FROM game_santa_house_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Santa House source accepts one immutable native observation' USING ERRCODE='23514'; END IF;
 DELETE FROM game_santa_house_source_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_santa_house_source_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_node,p_registration,p_character,p_session,p_command,
  p_connection,p_lease,p_generation,p_world,p_house,p_source,p_actor,p_at);
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_santa_arm_house_source(uuid,numeric,bytea,uuid,uuid,numeric,numeric,numeric,numeric,uuid,text,uuid,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_santa_arm_house_source(uuid,numeric,bytea,uuid,uuid,numeric,numeric,numeric,numeric,uuid,text,uuid,jsonb,jsonb) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_santa_arm_house_source(uuid,numeric,bytea,uuid,uuid,numeric,numeric,numeric,numeric,uuid,text,uuid,jsonb,jsonb) TO oteryn_game_runtime;

-- Same closed one-tile action distance as the native House actor/placed source.
-- Custody independently compares this location with the real locked item row.
CREATE FUNCTION game_santa_gift_adjacent(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE at JSONB:=r.effects->'ground'; actor JSONB:=r.source_location->'actor_position';
BEGIN
 IF r.source_kind<>5 OR coalesce(r.source_location->>'custody_kind','house')<>'house' THEN RETURN TRUE; END IF;
 IF jsonb_typeof(at) IS DISTINCT FROM 'object' OR jsonb_typeof(actor) IS DISTINCT FROM 'array'
  OR jsonb_array_length(actor)<>3 THEN RETURN FALSE; END IF;
 RETURN ((r.source_location->'house'->>'spatial_position')::bytea =
   int4send((at->>'x')::integer)||int4send((at->>'y')::integer)||int4send((at->>'floor')::integer))
  AND (at->>'floor')::integer=(actor->>2)::integer
  AND abs((at->>'x')::bigint-(actor->>0)::bigint)<=1
  AND abs((at->>'y')::bigint-(actor->>1)::bigint)<=1;
END $$;
CREATE FUNCTION game_santa_gift_effects_valid(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT r.effects->>'kind'='consume_effect'
  AND r.effects->>'donor'='crystal-201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/'||CASE WHEN r.effects->>'source_item_id'='2995' THEN 'piggy_bank.lua:3-13' WHEN r.effects->>'source_item_id'='6511' THEN 'dolls.lua:23-30,111-149' ELSE 'dolls.lua:2,15-21,111-149' END
  AND r.effects->'private_speech'='false'::jsonb AND r.effects->'require_head'='false'::jsonb
  AND r.effects->'require_player_target'='false'::jsonb
  AND NOT (r.effects ?| ARRAY['dice_value','costume','achievement','timed_output'])
  AND CASE (CASE WHEN r.effects->>'source_item_id'='5791' THEN '6566' ELSE r.effects->>'source_item_id' END)
  WHEN '2995' THEN NOT(r.effects ? 'speech') AND NOT(r.effects ? 'damage')
   AND r.effects->'cues'=CASE r.effects->>'gift_currency'
    WHEN '3031' THEN '[{"binding":"appearance:effect/poff","effect_id":3}]'::jsonb
    WHEN '3035' THEN '[{"binding":"appearance:effect/sound_yellow","effect_id":22}]'::jsonb END
  WHEN '5080' THEN r.effects->>'speech'='Hug me!' AND NOT(r.effects ? 'damage') AND r.effects->'cues'='[]'::jsonb
  WHEN '6511' THEN r.effects->>'speech'=ANY(ARRAY['Ho ho ho','Jingle bells, jingle bells...','Have you been naughty?','Have you been nice?','Merry Christmas!','Can you stop squeezing me now... I''m starting to feel a little sick.']) AND NOT(r.effects ? 'damage') AND r.effects->'cues'='[]'::jsonb
  WHEN '6566' THEN CASE r.effects->>'speech'
   WHEN 'Fchhhhhh!' THEN NOT(r.effects ? 'damage') AND r.effects->'cues'='[]'::jsonb
   WHEN 'Zchhhhhh!' THEN NOT(r.effects ? 'damage') AND r.effects->'cues'='[]'::jsonb
   WHEN 'Groaaarrr.... *cough*' THEN NOT(r.effects ? 'damage') AND r.effects->'cues'='[{"binding":"appearance:effect/poff","effect_id":3}]'::jsonb
   WHEN 'Aaa... CHOO!' THEN NOT(r.effects ? 'damage') AND r.effects->'cues'='[{"binding":"appearance:effect/firearea","effect_id":7}]'::jsonb
   WHEN 'You... will.... burn!!' THEN r.effects->'damage'='1'::jsonb AND r.effects->'cues'='[{"binding":"appearance:effect/explosionhit","effect_id":6}]'::jsonb
   ELSE FALSE END
  ELSE FALSE END
$$;
-- Same native SHA256 domain and unsigned network-order fields. Resource bound
-- fails closed instead of accepting a different draw if sixteen rejections occur.
CREATE FUNCTION game_santa_gift_draw(p_session UUID,p_command NUMERIC,p_source UUID,p_domain INTEGER,p_upper INTEGER)
RETURNS INTEGER LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE command_bytes BYTEA; digest BYTEA; value NUMERIC; zone NUMERIC; counter INTEGER; n INTEGER;
BEGIN
 IF p_command<>trunc(p_command) OR p_command NOT BETWEEN 1 AND 18446744073709551615 OR (p_domain,p_upper) NOT IN((210,6),(211,5),(212,6)) THEN
  RAISE EXCEPTION 'invalid closed Santa RNG domain' USING ERRCODE='23514'; END IF;
 command_bytes:=int8send((CASE WHEN p_command>9223372036854775807 THEN p_command-18446744073709551616 ELSE p_command END)::bigint);
 zone:=18446744073709551615-mod(18446744073709551615,p_upper);
 FOR counter IN 0..15 LOOP
  digest:=sha256(convert_to('oteryn:reward-bag-draw:v1','UTF8')||uuid_send(p_session)||command_bytes
   ||uuid_send(p_source)||set_byte(decode('00','hex'),0,p_domain)||int4send(counter));
  value:=0;
  FOR n IN 0..7 LOOP value:=value*256+get_byte(digest,n); END LOOP;
  IF value<zone THEN RETURN mod(value,p_upper)::integer; END IF;
 END LOOP;
 RAISE EXCEPTION 'Santa RNG rejection resource bound' USING ERRCODE='23514';
END $$;
CREATE FUNCTION game_santa_gift_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; o JSONB; parent UUID; node UUID; node_revision NUMERIC;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF game_santa_gift_record(r) IS DISTINCT FROM TRUE OR i.item_instance_id IS NULL
  OR to_jsonb(i) IS DISTINCT FROM r.before_item OR r.before_secondary IS DISTINCT FROM r.before_item
  OR r.secondary_item_instance_id IS DISTINCT FROM r.source_item_instance_id OR r.source_kind NOT IN(1,3,4,5)
  OR r.consume_source OR r.source_count<>0 OR r.extra_inputs<>'[]'::jsonb
  OR i.lifecycle<>1 OR i.quantity<>1 OR i.world_id<>r.world_id OR i.definition_family<>'Item'
  OR game_santa_gift_adjacent(r) IS DISTINCT FROM TRUE
  OR game_santa_gift_effects_valid(r) IS DISTINCT FROM TRUE OR r.content_digest IS NULL
  OR game_santa_gift_custody(r) IS DISTINCT FROM TRUE
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'Santa gift requires exact unchanged plain source and custody' USING ERRCODE='23514'; END IF;
 node:=(r.effects->'gift_node'->>'id')::uuid;
 node_revision:=(r.effects->'gift_node'->>'revision')::numeric;
 IF node IS NULL OR node_revision IS NULL OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR game_node_lock_current_registration(node,node_revision) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Santa gift requires current Node registration' USING ERRCODE='23514'; END IF;
 IF r.source_kind=5 THEN
  IF node IS DISTINCT FROM (r.source_location->>'node')::uuid
   OR node_revision IS DISTINCT FROM (r.source_location->>'node_revision')::numeric THEN
   RAISE EXCEPTION 'Santa gift House Node mismatch' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM game_santa_house_source_context c WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
   AND (c.node_id,c.registration,c.character_id,c.game_session_id,c.world_id,c.house_key,c.source_item,c.actor_position,c.source_position)
   IS NOT DISTINCT FROM(node,node_revision,r.character_id,r.game_session_id,r.world_id,r.source_location->'house'->>'house_key',r.source_item_instance_id,
    r.source_location->'actor_position',jsonb_build_array((r.effects->'ground'->>'x')::integer,(r.effects->'ground'->>'y')::integer,(r.effects->'ground'->>'floor')::integer))
   AND c.command_id=r.command_id AND c.connection=(r.audit_context->>'connection_generation')::numeric AND c.lease=(r.audit_context->>'lease_generation')::numeric
   AND c.generation=(r.audit_context->>'scope_generation')::numeric) THEN
   RAISE EXCEPTION 'Santa House receipt differs from private native source observation' USING ERRCODE='23514'; END IF;
  PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
   (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
   (r.audit_context->>'scope_generation')::numeric,node,node_revision,r.world_id,r.source_location->'house'->>'house_key');
 ELSE
  IF node IS NULL OR node_revision IS NULL
   OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
   OR game_node_lock_current_registration(node,node_revision) IS DISTINCT FROM TRUE
   OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
    JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id AND c.account_id=s.account_id
    JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
    JOIN game_runtime_scope_assignments runtime ON runtime.scope_kind=1 AND runtime.world_id=r.world_id
     AND runtime.channel_id=s.runtime_scope_channel_id
    JOIN game_durability_admission_runtime_guards g ON g.scope_key=runtime.scope_key
    WHERE s.game_session_id=r.game_session_id AND s.character_id=r.character_id AND s.world_id=r.world_id
     AND s.session_state IN(1,2) AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=r.world_id
     AND s.current_generation=(r.audit_context->>'connection_generation')::numeric
     AND s.character_lease_generation=(r.audit_context->>'lease_generation')::numeric
     AND s.scope_ownership_generation=(r.audit_context->>'scope_generation')::numeric
     AND c.eligible AND c.lease_generation=s.character_lease_generation AND c.holder_game_session_id=s.game_session_id
     AND a.presence_character_id=c.character_id AND a.holder_game_session_id=s.game_session_id
     AND runtime.state=1 AND runtime.ownership_generation=s.scope_ownership_generation
     AND runtime.holder_node_id=node AND runtime.holder_registration_revision=node_revision
     AND (r.source_kind<>3 OR EXISTS(SELECT 1 FROM game_item_ground_locations ground
      WHERE ground.item_instance_id=r.source_item_instance_id AND ground.channel_id=s.runtime_scope_channel_id
       AND ground.world_id=r.world_id AND ground.runtime_scope_ownership_generation=s.scope_ownership_generation))
     AND g.ready AND g.ownership_generation=runtime.ownership_generation)
  THEN RAISE EXCEPTION 'Santa gift requires current Character and Node' USING ERRCODE='23514'; END IF;
 END IF;
 IF r.effects->>'source_item_id'='2995' THEN
  o:=r.outputs->0; parent:=(o->>'gift_parent')::uuid;
  IF r.effects->>'gift_currency' IS DISTINCT FROM (CASE WHEN game_santa_gift_draw(r.game_session_id,r.command_id,r.source_item_instance_id,210,6)=0 THEN '3031' ELSE '3035' END)
   OR (r.effects ? 'gift_after' AND i.state_revision>=18446744073709551615) THEN
   RAISE EXCEPTION 'Santa bank requires exact native random branch and transform revision' USING ERRCODE='23514'; END IF;
  IF jsonb_array_length(r.outputs)<>1 OR (o->>'count')::integer<>1
   OR o->>'key' IS DISTINCT FROM ('oteryn:item.tibia.i'||(r.effects->>'gift_currency'))
   OR o->>'revision' IS DISTINCT FROM i.definition_revision_ref
   OR o->'decay' IS DISTINCT FROM 'null'::jsonb OR NOT game_character_is_uuid_v7((o->>'item')::uuid)
   OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
   OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s JOIN game_item_instances b
     ON b.item_instance_id=s.item_instance_id AND b.world_id=s.world_id
     WHERE s.character_id=r.character_id AND s.world_id=r.world_id AND s.item_instance_id=parent AND b.lifecycle=1)
   OR r.parent_capacity NOT BETWEEN 1 AND 500
   OR (SELECT count(*) FROM (SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=parent UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=parent) entries)>=r.parent_capacity
   OR (o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM (
     SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=parent UNION ALL
     SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=parent) entries)
   OR (r.effects->>'gift_currency'='3031' AND
      (r.effects->'gift_after' IS DISTINCT FROM jsonb_build_object('key','oteryn:item.tibia.i2996','revision',i.definition_revision_ref)
       OR r.effects->'achievement_progress' IS DISTINCT FROM '[{"key":"oteryn:achievement/allowance_collector","target":50}]'::jsonb))
   OR (r.effects->>'gift_currency'='3035' AND (r.effects ? 'gift_after' OR r.effects->'achievement_progress' IS DISTINCT FROM '[]'::jsonb))
   OR r.effects->>'gift_currency' IS NULL OR r.effects->>'gift_currency' NOT IN('3031','3035')
  THEN RAISE EXCEPTION 'Santa bank delivers exactly one native coin into current backpack' USING ERRCODE='23514'; END IF;
 ELSE
  IF r.effects->>'source_item_id'='6511' AND r.effects->>'speech' IS DISTINCT FROM
   (ARRAY['Ho ho ho','Jingle bells, jingle bells...','Have you been naughty?','Have you been nice?','Merry Christmas!','Can you stop squeezing me now... I''m starting to feel a little sick.'])[1+game_santa_gift_draw(r.game_session_id,r.command_id,r.source_item_instance_id,212,6)] THEN
   RAISE EXCEPTION 'Santa doll requires exact native random speech branch' USING ERRCODE='23514'; END IF;
  IF r.effects->>'source_item_id' IN('5791','6566') AND r.effects->>'speech' IS DISTINCT FROM
   (ARRAY['Fchhhhhh!','Zchhhhhh!','Groaaarrr.... *cough*','Aaa... CHOO!','You... will.... burn!!'])[1+game_santa_gift_draw(r.game_session_id,r.command_id,r.source_item_instance_id,211,5)] THEN
   RAISE EXCEPTION 'Santa dragon requires exact native random speech branch' USING ERRCODE='23514'; END IF;
  IF r.outputs<>'[]'::jsonb OR r.effects ? 'gift_after' OR r.effects ? 'gift_currency'
   OR r.effects->'achievement_progress' IS DISTINCT FROM '[]'::jsonb THEN
   RAISE EXCEPTION 'Santa doll cannot mint or transform' USING ERRCODE='23514'; END IF;
 END IF;
END $$;
CREATE FUNCTION game_santa_gift_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; i game_item_instances%ROWTYPE; o JSONB; after_key TEXT;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 after_key:=r.effects->'gift_after'->>'key';
 IF r.transaction_id IS NULL OR game_santa_gift_record(r) IS DISTINCT FROM TRUE
  OR r.created_xact_id<>pg_current_xact_id() OR game_santa_gift_custody(r) IS DISTINCT FROM TRUE
  OR (after_key IS NULL AND to_jsonb(i) IS DISTINCT FROM r.before_item)
  OR (after_key IS NOT NULL AND (i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR i.state_revision IS DISTINCT FROM (r.before_item->>'state_revision')::numeric+1
    OR i.definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i2996'
    OR (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
      IS DISTINCT FROM (r.before_item-ARRAY['definition_production_key','last_transaction_id','state_revision'])))
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN after_key IS NULL THEN 0 ELSE 1 END)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
   AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
   AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
   AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'Santa gift changed source, custody or exact audit' USING ERRCODE='23514'; END IF;
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF (SELECT count(*) FROM (SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=(o->>'gift_parent')::uuid UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=(o->>'gift_parent')::uuid) entries)>r.parent_capacity
   OR NOT EXISTS(SELECT 1 FROM game_item_instances coin JOIN game_item_container_entries e USING(item_instance_id)
    JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id AND s.character_id=e.character_id AND s.world_id=e.world_id
    WHERE coin.item_instance_id=(o->>'item')::uuid AND coin.minted_transaction_id=r.transaction_id
     AND coin.world_id=r.world_id AND coin.definition_family='Item' AND coin.definition_production_key=o->>'key'
     AND coin.definition_revision_ref=o->>'revision' AND coin.quantity=1 AND coin.lifecycle=1
     AND coin.last_transaction_id IS NULL AND coin.state_revision=1
     AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=(o->>'gift_parent')::uuid
     AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id)
   OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=(o->>'item')::uuid)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=(o->>'item')::uuid)
   OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=(o->>'item')::uuid)
  THEN RAISE EXCEPTION 'Santa currency requires exact fresh actor backpack custody' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_santa_gift_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Santa gift requires original receipt transaction' USING ERRCODE='23514'; END IF;
 PERFORM game_santa_gift_admit(r);
 IF r.effects ? 'gift_after' THEN
  UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i2996',state_revision=state_revision+1,
   last_transaction_id=r.transaction_id WHERE item_instance_id=r.source_item_instance_id;
 END IF;
END $$;
-- Preserve every existing constraint arm, adding only the closed bank mint shape.
DO $migration$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_no_consume_shape';
 IF body IS NULL OR left(body,7)<>'CHECK (' THEN RAISE EXCEPTION 'Santa gift no-consume contract missing'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_no_consume_shape;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_no_consume_shape CHECK ('
  ||substring(body FROM 8 FOR length(body)-8)
  ||' OR (recipe=''oteryn:novelty-item.i2995.v1'' AND NOT consume_source AND source_count=0 AND jsonb_array_length(outputs)=1 AND extra_inputs=''[]''::jsonb AND secondary_item_instance_id=source_item_instance_id))';
END $migration$;
DO $migration$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF body IS NULL OR left(body,7)<>'CHECK (' THEN RAISE EXCEPTION 'Santa gift source contract missing'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
  ||substring(body FROM 8 FOR length(body)-8)
  ||' OR (source_kind IN(3,5) AND parent_item_instance_id IS NULL AND NOT consume_source AND source_count=0'
  ||' AND extra_inputs=''[]''::jsonb AND secondary_item_instance_id=source_item_instance_id'
  ||' AND recipe IN(''oteryn:novelty-item.i2995.v1'',''oteryn:novelty-item.i5080.v1'',''oteryn:novelty-item.i6511.v1'',''oteryn:novelty-item.i5791.v1'',''oteryn:novelty-item.i6566.v1'')))';
END $migration$;
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_consume_transform_admit','game_item_consume_transform_complete',
  'game_item_instance_guard','game_item_instance_change_proven','game_item_placement_proven'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_consume_transform_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing Santa gift proof chain: %',n; END IF;
  IF n='game_item_consume_transform_admit' THEN arm:=$arm$
   IF NEW.recipe IN('oteryn:novelty-item.i2995.v1','oteryn:novelty-item.i5080.v1','oteryn:novelty-item.i6511.v1','oteryn:novelty-item.i5791.v1','oteryn:novelty-item.i6566.v1') THEN PERFORM game_santa_gift_admit(NEW); NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END IF;
  $arm$;
  ELSIF n='game_item_consume_transform_complete' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=p_transaction AND recipe IN('oteryn:novelty-item.i2995.v1','oteryn:novelty-item.i5080.v1','oteryn:novelty-item.i6511.v1','oteryn:novelty-item.i5791.v1','oteryn:novelty-item.i6566.v1')) THEN
    PERFORM game_santa_gift_complete(p_transaction); RETURN;
   END IF;
  $arm$;
  ELSIF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE game_santa_gift_record(r)
    AND r.source_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
    AND r.created_xact_id=pg_current_xact_id() AND r.transaction_id=NEW.last_transaction_id
    AND r.effects->'gift_after'->>'key'='oteryn:item.tibia.i2996') THEN
    IF NEW.definition_production_key<>'oteryn:item.tibia.i2996' OR NEW.state_revision<>OLD.state_revision+1
     OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision']) THEN
     RAISE EXCEPTION 'Santa piggy transforms the same exact bank instance' USING ERRCODE='23514'; END IF;
    RETURN NEW;
   END IF;
  $arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE game_santa_gift_record(r)
    AND r.source_item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
    AND r.created_xact_id=pg_current_xact_id()) THEN PERFORM game_santa_gift_complete(NEW.last_transaction_id); RETURN NULL; END IF;
  $arm$;
  ELSE arm:=$arm$
   IF TG_TABLE_NAME='game_item_container_entries' THEN
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,
    LATERAL jsonb_array_elements(r.outputs) o WHERE game_santa_gift_record(r)
    AND (o->>'item')::uuid=NEW.item_instance_id AND (o->>'gift_parent')::uuid=NEW.parent_item_instance_id
    AND (o->>'ordinal')::numeric=NEW.placement_ordinal AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
    AND r.transaction_id=NEW.placed_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
   END IF;
  $arm$;
  END IF;
  EXECUTE substring(body FROM 1 FOR begin_at+4)||E'\n'||arm||substring(body FROM begin_at+5);
 END LOOP;
END $migration$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY['game_santa_gift_record(game_item_consume_transform_records)'::regprocedure,
  'game_santa_gift_custody(game_item_consume_transform_records)'::regprocedure,
  'game_santa_gift_adjacent(game_item_consume_transform_records)'::regprocedure,
  'game_santa_gift_effects_valid(game_item_consume_transform_records)'::regprocedure,
  'game_santa_gift_draw(uuid,numeric,uuid,integer,integer)'::regprocedure,
  'game_santa_gift_admit(game_item_consume_transform_records)'::regprocedure,
  'game_santa_gift_complete(uuid)'::regprocedure,'game_santa_gift_apply(uuid)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;
