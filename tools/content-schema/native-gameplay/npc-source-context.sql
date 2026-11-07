-- Draft: finite Source-program context. Actual program/policy rows are generated
-- from the reviewed source input; runtime cannot insert or edit those rows.
CREATE TABLE game_npc_source_programs (
 program_sha256 BYTEA PRIMARY KEY CHECK(octet_length(program_sha256)=32),
 source_bytes BYTEA NOT NULL CHECK(octet_length(source_bytes)<=2097152),
 CHECK(sha256(source_bytes)=program_sha256)
);
CREATE TABLE game_npc_source_policies (
 program_sha256 BYTEA NOT NULL REFERENCES game_npc_source_programs(program_sha256),
 source_key TEXT NOT NULL CHECK(octet_length(source_key) BETWEEN 1 AND 256),
 npc_key TEXT NOT NULL CHECK(octet_length(npc_key) BETWEEN 1 AND 128),
 track_keys TEXT[] NOT NULL CHECK(cardinality(track_keys)<=512),
 writable_tracks JSONB NOT NULL CHECK(jsonb_typeof(writable_tracks)='object'),
 allowed_grant_keys TEXT[] NOT NULL CHECK(cardinality(allowed_grant_keys)<=2),
 item_costs JSONB NOT NULL CHECK(jsonb_typeof(item_costs)='array' AND jsonb_array_length(item_costs)<=32),
 effects JSONB NOT NULL CHECK(jsonb_typeof(effects)='array' AND jsonb_array_length(effects)<=64),
 fee_service_key TEXT, fee_gold_units BIGINT CHECK(fee_gold_units>0),
 enabled BOOLEAN NOT NULL DEFAULT FALSE,
 PRIMARY KEY(program_sha256,source_key),
 CHECK((fee_service_key IS NULL)=(fee_gold_units IS NULL))
);
REVOKE ALL ON game_npc_source_programs,game_npc_source_policies FROM PUBLIC;
GRANT SELECT ON game_npc_source_programs,game_npc_source_policies TO oteryn_game_runtime;
CREATE TABLE game_npc_source_context (
 backend_pid INTEGER PRIMARY KEY, transaction_xid XID8 NOT NULL,
 node_id UUID NOT NULL,registration NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL,account_id UUID NOT NULL,world_id UUID NOT NULL,channel_id UUID NOT NULL,
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 connection_generation NUMERIC(20,0) NOT NULL,lease_generation NUMERIC(20,0) NOT NULL,scope_generation NUMERIC(20,0) NOT NULL,
 original_revision NUMERIC(20,0) NOT NULL,
 program_sha256 BYTEA NOT NULL,source_key TEXT NOT NULL,npc_key TEXT NOT NULL,
 npc_transaction_id UUID NOT NULL,request_binding BYTEA NOT NULL,
 before_state JSONB NOT NULL,expected_plan JSONB NOT NULL,plan_sha256 BYTEA NOT NULL,
 cost_transaction_id UUID,fee_transaction_id UUID,captured BOOLEAN NOT NULL DEFAULT FALSE,
 FOREIGN KEY(program_sha256,source_key) REFERENCES game_npc_source_policies(program_sha256,source_key)
);
CREATE INDEX game_npc_source_context_program ON game_npc_source_context(program_sha256,source_key);
REVOKE ALL ON game_npc_source_context FROM PUBLIC,oteryn_game_runtime;
ALTER TABLE game_npc_cosmetic_receipts
 ADD COLUMN program_sha256 BYTEA CHECK(program_sha256 IS NULL OR octet_length(program_sha256)=32),
 ADD COLUMN plan_sha256 BYTEA CHECK(plan_sha256 IS NULL OR octet_length(plan_sha256)=32),
 ADD COLUMN fee_transaction_id UUID UNIQUE;
