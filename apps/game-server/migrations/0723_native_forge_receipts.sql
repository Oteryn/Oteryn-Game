-- P18: complete the existing native Forge owner. These are operation evidence,
-- never a second wallet, inventory, tier store or resource balance.
-- Crystal 201189f game.cpp:10994-11064; config.lua.dist:469 (500-10 ms).
CREATE TABLE game_forge_decision_root (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton),
 secret BYTEA NOT NULL CHECK(octet_length(secret)=32)
);
INSERT INTO game_forge_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid())));
CREATE TRIGGER game_forge_decision_root_immutable BEFORE UPDATE OR DELETE ON game_forge_decision_root
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_forge_decision_root_no_truncate BEFORE TRUNCATE ON game_forge_decision_root EXECUTE FUNCTION game_item_reject_truncate();

CREATE TABLE game_forge_cooldowns (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 next_action_at BIGINT NOT NULL CHECK(next_action_at>0)
);
CREATE TRIGGER game_forge_cooldowns_no_truncate BEFORE TRUNCATE ON game_forge_cooldowns EXECUTE FUNCTION game_item_reject_truncate();

CREATE TABLE game_forge_reservations (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 occurrence_id UUID NOT NULL CHECK(game_character_is_uuid_v7(occurrence_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id) AND event_id<>transaction_id),
 entry_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(entry_id) AND entry_id NOT IN(transaction_id,event_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 channel_id UUID NOT NULL CHECK(game_character_is_uuid_v7(channel_id) AND channel_id<>world_id),
 intent BYTEA NOT NULL CHECK(octet_length(intent)=51 AND get_byte(intent,0) BETWEEN 1 AND 7),
 -- Q7: immutable native operands are server-resolved; the original handle bytes
 -- remain the replay/occurrence identity even after the items disappear.
 first_item_instance_id UUID CHECK(game_character_is_uuid_v7(first_item_instance_id)),
 second_item_instance_id UUID CHECK(game_character_is_uuid_v7(second_item_instance_id)),
 -- The existing D1230 paid stage is a quote, frozen before effects can fail.
 expected_dust_upgrade_count SMALLINT CHECK(expected_dust_upgrade_count BETWEEN 0 AND 225),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 ruleset_revision TEXT NOT NULL CHECK(ruleset_revision='crystal-201189f-forge-v1'),
 simulation_revision BIGINT NOT NULL CHECK(simulation_revision BETWEEN 1 AND 4294967295),
 success_roll SMALLINT NOT NULL CHECK(success_roll BETWEEN 0 AND 99),
 protection_roll SMALLINT NOT NULL CHECK(protection_roll BETWEEN 0 AND 99),
 bonus_roll SMALLINT NOT NULL CHECK(bonus_roll BETWEEN 0 AND 10000),
 occurred_at BIGINT NOT NULL CHECK(occurred_at BETWEEN 1 AND 9223372029078775807),
 exhausted BOOLEAN NOT NULL,
 work_units_used SMALLINT NOT NULL DEFAULT 0 CHECK(work_units_used BETWEEN 0 AND 3),
 PRIMARY KEY(character_id,occurrence_id),
 UNIQUE(character_id,occurrence_id,transaction_id,event_id),
 CHECK(substring(intent FROM 2 FOR 16)=uuid_send(occurrence_id)),
 CHECK((get_byte(intent,0)=7)=(expected_dust_upgrade_count IS NOT NULL)),
 CHECK(get_byte(intent,50)=0 AND CASE WHEN get_byte(intent,0) BETWEEN 1 AND 4 THEN
   (get_byte(intent,17)&3)=3 AND get_byte(intent,17)<=15
   AND (get_byte(intent,0)=1 OR get_byte(intent,17)=3)
   AND substring(intent FROM 19 FOR 8)<>decode(repeat('00',8),'hex')
   AND substring(intent FROM 35 FOR 8)<>decode(repeat('00',8),'hex')
   AND substring(intent FROM 27 FOR 8)=decode(repeat('00',8),'hex')
   AND substring(intent FROM 43 FOR 8)=decode(repeat('00',8),'hex')
  ELSE get_byte(intent,17)=0 AND substring(intent FROM 19 FOR 32)=decode(repeat('00',32),'hex') END),
 CHECK((first_item_instance_id IS NULL AND second_item_instance_id IS NULL)
   OR (first_item_instance_id IS NOT NULL AND second_item_instance_id IS NOT NULL
     AND first_item_instance_id<>second_item_instance_id AND get_byte(intent,0) BETWEEN 1 AND 4))
);
CREATE FUNCTION game_forge_reservation_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.work_units_used=OLD.work_units_used+1
   AND (to_jsonb(NEW)-'work_units_used')=(to_jsonb(OLD)-'work_units_used') THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'Forge reservation identity, intent, revisions and draws are immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_forge_reservation_immutable BEFORE UPDATE OR DELETE ON game_forge_reservations
 FOR EACH ROW EXECUTE FUNCTION game_forge_reservation_guard();
CREATE TRIGGER game_forge_reservation_no_truncate BEFORE TRUNCATE ON game_forge_reservations EXECUTE FUNCTION game_item_reject_truncate();

-- Both entry points consume one DUR-03 allowance. Each stored counter remains
-- immutable except for +1, including adoption of an earlier pending occurrence.
CREATE FUNCTION game_forge_charge_attempt(p_character UUID,p_occurrence UUID)
RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE common_used SMALLINT; legacy_used SMALLINT; next_used SMALLINT;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:forge-dust-limit:' || encode(uuid_send(p_character),'hex') || ':' || encode(uuid_send(p_occurrence),'hex'),0));
 SELECT work_units_used INTO common_used FROM game_forge_reservations
  WHERE character_id=p_character AND occurrence_id=p_occurrence FOR UPDATE;
 SELECT work_units_used INTO legacy_used FROM game_forge_dust_limit_reservations
  WHERE character_id=p_character AND occurrence_id=p_occurrence FOR UPDATE;
 IF common_used IS NULL AND legacy_used IS NULL THEN RETURN false; END IF;
 next_used:=greatest(coalesce(common_used,0),coalesce(legacy_used,0))+1;
 IF next_used>3 THEN RETURN false; END IF;
 WHILE common_used<next_used LOOP
  UPDATE game_forge_reservations SET work_units_used=work_units_used+1
   WHERE character_id=p_character AND occurrence_id=p_occurrence AND work_units_used=common_used;
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge common attempt changed' USING ERRCODE='23514'; END IF;
  common_used:=common_used+1;
 END LOOP;
 WHILE legacy_used<next_used LOOP
  UPDATE game_forge_dust_limit_reservations SET work_units_used=work_units_used+1
   WHERE character_id=p_character AND occurrence_id=p_occurrence AND work_units_used=legacy_used;
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge legacy attempt changed' USING ERRCODE='23514'; END IF;
  legacy_used:=legacy_used+1;
 END LOOP;
 RETURN true;
END $$;

CREATE TABLE game_forge_operations (
 character_id UUID NOT NULL,
 occurrence_id UUID NOT NULL,
 transaction_id UUID NOT NULL UNIQUE,
 event_id UUID NOT NULL UNIQUE,
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 operation SMALLINT NOT NULL CHECK(operation BETWEEN 1 AND 7),
 reply JSONB NOT NULL CHECK(jsonb_typeof(reply)='object' AND octet_length(reply::text)<=512),
 applied BOOLEAN NOT NULL,
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=87),
 outputs JSONB NOT NULL CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs)<=1),
 stash JSONB CHECK(stash IS NULL OR jsonb_typeof(stash)='object'),
 dust_spent BIGINT NOT NULL CHECK(dust_spent IN(0,60,100,130,160)),
 gold_spent BIGINT NOT NULL CHECK(gold_spent BETWEEN 0 AND 300000000000),
 dust_before BIGINT,
 dust_limit SMALLINT,
 occurred_at BIGINT NOT NULL,
 audit_proof BYTEA CHECK(octet_length(audit_proof)<=4096),
 envelope_sha256 BYTEA CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,occurrence_id),
 FOREIGN KEY(character_id,occurrence_id,transaction_id,event_id)
  REFERENCES game_forge_reservations(character_id,occurrence_id,transaction_id,event_id),
 UNIQUE(game_session_id,command_id)
);

