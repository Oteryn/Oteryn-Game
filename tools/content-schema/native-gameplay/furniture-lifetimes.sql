-- Continuous lamps and short decoration animations reuse the existing timed Item rows.
-- This is source lifetime population, not an additional clock or scheduling store.
CREATE TABLE game_house_furniture_lifetimes (
 item_id INTEGER PRIMARY KEY CHECK(item_id>0),
 duration_ms BIGINT CHECK(duration_ms BETWEEN 1 AND 604800000),
 target_id INTEGER CHECK(target_id>0),
 stop_duration BOOLEAN NOT NULL
);
INSERT INTO game_house_furniture_lifetimes VALUES
__LIFETIME_ROWS__;
REVOKE ALL ON game_house_furniture_lifetimes FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_lifetimes TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_furniture_timer_proposal(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE before_timer JSONB; lifetime game_house_furniture_lifetimes%ROWTYPE;
BEGIN
 SELECT to_jsonb(t) INTO before_timer FROM game_item_timed_states t WHERE t.item_instance_id=r.source_item_instance_id;
 IF NOT (r.effects ? 'house_temporal') THEN RETURN before_timer IS NULL; END IF;
 SELECT * INTO lifetime FROM game_house_furniture_lifetimes
  WHERE 'oteryn:item.tibia.i'||item_id::text=r.effects->'house_after'->>'key';
 RETURN before_timer IS NOT DISTINCT FROM nullif(r.effects->'house_temporal'->'before','null'::jsonb)
  AND (before_timer->>'charges') IS NULL
  AND ((before_timer->>'deadline_at')::bigint IS NULL OR (before_timer->>'deadline_at')::bigint>r.occurred_at)
  AND nullif(r.effects->'house_temporal'->'duration','null'::jsonb) IS NOT DISTINCT FROM to_jsonb(lifetime.duration_ms)
  AND nullif(r.effects->'house_temporal'->'target','null'::jsonb)
   IS NOT DISTINCT FROM to_jsonb('oteryn:item.tibia.i'||lifetime.target_id::text);
END $$;

-- A source action may update its existing clock only with that same immutable receipt.
-- Its before row is observed under the existing named-House source locks.
CREATE FUNCTION game_house_furniture_timer_values(r game_item_consume_transform_records)
 RETURNS TABLE(remaining BIGINT,deadline BIGINT) LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE old_row JSONB:=r.effects->'house_temporal'->'before'; life game_house_furniture_lifetimes%ROWTYPE;
 old_remaining BIGINT:=(old_row->>'remaining_ms')::bigint; old_deadline BIGINT:=(old_row->>'deadline_at')::bigint;
 source_stops BOOLEAN;
BEGIN
 SELECT * INTO life FROM game_house_furniture_lifetimes WHERE 'oteryn:item.tibia.i'||item_id::text=r.effects->'house_after'->>'key';
 SELECT stop_duration INTO source_stops FROM game_house_furniture_lifetimes
  WHERE 'oteryn:item.tibia.i'||item_id::text=r.before_item->>'definition_production_key';
 IF r.effects->>'house_wrap_action'='1' THEN
  remaining:=CASE WHEN old_deadline IS NOT NULL THEN greatest(0,least(old_remaining,old_deadline-r.occurred_at)) ELSE old_remaining END;deadline:=NULL;
 ELSIF r.effects->>'house_rotate'='true' AND old_remaining IS NOT NULL AND life.duration_ms IS NOT NULL THEN
  remaining:=least(old_remaining,life.duration_ms);
  IF old_deadline IS NOT NULL THEN remaining:=greatest(0,least(remaining,old_deadline-r.occurred_at)); END IF;
  deadline:=coalesce(old_deadline,r.occurred_at+remaining);
 ELSIF life.duration_ms IS NOT NULL THEN
  remaining:=CASE WHEN (source_stops OR r.effects->>'house_wrap_action'='2') AND old_remaining IS NOT NULL THEN least(old_remaining,life.duration_ms) ELSE life.duration_ms END;
  deadline:=r.occurred_at+remaining;
 ELSIF life.stop_duration THEN
  remaining:=CASE WHEN old_deadline IS NOT NULL THEN greatest(0,least(old_remaining,old_deadline-r.occurred_at)) ELSE old_remaining END;
  deadline:=NULL;
 ELSE remaining:=NULL;deadline:=NULL;
 END IF;
 RETURN NEXT;
END $$;

-- A placement starts a furniture ItemType clock; wrapping pauses its remaining
-- budget and unwrapping resumes it on the same instance. The original wrap
-- receipt supplies the same-XID before timer; no second receipt store is needed.
ALTER TABLE game_house_furniture_wrap_receipts
 ADD COLUMN before_timer JSONB,
 ADD COLUMN acted_at BIGINT NOT NULL DEFAULT floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure);
 needle:='OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p_item)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing wrap timer admission'; END IF;
 body:=replace(body,needle,'OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p_item AND charges IS NOT NULL)');
 needle:='world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing wrap timer snapshot columns'; END IF;
 body:=replace(body,needle,'world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location,before_timer)');
 needle:='VALUES(tx,p_session,p_command,p_character,p_item,p_world,p_house,p_intent,p_action,to_jsonb(i),p_after,to_jsonb(h),to_jsonb(e));';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing wrap timer snapshot values'; END IF;
 body:=replace(body,needle,'VALUES(tx,p_session,p_command,p_character,p_item,p_world,p_house,p_intent,p_action,to_jsonb(i),p_after,to_jsonb(h),to_jsonb(e),(SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=p_item));');
 EXECUTE body;