-- Immutable cost records capture the actual equipment epoch before worn materials
-- are removed. The existing proof basis already hashes the whole stamped row.
ALTER TABLE game_srv7_item_costs ADD COLUMN equipment_before JSONB;
DO $migration$ DECLARE body TEXT;needle TEXT;BEGIN
 body:=pg_get_functiondef('game_srv7_cost_admit()'::regprocedure);
 needle:=' NEW.created_xact_id:=pg_current_xact_id();';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'NPC full equipment capture boundary changed';END IF;
 EXECUTE replace(body,needle,$capture$
 IF NEW.equipment_revision_before IS NOT NULL THEN
  SELECT to_jsonb(npc_equipment_before) INTO STRICT NEW.equipment_before
   FROM game_character_equipment_state npc_equipment_before WHERE npc_equipment_before.character_id=NEW.character_id FOR UPDATE;
  IF (NEW.equipment_before->>'revision')::numeric IS DISTINCT FROM NEW.equipment_revision_before THEN
   RAISE EXCEPTION 'NPC material equipment capture differs from actual epoch' USING ERRCODE='23514';END IF;
 ELSE NEW.equipment_before:=NULL;END IF;
 NEW.created_xact_id:=pg_current_xact_id();$capture$);
END $migration$;

-- No source language executes: only these four closed integer expressions.
CREATE FUNCTION game_npc_source_expression(p_expr JSONB,p_tracks JSONB,p_now BIGINT) RETURNS BIGINT
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE result BIGINT;BEGIN
 CASE p_expr->>'op'
 WHEN 'literal' THEN result:=(p_expr->>'value')::bigint;
 WHEN 'now' THEN result:=p_now;
 WHEN 'now_plus' THEN result:=p_now+(p_expr->>'seconds')::bigint;
 WHEN 'add' THEN
  result:=coalesce((p_tracks->>(p_expr->>'storage'))::bigint,-1)+(p_expr->>'amount')::bigint;
  IF p_expr ? 'clamp_min' THEN result:=greatest(result,(p_expr->>'clamp_min')::bigint);END IF;
  IF p_expr ? 'clamp_max' THEN result:=least(result,(p_expr->>'clamp_max')::bigint);END IF;
 ELSE RAISE EXCEPTION 'unknown finite NPC integer expression' USING ERRCODE='23514';
 END CASE;
 IF result IS NULL THEN RAISE EXCEPTION 'incomplete finite NPC integer expression' USING ERRCODE='23514';END IF;
 RETURN result;
END $$;