CREATE TABLE game_forge_history (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 occurrence_id UUID NOT NULL CHECK(game_character_is_uuid_v7(occurrence_id)),
 history_ordinal BIGINT NOT NULL CHECK(history_ordinal>0),
 reply JSONB NOT NULL CHECK(jsonb_typeof(reply)='object' AND octet_length(reply::text)<=512),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,occurrence_id),
 UNIQUE(character_id,history_ordinal)
);
CREATE INDEX game_forge_history_recent ON game_forge_history(character_id,history_ordinal DESC);
-- Preserve native DustLimit outcomes already visible through command181.
INSERT INTO game_forge_history(character_id,occurrence_id,history_ordinal,reply)
 SELECT h.character_id,h.occurrence_id,h.history_ordinal,
  jsonb_build_object('operation','dust_limit','succeeded',true,
    'result',jsonb_build_object('dust',r.balance_after,'dust_limit',r.dust_limit_after))
 FROM game_forge_dust_limit_history h JOIN game_forge_dust_limit_receipts r USING(character_id,occurrence_id)
 WHERE r.outcome=1;

-- Exact current carried ancestry, excluding equipped roots; depth and tree bounds
-- are the established item-loop8/500 limits. The saved ordinal is never a new handle.
CREATE FUNCTION game_forge_carried(p_character UUID,p_world UUID,p_item UUID,p_location JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 WITH RECURSIVE carried(item,parent,ordinal,depth,kind) AS (
  SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,1,1
   FROM game_item_container_entries e JOIN game_item_container_slots b
    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id AND b.world_id=e.world_id
   WHERE e.character_id=p_character AND e.world_id=p_world
  UNION ALL
  SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,c.depth+1,2
   FROM carried c JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item AND e.world_id=p_world
   JOIN game_item_instances parent ON parent.item_instance_id=c.item AND parent.lifecycle=1
   WHERE c.depth<8
 ) SELECT EXISTS(SELECT 1 FROM carried c WHERE c.item=p_item
     AND c.parent=(p_location->>'parent')::uuid AND c.ordinal=(p_location->>'ordinal')::numeric
     AND c.kind=(p_location->>'kind')::int)
$$;

CREATE FUNCTION game_forge_operation_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_forge_reservations%ROWTYPE; i game_item_instances%ROWTYPE; l JSONB; n INT:=0; key TEXT;
BEGIN
 SELECT * INTO STRICT r FROM game_forge_reservations WHERE character_id=NEW.character_id AND occurrence_id=NEW.occurrence_id FOR UPDATE;
 IF r.work_units_used=0 OR r.transaction_id<>NEW.transaction_id OR r.event_id<>NEW.event_id
    OR r.occurred_at<>NEW.occurred_at OR get_byte(r.intent,0)<>NEW.operation
    OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=r.world_id AND lifecycle=1)
 THEN RAISE EXCEPTION 'Forge operation requires its current native reservation' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT NEW.applied THEN
  IF NEW.inputs<>'[]'::jsonb OR NEW.outputs<>'[]'::jsonb OR NEW.stash IS NOT NULL
    OR NEW.dust_spent<>0 OR NEW.gold_spent<>0 OR NEW.audit_proof IS NOT NULL OR NEW.envelope_sha256 IS NOT NULL
  THEN RAISE EXCEPTION 'terminal refusal cannot carry effects' USING ERRCODE='23514'; END IF;
  IF NEW.operation<>7 AND (NEW.reply->'applied' IS DISTINCT FROM 'false'::jsonb
     OR NEW.reply->'succeeded' IS DISTINCT FROM 'false'::jsonb OR NEW.reply->'result' IS DISTINCT FROM 'null'::jsonb
     OR jsonb_typeof(NEW.reply->'reason') IS DISTINCT FROM 'string') THEN
   RAISE EXCEPTION 'terminal refusal requires a closed refusal reply' USING ERRCODE='23514'; END IF;
  RETURN NEW;
 END IF;
 IF NEW.operation=7 THEN RAISE EXCEPTION 'DustLimit composes its existing receipt' USING ERRCODE='23514'; END IF;
 IF r.exhausted OR NEW.dust_before IS NULL OR NEW.dust_limit IS NULL OR NEW.audit_proof IS NULL
    OR NOT (NEW.reply ?& ARRAY['operation','succeeded','applied','reason','bonus','result'])
    OR NEW.reply->'applied' IS DISTINCT FROM 'true'::jsonb OR jsonb_typeof(NEW.reply->'succeeded') IS DISTINCT FROM 'boolean'
    OR jsonb_typeof(NEW.reply->'result') IS DISTINCT FROM 'object'
    OR NOT (NEW.reply->'result' ?& ARRAY['first_item','first_tier','second_item','second_tier','dust','dust_limit','gold_spent','cores_spent','slivers_spent','slivers_created','cores_created'])
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['dust','dust_limit','gold_spent','cores_spent','slivers_spent','slivers_created','cores_created']) k
       WHERE jsonb_typeof(NEW.reply->'result'->k) IS DISTINCT FROM 'number')
    OR NEW.reply->'result'->>'gold_spent' IS DISTINCT FROM NEW.gold_spent::text
    OR (NEW.reply->'result'->>'dust')::bigint<>NEW.dust_before-NEW.dust_spent
    OR (NEW.reply->'result'->>'dust_limit')::smallint<>NEW.dust_limit
    OR NEW.dust_spent>NEW.dust_before
    OR NEW.dust_limit NOT BETWEEN 100 AND 4600 OR (NEW.dust_limit-100)%20<>0
    OR NEW.dust_before<>(SELECT coalesce((SELECT balance FROM game_character_forge_dust WHERE character_id=NEW.character_id),0))
    OR NEW.dust_limit<>(SELECT coalesce((SELECT dust_limit FROM game_character_forge_dust WHERE character_id=NEW.character_id),100))
    OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.inputs))<>jsonb_array_length(NEW.inputs)
 THEN RAISE EXCEPTION 'invalid native Forge action proof' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
  n:=n+1;
  IF jsonb_typeof(l) IS DISTINCT FROM 'object' OR NOT(l ?& ARRAY['item','before','location','quantity_after','tier_before','tier_after'])
     OR jsonb_typeof(l->'item') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'before') IS DISTINCT FROM 'object'
     OR jsonb_typeof(l->'location') IS DISTINCT FROM 'object'
     OR NOT(l->'location' ?& ARRAY['kind','parent','ordinal'])
     OR jsonb_typeof(l->'location'->'parent') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'location'->'ordinal') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'location'->'kind') IS DISTINCT FROM 'number'
     OR EXISTS(SELECT 1 FROM unnest(ARRAY['quantity_after','tier_before','tier_after']) k WHERE jsonb_typeof(l->k) IS DISTINCT FROM 'number')
  THEN RAISE EXCEPTION 'Forge input proof is incomplete' USING ERRCODE='23514'; END IF;
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM l->'before' OR i.world_id<>r.world_id OR i.lifecycle<>1
    OR i.definition_family<>'Item' OR i.quantity<1
    OR NOT game_forge_carried(NEW.character_id,r.world_id,i.item_instance_id,l->'location')
    OR (l->>'tier_before')::int<>coalesce((SELECT tier FROM game_item_tiers WHERE item_instance_id=i.item_instance_id),0)
    OR (l->>'tier_after')::int NOT BETWEEN 0 AND 10 OR (l->>'quantity_after')::bigint NOT BETWEEN 0 AND 100
    OR ((l->>'quantity_after')::bigint=0 AND (l->>'tier_after')::int<>0)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  THEN RAISE EXCEPTION 'Forge input changed custody, state or tier' USING ERRCODE='23514'; END IF;
  IF NEW.operation BETWEEN 1 AND 4 AND n<=2 THEN
   IF i.quantity<>1 OR (l->>'quantity_after')::bigint NOT IN(0,1)
      OR (n=1 AND i.item_instance_id IS DISTINCT FROM r.first_item_instance_id)
      OR (n=2 AND i.item_instance_id IS DISTINCT FROM r.second_item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'Forge pair must match its distinct single-item intent' USING ERRCODE='23514'; END IF;
  ELSE
   key:=CASE WHEN NEW.operation=5 THEN 'oteryn:item.tibia.i37109'
             WHEN NEW.operation=6 AND (l->>'quantity_after')::bigint>i.quantity THEN 'oteryn:item.tibia.i37110'
             WHEN NEW.operation=6 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END;
   IF i.definition_production_key<>key OR (l->>'tier_before')::int<>0 OR (l->>'tier_after')::int<>0
      OR (NEW.operation BETWEEN 1 AND 4 AND (l->>'quantity_after')::bigint>=i.quantity)
      OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'Forge material requires plain native resource units' USING ERRCODE='23514'; END IF;
  END IF;
 END LOOP;
 IF NEW.operation BETWEEN 1 AND 4 AND jsonb_array_length(NEW.inputs)<2 THEN
  RAISE EXCEPTION 'Forge pair missing' USING ERRCODE='23514'; END IF;
 IF NEW.operation BETWEEN 1 AND 4 AND NEW.outputs<>'[]'::jsonb THEN
  RAISE EXCEPTION 'tier changes preserve ItemInstance identity' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF jsonb_typeof(l) IS DISTINCT FROM 'object' OR NOT(l ?& ARRAY['item','key','revision','count','location'])
     OR jsonb_typeof(l->'item') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'key') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'revision') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'count') IS DISTINCT FROM 'number'
     OR jsonb_typeof(l->'location') IS DISTINCT FROM 'object' OR NOT(l->'location' ?& ARRAY['kind','parent','ordinal'])
     OR jsonb_typeof(l->'location'->'parent') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'location'->'ordinal') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'location'->'kind') IS DISTINCT FROM 'number'
  THEN RAISE EXCEPTION 'Forge output proof is incomplete' USING ERRCODE='23514'; END IF;
  IF NEW.operation NOT IN(5,6) OR NOT game_character_is_uuid_v7((l->>'item')::uuid)
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid)
    OR l->>'key'<>(CASE NEW.operation WHEN 5 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END)
    OR (l->>'count')::int NOT BETWEEN 1 AND (CASE NEW.operation WHEN 5 THEN 3 ELSE 1 END)
    OR octet_length(l->>'revision') NOT BETWEEN 1 AND 512 OR (l->'location'->>'ordinal')::numeric<=0
    OR (l->'location'->>'kind')::int NOT IN(1,2)
  THEN RAISE EXCEPTION 'invalid fresh Forge resource output' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.stash IS NOT NULL AND (NOT(NEW.stash ?& ARRAY['entry','key','revision','amount','before','after','previous'])
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['entry','key','revision','previous']) k WHERE jsonb_typeof(NEW.stash->k) IS DISTINCT FROM 'string')
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['amount','before','after']) k WHERE jsonb_typeof(NEW.stash->k) IS DISTINCT FROM 'number')
    OR (NEW.stash->>'amount')::bigint<=0 OR NEW.stash->>'key'<>(CASE WHEN NEW.operation=6 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END)
    OR NEW.operation=5) THEN RAISE EXCEPTION 'Forge stash proof is incomplete' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_forge_operation_finish() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.created_xact_id=pg_current_xact_id() AND OLD.envelope_sha256 IS NULL AND NEW.envelope_sha256 IS NOT NULL
   AND ((to_jsonb(NEW)-'envelope_sha256')=(to_jsonb(OLD)-'envelope_sha256')
    OR (OLD.operation=7 AND NOT OLD.applied AND NEW.applied
      AND (to_jsonb(NEW)-ARRAY['envelope_sha256','applied'])=(to_jsonb(OLD)-ARRAY['envelope_sha256','applied'])
      AND EXISTS(SELECT 1 FROM game_forge_dust_limit_receipts d WHERE d.transaction_id=NEW.transaction_id
        AND d.character_id=NEW.character_id AND d.occurrence_id=NEW.occurrence_id AND d.outcome=1
        AND d.envelope_sha256=NEW.envelope_sha256 AND d.created_xact_id=pg_current_xact_id())))
 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'terminal Forge outcomes and effect evidence are immutable' USING ERRCODE='23514';