END $migration$;

CREATE FUNCTION game_house_furniture_timer_record(p_tx UUID) RETURNS game_item_consume_transform_records
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; w game_house_furniture_wrap_receipts%ROWTYPE;
 life game_house_furniture_lifetimes%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_tx AND (source_kind=5 OR (source_kind IN(1,4) AND (effects->>'kind'='furniture_rotation' OR effects->>'portable_use'='true')));
 IF FOUND THEN RETURN r; END IF;
 SELECT * INTO w FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_tx;
 IF NOT FOUND THEN RETURN r; END IF;
 r.transaction_id:=w.transaction_id;r.source_kind:=5;r.created_xact_id:=w.created_xact_id;
 r.source_item_instance_id:=w.item_instance_id;r.before_item:=w.before_item;r.occurred_at:=w.acted_at;
 r.effects:=jsonb_build_object('house_after',jsonb_build_object('key',w.after_key,'revision',w.before_item->>'definition_revision_ref'),
  'house_wrap_action',w.action);
 SELECT * INTO life FROM game_house_furniture_lifetimes WHERE 'oteryn:item.tibia.i'||item_id::text=w.after_key;
 IF w.before_timer IS NOT NULL OR life.duration_ms IS NOT NULL THEN
  r.effects:=r.effects||jsonb_build_object('house_temporal',jsonb_build_object('before',w.before_timer,
   'duration',life.duration_ms,'target','oteryn:item.tibia.i'||life.target_id::text));
 END IF;
 RETURN r;
END $$;

ALTER FUNCTION game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)
 RENAME TO game_house_furniture_wrap_before_srv7_timers;
CREATE FUNCTION game_house_furniture_wrap(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,
 p_command NUMERIC,p_item UUID,p_revision NUMERIC,p_acl NUMERIC,p_action SMALLINT,p_position BYTEA,
 p_intent BYTEA,p_after TEXT,p_capacity INTEGER,p_backpack TEXT) RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE result NUMERIC; tx UUID;
BEGIN
 result:=game_house_furniture_wrap_before_srv7_timers(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,
  p_world,p_house,p_command,p_item,p_revision,p_acl,p_action,p_position,p_intent,p_after,p_capacity,p_backpack);
 SELECT transaction_id INTO tx FROM game_house_furniture_wrap_receipts WHERE game_session_id=p_session AND command_id=p_command
  AND created_xact_id=pg_current_xact_id();
 IF tx IS NOT NULL THEN PERFORM game_house_furniture_timer_apply(tx); END IF;
 RETURN result;
END $$;