CREATE TABLE game_npc_source_clock (
 backend_pid INTEGER PRIMARY KEY, transaction_xid XID8 NOT NULL,node_id UUID NOT NULL, registration NUMERIC(20,0) NOT NULL,now_seconds BIGINT NOT NULL
);
REVOKE ALL ON game_npc_source_clock FROM PUBLIC,oteryn_game_runtime;
CREATE FUNCTION game_npc_source_observe_clock(p_node UUID,p_registration NUMERIC,p_secret BYTEA) RETURNS BIGINT
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE observed BIGINT;BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'NPC source clock lacks current Node possession' USING ERRCODE='23514';END IF;
 SELECT now_seconds INTO observed FROM game_npc_source_clock WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() AND node_id=p_node AND registration=p_registration;
 IF observed IS NOT NULL THEN RETURN observed;END IF;
 IF EXISTS(SELECT 1 FROM game_npc_source_clock WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'NPC source clock already belongs to another Node' USING ERRCODE='23514';END IF;
 DELETE FROM game_npc_source_clock WHERE backend_pid=pg_backend_pid() OR backend_pid NOT IN(SELECT pid FROM pg_stat_activity);
 observed:=floor(extract(epoch FROM game_daily_effect_now()))::bigint;
 INSERT INTO game_npc_source_clock VALUES(pg_backend_pid(),pg_current_xact_id(),p_node,p_registration,observed);
 RETURN observed;
END $$;
CREATE FUNCTION game_npc_arm_source(p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_context JSONB,p_program BYTEA,p_before JSONB,p_plan JSONB) RETURNS BYTEA
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE policy game_npc_source_policies%ROWTYPE;s game_durability_reconnect_sessions%ROWTYPE;
 root game_character_roots%ROWTYPE; actual_tracks JSONB;actor_sex TEXT;change JSONB;rule JSONB;change_count INTEGER;
BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
  OR game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE
  OR jsonb_typeof(p_context) IS DISTINCT FROM 'object' OR jsonb_typeof(p_before) IS DISTINCT FROM 'object'
  OR jsonb_typeof(p_plan) IS DISTINCT FROM 'object' OR octet_length(p_before::text)>196608 OR octet_length(p_plan::text)>196608 THEN
  RAISE EXCEPTION 'NPC source arm lacks current Node/closed native observation' USING ERRCODE='23514';END IF;
 SELECT * INTO policy FROM game_npc_source_policies WHERE program_sha256=p_program AND source_key=p_context->>'source_key';
 IF policy.source_key IS NULL OR NOT policy.enabled OR policy.npc_key IS DISTINCT FROM p_context->>'npc_key' THEN
  RAISE EXCEPTION 'NPC source is not in the active finite program' USING ERRCODE='23514';END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions WHERE game_session_id=(p_context->>'game_session_id')::uuid FOR UPDATE;
 SELECT * INTO root FROM game_character_roots WHERE character_id=s.character_id FOR UPDATE;
 IF s.game_session_id IS NULL OR root.character_id IS NULL OR s.session_state<>2 OR s.current_transport_ref IS NULL OR root.lifecycle<>1
  OR s.character_id IS DISTINCT FROM (p_context->>'character_id')::uuid OR s.account_id IS DISTINCT FROM root.account_id
  OR s.world_id IS DISTINCT FROM (p_context->>'world_id')::uuid OR s.runtime_scope_kind<>1
  OR s.runtime_scope_world_id IS DISTINCT FROM s.world_id OR s.runtime_scope_channel_id IS DISTINCT FROM (p_context->>'channel_id')::uuid
  OR s.current_generation IS DISTINCT FROM (p_context->>'connection_generation')::numeric
  OR s.character_lease_generation IS DISTINCT FROM (p_context->>'lease_generation')::numeric
  OR s.scope_ownership_generation IS DISTINCT FROM (p_context->>'scope_generation')::numeric
  OR root.character_revision IS DISTINCT FROM (p_context->>'original_revision')::numeric
  OR root.character_revision>=18446744073709551615
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments assignment
   JOIN game_durability_admission_runtime_guards runtime_guard ON runtime_guard.scope_key=assignment.scope_key
   JOIN game_durability_admission_character_guards character_guard ON character_guard.character_id=s.character_id AND character_guard.account_id=s.account_id
   JOIN game_durability_admission_account_guards account_guard ON account_guard.account_id=s.account_id
   WHERE assignment.scope_kind=1 AND assignment.world_id=s.world_id AND assignment.channel_id=s.runtime_scope_channel_id
    AND assignment.state=1 AND assignment.ownership_generation=s.scope_ownership_generation
    AND assignment.holder_node_id=p_node AND assignment.holder_registration_revision=p_registration
    AND runtime_guard.ready AND runtime_guard.ownership_generation=assignment.ownership_generation
    AND character_guard.eligible AND character_guard.holder_game_session_id=s.game_session_id AND character_guard.lease_generation=s.character_lease_generation
    AND account_guard.presence_character_id=s.character_id AND account_guard.holder_game_session_id=s.game_session_id)
  OR EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=s.character_id)
  OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=s.character_id) THEN
  RAISE EXCEPTION 'NPC source observation lost its exact current Character/Channel fence' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments assignment WHERE assignment.scope_kind=1 AND assignment.world_id=s.world_id
  AND assignment.channel_id=s.runtime_scope_channel_id AND assignment.state=1 AND assignment.ownership_generation=s.scope_ownership_generation
  AND assignment.holder_node_id=p_node AND assignment.holder_registration_revision=p_registration FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'NPC source Channel changed before private capture' USING ERRCODE='23514';END IF;
 actor_sex:=CASE root.sex WHEN 1 THEN 'female' WHEN 2 THEN 'male' ELSE NULL END;
 SELECT coalesce(jsonb_object_agg(track_key,value),'{}'::jsonb) INTO actual_tracks FROM game_character_quest_tracks
  WHERE character_id=s.character_id AND track_key=ANY(policy.track_keys);
 IF actor_sex IS NULL OR p_before->>'name' IS DISTINCT FROM root.name OR p_before->>'sex' IS DISTINCT FROM actor_sex OR p_before->'tracks' IS DISTINCT FROM actual_tracks
  OR p_plan->>'source_key' IS DISTINCT FROM policy.source_key OR jsonb_typeof(p_plan->'changes') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p_plan->'changes')>8 OR jsonb_typeof(p_plan->'grant_keys') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p_plan->'grant_keys')>2 THEN
  RAISE EXCEPTION 'NPC source plan differs from actual locked before state' USING ERRCODE='23514';END IF;
 IF jsonb_typeof(p_before->'achievement_progress') IS DISTINCT FROM 'object' OR EXISTS(
  SELECT 1 FROM jsonb_each_text(p_before->'achievement_progress') npc_progress_before
  WHERE npc_progress_before.value::bigint IS DISTINCT FROM coalesce((SELECT npc_progress.progress FROM game_character_achievement_progress npc_progress WHERE npc_progress.character_id=s.character_id AND npc_progress.achievement_key=npc_progress_before.key),0)) THEN
  RAISE EXCEPTION 'NPC source progress observation differs from locked actual state' USING ERRCODE='23514';END IF;
 IF p_plan->'effects' IS DISTINCT FROM policy.effects
  OR (p_before->>'level')::bigint IS DISTINCT FROM (SELECT progression.level FROM game_character_progression_state progression WHERE progression.character_id=s.character_id AND progression.character_revision=root.character_revision)
  OR p_plan->'costs' IS DISTINCT FROM policy.item_costs
  OR EXISTS(SELECT 1 FROM jsonb_array_elements_text(p_plan->'grant_keys') AS npc_grant(value) WHERE NOT npc_grant.value=ANY(policy.allowed_grant_keys))
  OR (SELECT count(DISTINCT value) FROM jsonb_array_elements_text(p_plan->'grant_keys'))<>jsonb_array_length(p_plan->'grant_keys')
  OR (p_before->>'now') IS NULL OR NOT EXISTS(SELECT 1 FROM game_npc_source_clock npc_clock WHERE npc_clock.backend_pid=pg_backend_pid() AND npc_clock.transaction_xid=pg_current_xact_id() AND npc_clock.node_id=p_node AND npc_clock.registration=p_registration AND npc_clock.now_seconds=(p_before->>'now')::bigint) THEN
  RAISE EXCEPTION 'NPC source plan changes its static costs/grants or source clock' USING ERRCODE='23514';END IF;
 SELECT count(*) INTO change_count FROM jsonb_object_keys(policy.writable_tracks);
 IF change_count<>jsonb_array_length(p_plan->'changes') OR
  (SELECT count(DISTINCT value->>'key') FROM jsonb_array_elements(p_plan->'changes'))<>change_count THEN
  RAISE EXCEPTION 'NPC source plan changes its exact track set' USING ERRCODE='23514';END IF;
 FOR change IN SELECT value FROM jsonb_array_elements(p_plan->'changes') LOOP
  rule:=policy.writable_tracks->(change->>'key');
  IF rule IS NULL OR change->>'quest' IS DISTINCT FROM rule->>'quest'
   OR (change->>'before')::bigint IS DISTINCT FROM coalesce((actual_tracks->>(change->>'key'))::bigint,-1)
   OR (change->>'after')::bigint IS DISTINCT FROM game_npc_source_expression(rule->'value',actual_tracks,(p_before->>'now')::bigint) THEN
   RAISE EXCEPTION 'NPC source track delta differs from actual before and finite expression' USING ERRCODE='23514';END IF;
 END LOOP;
 IF EXISTS(SELECT 1 FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'NPC source observation is sealed once per transaction' USING ERRCODE='23514';END IF;
 DELETE FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() OR backend_pid NOT IN(SELECT pid FROM pg_stat_activity);
 INSERT INTO game_npc_source_context(backend_pid,transaction_xid,node_id,registration,character_id,account_id,world_id,channel_id,
 game_session_id,command_id,connection_generation,lease_generation,scope_generation,original_revision,program_sha256,source_key,npc_key,
 npc_transaction_id,request_binding,before_state,expected_plan,plan_sha256)
 VALUES(pg_backend_pid(),pg_current_xact_id(),p_node,p_registration,s.character_id,s.account_id,s.world_id,s.runtime_scope_channel_id,
 s.game_session_id,(p_context->>'command_id')::numeric,s.current_generation,s.character_lease_generation,s.scope_ownership_generation,root.character_revision,
 p_program,policy.source_key,policy.npc_key,(p_context->>'npc_transaction_id')::uuid,decode(p_context->>'request_binding','hex'),p_before,p_plan,sha256(convert_to(p_plan::text,'UTF8')));
 RETURN sha256(convert_to(p_plan::text,'UTF8'));
END $$;

CREATE FUNCTION game_npc_source_bind_cost(p_cost UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE; r game_srv7_item_costs%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() FOR UPDATE;
 SELECT * INTO r FROM game_srv7_item_costs WHERE transaction_id=p_cost;
 IF c.character_id IS NULL OR c.captured OR c.cost_transaction_id IS NOT NULL OR p_cost=c.npc_transaction_id OR p_cost IS NOT DISTINCT FROM c.fee_transaction_id OR r.created_xact_id<>pg_current_xact_id()
  OR r.character_id IS DISTINCT FROM c.character_id OR r.game_session_id IS DISTINCT FROM c.game_session_id
  OR r.command_id IS DISTINCT FROM c.command_id OR r.cause_kind IS DISTINCT FROM 'npc_cosmetic' OR r.source_key IS DISTINCT FROM c.source_key
  OR r.requirements IS DISTINCT FROM c.expected_plan->'costs' THEN
  RAISE EXCEPTION 'NPC source cost is not its exact current physical cost' USING ERRCODE='23514';END IF;
 UPDATE game_npc_source_context SET cost_transaction_id=p_cost WHERE backend_pid=pg_backend_pid();
END $$;
CREATE FUNCTION game_npc_source_bind_fee(p_fee UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;p game_npc_source_policies%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() FOR UPDATE;
 SELECT * INTO p FROM game_npc_source_policies WHERE program_sha256=c.program_sha256 AND source_key=c.source_key;
 IF c.character_id IS NULL OR c.captured OR c.fee_transaction_id IS NOT NULL OR p_fee=c.npc_transaction_id OR p_fee IS NOT DISTINCT FROM c.cost_transaction_id OR p.fee_gold_units IS NULL
  OR p.fee_gold_units IS DISTINCT FROM (c.expected_plan->'fee'->>'gold')::bigint
  OR p.fee_service_key IS DISTINCT FROM c.expected_plan->'fee'->>'service' OR game_character_is_uuid_v7(p_fee) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'NPC source fee is not its exact declared native service' USING ERRCODE='23514';END IF;
 UPDATE game_npc_source_context SET fee_transaction_id=p_fee WHERE backend_pid=pg_backend_pid();
END $$;
CREATE FUNCTION game_npc_source_effect_current(p_parent UUID,p_source TEXT,p_effect JSONB) RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_context c
  JOIN game_runtime_scope_assignments assignment ON assignment.scope_kind=1 AND assignment.world_id=c.world_id AND assignment.channel_id=c.channel_id
  JOIN game_durability_reconnect_sessions session ON session.game_session_id=c.game_session_id
  WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.npc_transaction_id=p_parent AND c.source_key=p_source
   AND assignment.state=1 AND assignment.ownership_generation=c.scope_generation AND assignment.holder_node_id=c.node_id AND assignment.holder_registration_revision=c.registration
   AND session.character_id=c.character_id AND session.current_generation=c.connection_generation AND session.character_lease_generation=c.lease_generation
   AND session.scope_ownership_generation=c.scope_generation AND session.session_state=2
   AND EXISTS(SELECT 1 FROM jsonb_array_elements(c.expected_plan->'effects') effect WHERE effect.value=p_effect))
$$;
CREATE FUNCTION game_npc_source_parent_current(p_parent UUID,p_source TEXT) RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_context c JOIN game_npc_cosmetic_receipts parent ON parent.transaction_id=c.npc_transaction_id
  WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.npc_transaction_id=p_parent AND c.source_key=p_source
   AND parent.created_xact_id=pg_current_xact_id() AND parent.character_id=c.character_id AND parent.game_session_id=c.game_session_id
   AND parent.command_id=c.command_id AND parent.source_key=c.source_key AND parent.program_sha256=c.program_sha256 AND parent.plan_sha256=c.plan_sha256
   AND parent.original_character_revision=c.original_revision AND parent.committed_character_revision=c.original_revision+1)
$$;

CREATE FUNCTION game_npc_source_fee_current(p_fee UUID,p_character UUID,p_world UUID) RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_context c WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
  AND c.fee_transaction_id=p_fee AND c.character_id=p_character AND c.world_id=p_world)