END $$;

CREATE FUNCTION game_forge_history_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  IF (SELECT count(*) FROM game_forge_history WHERE character_id=OLD.character_id AND history_ordinal>OLD.history_ordinal)>=1000 THEN RETURN OLD; END IF;
  RAISE EXCEPTION 'only evict Forge history outside the shared1000 row budget' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'Forge history is immutable' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 IF NEW.history_ordinal<>(SELECT coalesce(max(history_ordinal),0)+1 FROM game_forge_history WHERE character_id=NEW.character_id)
    OR NOT (EXISTS(SELECT 1 FROM game_forge_operations p WHERE p.character_id=NEW.character_id
       AND p.occurrence_id=NEW.occurrence_id AND p.applied AND p.reply=NEW.reply AND p.created_xact_id=pg_current_xact_id())
     OR EXISTS(SELECT 1 FROM game_forge_dust_limit_receipts p WHERE p.character_id=NEW.character_id
       AND p.occurrence_id=NEW.occurrence_id AND p.outcome=1 AND p.created_xact_id=pg_current_xact_id()
       AND NEW.reply=jsonb_build_object('operation','dust_limit','succeeded',true,
         'result',jsonb_build_object('dust',p.balance_after,'dust_limit',p.dust_limit_after))))
 THEN RAISE EXCEPTION 'Forge history needs this transaction applied result' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE FUNCTION game_forge_history_from_dust_limit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO game_forge_history(character_id,occurrence_id,history_ordinal,reply)
 SELECT NEW.character_id,NEW.occurrence_id,(SELECT coalesce(max(history_ordinal),0)+1 FROM game_forge_history WHERE character_id=NEW.character_id),
   jsonb_build_object('operation','dust_limit','succeeded',true,
     'result',jsonb_build_object('dust',r.balance_after,'dust_limit',r.dust_limit_after))
 FROM game_forge_dust_limit_receipts r WHERE r.character_id=NEW.character_id AND r.occurrence_id=NEW.occurrence_id AND r.outcome=1
 ON CONFLICT(character_id,occurrence_id) DO NOTHING;
 DELETE FROM game_forge_history WHERE character_id=NEW.character_id AND history_ordinal IN
  (SELECT history_ordinal FROM game_forge_history WHERE character_id=NEW.character_id ORDER BY history_ordinal DESC OFFSET 1000);
 RETURN NULL;