CREATE FUNCTION game_house_furniture_timer_apply(p_tx UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; t game_item_timed_states%ROWTYPE;
 w game_item_timed_state_writes%ROWTYPE; v RECORD; duration BIGINT;
BEGIN
 r:=game_house_furniture_timer_record(p_tx);
 IF r.transaction_id IS NULL OR NOT(r.effects ? 'house_temporal') THEN RETURN; END IF;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF (CASE WHEN t.item_instance_id IS NULL THEN NULL ELSE to_jsonb(t) END) IS DISTINCT FROM nullif(r.effects->'house_temporal'->'before','null'::jsonb)
  THEN RAISE EXCEPTION 'House decoration timer changed' USING ERRCODE='23514'; END IF;
 SELECT * INTO v FROM game_house_furniture_timer_values(r);
 duration:=(r.effects->'house_temporal'->>'duration')::bigint;
 w.training_created_xact_id:=pg_current_xact_id();
 w.item_instance_id:=r.source_item_instance_id;w.expected_revision:=coalesce(t.state_revision,0);
 w.cause:=CASE WHEN v.deadline IS NOT NULL THEN 5 ELSE 6 END;w.transaction_id:=r.transaction_id;
 w.definition_before_family:='Item';w.definition_before_production_key:=r.before_item->>'definition_production_key';
 w.definition_before_revision_ref:=r.before_item->>'definition_revision_ref';w.definition_after_family:='Item';
 w.definition_after_production_key:=r.effects->'house_after'->>'key';w.definition_after_revision_ref:=r.effects->'house_after'->>'revision';
 w.full_remaining_ms:=duration;w.lit_continuous:=v.deadline IS NOT NULL;
 w.remaining_ms_before:=t.remaining_ms;w.deadline_before:=t.deadline_at;
 w.remaining_ms_after:=v.remaining;w.deadline_after:=v.deadline;
 IF duration IS NOT NULL THEN
  INSERT INTO game_item_timed_definitions VALUES('Item',w.definition_after_production_key,w.definition_after_revision_ref,NULL,duration,true)
   ON CONFLICT DO NOTHING;
 END IF;
 INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
 IF t.item_instance_id IS NULL THEN
  INSERT INTO game_item_timed_states VALUES(w.item_instance_id,NULL,v.remaining,v.deadline,1);
 ELSE UPDATE game_item_timed_states SET charges=NULL,remaining_ms=v.remaining,deadline_at=v.deadline,state_revision=state_revision+1
  WHERE item_instance_id=w.item_instance_id;
 END IF;
END $$;

CREATE FUNCTION game_house_furniture_timer_action_complete(w game_item_timed_state_writes) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; v RECORD; before_timer JSONB;
BEGIN
 r:=game_house_furniture_timer_record(w.transaction_id);
 IF r.transaction_id IS NULL OR NOT(r.effects ? 'house_temporal') OR r.created_xact_id<>pg_current_xact_id()
  OR w.created_xact_id<>pg_current_xact_id() OR w.item_instance_id<>r.source_item_instance_id
  OR w.holder_character_id IS NOT NULL OR w.full_charges IS NOT NULL OR w.charges_before IS NOT NULL
  OR w.charges_after IS NOT NULL OR w.committed_at<r.occurred_at THEN RETURN FALSE; END IF;
 before_timer:=r.effects->'house_temporal'->'before';
 SELECT * INTO v FROM game_house_furniture_timer_values(r);
 RETURN w.expected_revision=coalesce((before_timer->>'state_revision')::numeric,0)
  AND w.cause=CASE WHEN v.deadline IS NOT NULL THEN 5 ELSE 6 END
  AND w.definition_before_family='Item' AND w.definition_after_family='Item'
  AND w.definition_before_production_key=r.before_item->>'definition_production_key'
  AND w.definition_before_revision_ref=r.before_item->>'definition_revision_ref'
  AND w.definition_after_production_key=r.effects->'house_after'->>'key'
  AND w.definition_after_revision_ref=r.effects->'house_after'->>'revision'
  AND w.remaining_ms_before IS NOT DISTINCT FROM (before_timer->>'remaining_ms')::bigint
  AND w.deadline_before IS NOT DISTINCT FROM (before_timer->>'deadline_at')::bigint
  AND w.remaining_ms_after IS NOT DISTINCT FROM v.remaining AND w.deadline_after IS NOT DISTINCT FROM v.deadline
  AND w.full_remaining_ms IS NOT DISTINCT FROM (r.effects->'house_temporal'->>'duration')::bigint
  AND w.lit_continuous=(v.deadline IS NOT NULL)
  AND EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=w.item_instance_id
   AND t.state_revision=w.expected_revision+1 AND t.charges IS NULL
   AND t.remaining_ms IS NOT DISTINCT FROM w.remaining_ms_after AND t.deadline_at IS NOT DISTINCT FROM w.deadline_after);
END $$;

DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_item_action_house_admit(game_item_consume_transform_records)'::regprocedure);
 needle:='OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing House timer admission'; END IF;
 EXECUTE replace(body,needle,'OR game_house_furniture_timer_proposal(r) IS DISTINCT FROM TRUE');
 body:=pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure);
 EXECUTE overlay(body placing E'BEGIN\n IF game_house_furniture_timer_action_complete(NEW) THEN RETURN NULL; END IF;\n'
  from regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i') for 5);
END $migration$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_furniture_timer_record(UUID)'::regprocedure,
  'game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure,
  'game_house_furniture_timer_proposal(game_item_consume_transform_records)'::regprocedure,
  'game_house_furniture_timer_values(game_item_consume_transform_records)'::regprocedure,
  'game_house_furniture_timer_apply(UUID)'::regprocedure,
  'game_house_furniture_timer_action_complete(game_item_timed_state_writes)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_furniture_timer_apply(UUID),
 game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)
 TO oteryn_game_runtime;