$$;
CREATE FUNCTION game_npc_cash_prior_equipment(p_fee UUID,p_character UUID,p_world UUID)
RETURNS TABLE(equipment_before JSONB,cost_transaction_id UUID) LANGUAGE sql SECURITY DEFINER AS $$
 SELECT cost.equipment_before,cost.transaction_id FROM game_npc_source_context c
 JOIN game_srv7_item_costs cost ON cost.transaction_id=c.cost_transaction_id
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.fee_transaction_id=p_fee
  AND c.character_id=p_character AND c.world_id=p_world AND cost.created_xact_id=pg_current_xact_id()
  AND cost.character_id=c.character_id AND cost.game_session_id=c.game_session_id AND cost.command_id=c.command_id
  AND cost.cause_kind='npc_cosmetic' AND cost.source_key=c.source_key AND cost.equipment_before IS NOT NULL
$$;
CREATE FUNCTION game_npc_source_fee_complete(p_fee UUID,p_character UUID,p_world UUID) RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_context c JOIN game_npc_cosmetic_receipts receipt ON receipt.transaction_id=c.npc_transaction_id
  JOIN game_character_roots root ON root.character_id=c.character_id
  WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.fee_transaction_id=p_fee
   AND c.character_id=p_character AND c.world_id=p_world AND receipt.created_xact_id=pg_current_xact_id()
   AND receipt.character_id=c.character_id AND receipt.game_session_id=c.game_session_id AND receipt.command_id=c.command_id
   AND receipt.fee_transaction_id=p_fee AND receipt.cost_transaction_id IS NOT DISTINCT FROM c.cost_transaction_id
   AND receipt.program_sha256=c.program_sha256 AND receipt.plan_sha256=c.plan_sha256 AND receipt.source_key=c.source_key
   AND receipt.original_character_revision=c.original_revision AND receipt.committed_character_revision=c.original_revision+1
   AND root.character_revision=receipt.committed_character_revision)
$$;
DO $privileges$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY[
 'game_npc_source_expression(jsonb,jsonb,bigint)','game_npc_source_observe_clock(uuid,numeric,bytea)','game_npc_arm_source(uuid,numeric,bytea,jsonb,bytea,jsonb,jsonb)','game_npc_source_bind_cost(uuid)','game_npc_source_bind_fee(uuid)',
 'game_npc_source_effect_current(uuid,text,jsonb)','game_npc_source_parent_current(uuid,text)','game_npc_source_fee_current(uuid,uuid,uuid)','game_npc_cash_prior_equipment(uuid,uuid,uuid)','game_npc_source_fee_complete(uuid,uuid,uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
END $privileges$;