END $$;

CREATE FUNCTION game_forge_history_evicted() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- Keep the legacy read API as a projection of the same retention budget.
 DELETE FROM game_forge_dust_limit_history WHERE character_id=OLD.character_id AND occurrence_id=OLD.occurrence_id;
 RETURN NULL;
END $$;
DO $migration$ DECLARE body TEXT; begin_at INT; BEGIN
 body:=pg_get_functiondef('game_forge_dust_limit_history_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing legacy Forge history proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM game_forge_history h WHERE h.character_id=OLD.character_id AND h.occurrence_id=OLD.occurrence_id)
    AND (SELECT count(*) FROM game_forge_history WHERE character_id=OLD.character_id)>=1000
    AND EXISTS(SELECT 1 FROM game_forge_history WHERE character_id=OLD.character_id AND created_xact_id=pg_current_xact_id())
  THEN RETURN OLD; END IF;
$arm$ from begin_at for 5);
END $migration$;

CREATE TRIGGER game_forge_operation_admit BEFORE INSERT ON game_forge_operations FOR EACH ROW EXECUTE FUNCTION game_forge_operation_admit();
CREATE TRIGGER game_forge_operation_immutable BEFORE UPDATE OR DELETE ON game_forge_operations FOR EACH ROW EXECUTE FUNCTION game_forge_operation_finish();
CREATE TRIGGER game_forge_operation_no_truncate BEFORE TRUNCATE ON game_forge_operations EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_forge_history_guard BEFORE INSERT OR UPDATE OR DELETE ON game_forge_history FOR EACH ROW EXECUTE FUNCTION game_forge_history_guard();
CREATE TRIGGER game_forge_history_no_truncate BEFORE TRUNCATE ON game_forge_history EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_forge_history_evicted AFTER DELETE ON game_forge_history FOR EACH ROW EXECUTE FUNCTION game_forge_history_evicted();
CREATE TRIGGER game_forge_history_from_dust_limit AFTER INSERT ON game_forge_dust_limit_history FOR EACH ROW EXECUTE FUNCTION game_forge_history_from_dust_limit();

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_forge_reservation_guard()','game_forge_operation_admit()',
 'game_forge_operation_finish()','game_forge_history_guard()','game_forge_history_from_dust_limit()','game_forge_history_evicted()',
 'game_forge_carried(uuid,uuid,uuid,jsonb)','game_forge_charge_attempt(uuid,uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
REVOKE ALL ON game_forge_decision_root,game_forge_cooldowns,game_forge_reservations,game_forge_operations,game_forge_history FROM PUBLIC;
GRANT SELECT ON game_forge_decision_root TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_forge_cooldowns,game_forge_reservations,game_forge_operations TO oteryn_game_runtime;
GRANT UPDATE(next_action_at) ON game_forge_cooldowns TO oteryn_game_runtime;
GRANT UPDATE(work_units_used) ON game_forge_reservations TO oteryn_game_runtime;
GRANT UPDATE(envelope_sha256,applied) ON game_forge_operations TO oteryn_game_runtime;
GRANT SELECT,INSERT,DELETE ON game_forge_history TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_forge_carried(uuid,uuid,uuid,jsonb) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_forge_charge_attempt(uuid,uuid) TO oteryn_game_runtime;
