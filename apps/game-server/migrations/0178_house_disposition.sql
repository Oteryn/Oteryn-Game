-- HOUSE-1b. MIGRATOR is the sole installer. Private native custody writer;
-- neither the runtime nor the control role gains a disposition mutation grant.
-- DRAINED retains the owner and every content/entry fence for HOUSE-1a release.
INSERT INTO game_character_inbox_cause_kinds VALUES ('HOUSE_DISPOSITION');
CREATE TABLE game_house_dispositions (
 operation_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7(),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 ownership_revision NUMERIC(20,0) NOT NULL,
 cause TEXT NOT NULL DEFAULT 'MOVE_OUT' CHECK(cause='MOVE_OUT'),
 state SMALLINT NOT NULL CHECK(state IN(1,2,3,4)), -- notice, frozen, drained, cancelled
 noticed_at BIGINT NOT NULL, due_at BIGINT NOT NULL,
 written_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 cancelled_at BIGINT, frozen_at BIGINT, drained_at BIGINT,
 census_count INT CHECK(census_count BETWEEN 0 AND 2000), census_digest BYTEA,
 CHECK(due_at::numeric-noticed_at BETWEEN 86400000 AND 2592000000),
 CHECK((state=4)=(cancelled_at IS NOT NULL)),
 CHECK((state IN(2,3))=(frozen_at IS NOT NULL)),
 CHECK((state=3)=(drained_at IS NOT NULL)),
 CHECK((frozen_at IS NULL)=(census_count IS NULL)),
 CHECK((census_count IS NULL)=(census_digest IS NULL)),
 CHECK(census_digest IS NULL OR octet_length(census_digest)=32)
);
CREATE UNIQUE INDEX game_house_disposition_live ON game_house_dispositions(world_id,house_key) WHERE state<>4;
CREATE INDEX game_house_disposition_due ON game_house_dispositions(world_id,due_at,operation_id) WHERE state IN(1,2);
CREATE TABLE game_house_disposition_commands (
 game_session_id UUID NOT NULL, command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 binding BYTEA NOT NULL CHECK(octet_length(binding) BETWEEN 1 AND 2048),
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),
 outcome JSONB NOT NULL, PRIMARY KEY(game_session_id,command_id)
);
CREATE TABLE game_house_disposition_census (
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),
 item_instance_id UUID NOT NULL, reclaim_subject UUID NOT NULL REFERENCES game_character_roots(character_id),
 source_location JSONB NOT NULL, source_provenance JSONB NOT NULL, source_item JSONB NOT NULL,
 PRIMARY KEY(operation_id,item_instance_id)
);
CREATE TABLE game_house_disposition_steps (
 operation_id UUID NOT NULL, item_instance_id UUID NOT NULL,
 reclaim_subject UUID NOT NULL, ordinal NUMERIC(20,0) NOT NULL,
 step_number INT NOT NULL CHECK(step_number BETWEEN 1 AND 20),
 written_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(operation_id,item_instance_id) REFERENCES game_house_disposition_census(operation_id,item_instance_id),
 PRIMARY KEY(operation_id,item_instance_id)
);

CREATE TABLE game_house_disposition_audit_outbox (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 operation_id UUID NOT NULL REFERENCES game_house_dispositions,
 event_type_id BIGINT NOT NULL DEFAULT 8 CHECK(event_type_id=8),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 131072),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+2592000000),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 publication_state SMALLINT NOT NULL DEFAULT 1 CHECK(publication_state IN(1,2)),
 published_at BIGINT CHECK(published_at IS NULL OR published_at>=occurred_at),
 written_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((publication_state=2)=(published_at IS NOT NULL))
);
CREATE TRIGGER game_house_disposition_audit_guard BEFORE UPDATE OR DELETE ON game_house_disposition_audit_outbox
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_audit_guard();
REVOKE ALL ON game_house_disposition_audit_outbox FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,DELETE ON game_house_disposition_audit_outbox TO oteryn_game_runtime;
GRANT UPDATE(publication_state,published_at) ON game_house_disposition_audit_outbox TO oteryn_game_runtime;

CREATE FUNCTION game_house_disposition_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'UPDATE' OR
    (to_jsonb(NEW)-ARRAY['state','cancelled_at','frozen_at','drained_at','census_count','census_digest','written_xact_id']) IS DISTINCT FROM
    (to_jsonb(OLD)-ARRAY['state','cancelled_at','frozen_at','drained_at','census_count','census_digest','written_xact_id']) OR
    NOT ((OLD.state=1 AND NEW.state IN(2,4)) OR (OLD.state=2 AND NEW.state=3)) OR
    (OLD.state=2 AND (NEW.frozen_at,NEW.census_count,NEW.census_digest) IS DISTINCT FROM (OLD.frozen_at,OLD.census_count,OLD.census_digest)) THEN
  RAISE EXCEPTION 'immutable house disposition binding/transition' USING ERRCODE='23514';
 END IF;
 NEW.written_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_disposition_guard BEFORE UPDATE OR DELETE ON game_house_dispositions
 FOR EACH ROW EXECUTE FUNCTION game_house_disposition_guard();

CREATE FUNCTION game_house_disposition_guild_fence_abi() RETURNS INT
 LANGUAGE sql IMMUTABLE AS $$ SELECT 1 $$;
CREATE FUNCTION game_house_disposition_fence_check(w UUID,h TEXT,i UUID,removing BOOLEAN) RETURNS VOID
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_dispositions; guild_present BOOLEAN;
BEGIN
 -- Narrow native-trigger property pin: never wait after an item/provenance lock.
 PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=h FOR SHARE NOWAIT;
 guild_present:=to_regclass('game_house_guild_dispositions') IS NOT NULL OR EXISTS(
  SELECT 1 FROM pg_attribute WHERE attrelid='game_house_ownership'::regclass AND attname='owner_guild_id' AND NOT attisdropped);
 IF guild_present THEN
  IF NOT EXISTS(SELECT 1 FROM pg_proc WHERE oid=to_regprocedure('game_house_guild_disposition_fence_check(uuid,text,uuid,boolean)')
   AND prorettype='void'::regtype AND NOT proretset AND prokind='f') THEN
   RAISE EXCEPTION 'incomplete Guild disposition fence ABI' USING ERRCODE='23514';
  END IF;
  EXECUTE 'SELECT game_house_guild_disposition_fence_check($1,$2,$3,$4)' USING w,h,i,removing;
 END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state IN(2,3);
 IF FOUND AND NOT (removing AND i IS NOT NULL AND EXISTS(
  SELECT 1 FROM game_house_disposition_steps s WHERE s.operation_id=o.operation_id
   AND s.item_instance_id=i AND s.written_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'house disposition content is frozen' USING ERRCODE='55P03';
 END IF;
END $$;
CREATE FUNCTION game_house_disposition_content_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN PERFORM game_house_disposition_fence_check(OLD.world_id,OLD.house_key,OLD.item_instance_id,TG_OP='DELETE'); END IF;
 IF TG_OP<>'DELETE' THEN PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NEW.item_instance_id,FALSE); RETURN NEW; END IF;
 RETURN OLD;
END $$;
CREATE TRIGGER a_house_disposition_content_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_interior_locations
 FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE TRIGGER a_house_disposition_provenance_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_reclaim_provenance
 FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE FUNCTION game_house_disposition_item_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l RECORD;
BEGIN
 SELECT world_id,house_key INTO l FROM game_item_house_interior_locations WHERE item_instance_id=OLD.item_instance_id;
 IF FOUND THEN PERFORM game_house_disposition_fence_check(l.world_id,l.house_key,OLD.item_instance_id,FALSE); END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_item_fence BEFORE UPDATE OR DELETE ON game_item_instances
 FOR EACH ROW EXECUTE FUNCTION game_house_disposition_item_fence();
CREATE FUNCTION game_house_disposition_entry_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.state IN(1,2) THEN PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NULL,FALSE); END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_entry_fence BEFORE INSERT OR UPDATE ON game_house_scope_handoffs
 FOR EACH ROW EXECUTE FUNCTION game_house_disposition_entry_fence();

-- Rust proves recovery, Node possession and full current session fence before calling.
CREATE FUNCTION game_house_move_out_apply(w UUID,h TEXT,c UUID,notice BOOLEAN,due BIGINT,now_ms BIGINT)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE owner_row game_house_ownership; o game_house_dispositions;
BEGIN
 IF w IS NULL OR h IS NULL OR c IS NULL OR notice IS NULL OR now_ms IS NULL THEN RETURN NULL; END IF;
 SELECT * INTO owner_row FROM game_house_ownership WHERE world_id=w AND house_key=h FOR UPDATE;
 IF NOT FOUND OR owner_row.owner_character_id<>c THEN RETURN NULL; END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state<>4 FOR UPDATE;
 IF notice THEN
  IF FOUND OR due IS NULL OR due::numeric-now_ms NOT BETWEEN 86400000 AND 2592000000 THEN RETURN NULL; END IF;
  INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at)
   VALUES(w,h,c,owner_row.ownership_revision,1,now_ms,due) RETURNING * INTO o;
 ELSE
  IF NOT FOUND OR o.state<>1 OR now_ms>=o.due_at OR o.owner_character_id<>c OR o.ownership_revision<>owner_row.ownership_revision THEN RETURN NULL; END IF;
  UPDATE game_house_dispositions SET state=4,cancelled_at=now_ms WHERE operation_id=o.operation_id RETURNING * INTO o;
 END IF;
 RETURN jsonb_build_object('operation',o.operation_id,'state',o.state,'due',o.due_at,'count',0,'delivered',0);
END $$;

CREATE FUNCTION game_house_disposition_delivery_proven(op UUID,i UUID) RETURNS BOOLEAN
 LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_disposition_steps s
 JOIN game_house_dispositions o USING(operation_id)
 JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
 JOIN game_character_inbox_deliveries d ON d.item_instance_id=s.item_instance_id AND d.character_id=s.reclaim_subject
  AND d.world_id=o.world_id AND d.ordinal=s.ordinal AND d.cause_kind='HOUSE_DISPOSITION' AND d.cause_ref=op::text
 JOIN game_item_character_inbox_locations l ON l.item_instance_id=d.item_instance_id AND l.world_id=d.world_id AND l.character_id=d.character_id AND l.ordinal=d.ordinal
 WHERE s.operation_id=op AND s.item_instance_id=i AND s.reclaim_subject=c.reclaim_subject)
$$;
CREATE FUNCTION game_house_disposition_native_removal_proven(old_location JSONB) RETURNS BOOLEAN
 LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_disposition_census c
 JOIN game_house_dispositions o USING(operation_id)
 JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
 JOIN game_item_instances i USING(item_instance_id)
 JOIN game_item_character_inbox_locations l ON l.item_instance_id=i.item_instance_id
 WHERE c.source_location=old_location AND to_jsonb(i)=c.source_item AND s.written_xact_id=pg_current_xact_id()
  AND EXISTS(SELECT 1 FROM game_house_disposition_audit_outbox a WHERE a.operation_id=o.operation_id AND a.written_xact_id=pg_current_xact_id())
  AND o.state IN(2,3) AND game_house_disposition_delivery_proven(o.operation_id,i.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations x WHERE x.item_instance_id=i.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=i.item_instance_id))
$$;
-- Preserve both existing player shape arms byte-for-byte; add only ordinary disposition removal.
CREATE OR REPLACE FUNCTION game_house_location_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.placed_transaction_id
   AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.house_key=NEW.house_key
   AND p.destination_position=NEW.spatial_position AND p.destination_stack_ordinal=NEW.stack_ordinal
   AND p.shape IN(5,7) AND p.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'house insertion lacks own physical shape' USING ERRCODE='23514';END IF;
 ELSIF TG_OP='DELETE' THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.item_instance_id=OLD.item_instance_id
   AND p.world_id=OLD.world_id AND p.house_key=OLD.house_key AND p.source_position=OLD.spatial_position
   AND p.source_stack_ordinal=OLD.stack_ordinal AND p.source_placement=OLD.placed_transaction_id
   AND p.shape IN(6,7) AND p.created_xact_id=pg_current_xact_id())
   AND NOT game_house_disposition_native_removal_proven(to_jsonb(OLD)) THEN
   RAISE EXCEPTION 'house removal lacks actual source shape' USING ERRCODE='23514';END IF;
 ELSE RAISE EXCEPTION 'house relocation requires delete/insert' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;

CREATE FUNCTION game_house_disposition_advance(op UUID,max_census INT,max_step INT,requested_step INT) RETURNS INT
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_dispositions; owner_row game_house_ownership; n INT; x RECORD; ord NUMERIC(20,0); now_ms BIGINT; locked_roots UUID[]:='{}';
BEGIN
 IF max_census IS DISTINCT FROM 2000 OR max_step IS DISTINCT FROM 100 OR requested_step IS NULL THEN RAISE EXCEPTION 'invalid disposition bounds' USING ERRCODE='23514'; END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op;
 IF NOT FOUND THEN RETURN 0; END IF;
 -- Census/entry lock order is shared with the native handoff producer.
 LOCK TABLE game_house_scope_handoffs IN SHARE ROW EXCLUSIVE MODE;
 -- Recipient roots precede property/items/counters. The census bound also bounds this read.
 FOR x IN SELECT DISTINCT p.reclaim_subject_character_id AS id FROM game_item_house_reclaim_provenance p
  WHERE p.world_id=o.world_id AND p.house_key=o.house_key ORDER BY id LIMIT 2001 LOOP
  PERFORM 1 FROM game_character_roots WHERE character_id=x.id FOR UPDATE;
  locked_roots:=array_append(locked_roots,x.id);
 END LOOP;
 SELECT * INTO owner_row FROM game_house_ownership WHERE world_id=o.world_id AND house_key=o.house_key FOR UPDATE;
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE world_id=o.world_id AND house_key=o.house_key
  AND NOT(reclaim_subject_character_id=ANY(locked_roots))) THEN
  RAISE EXCEPTION 'house recipients changed before property lock' USING ERRCODE='40001'; END IF;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF o.state NOT IN(1,2) OR now_ms<o.due_at THEN RETURN 0; END IF;
 IF requested_step=0 AND o.state=2 THEN RETURN 0; END IF;
 IF requested_step<0 OR requested_step>20 THEN RAISE EXCEPTION 'invalid disposition step' USING ERRCODE='23514'; END IF;
 IF owner_row.owner_character_id IS DISTINCT FROM o.owner_character_id OR owner_row.ownership_revision IS DISTINCT FROM o.ownership_revision THEN
  RAISE EXCEPTION 'house disposition ownership changed' USING ERRCODE='23514'; END IF;
 IF o.state=1 THEN
  PERFORM game_house_disposition_fence_check(o.world_id,o.house_key,NULL,FALSE);
  IF requested_step<>0 THEN RAISE EXCEPTION 'freeze step required' USING ERRCODE='23514'; END IF;
  SELECT count(*) INTO n FROM (SELECT item_instance_id FROM game_item_house_interior_locations
   WHERE world_id=o.world_id AND house_key=o.house_key ORDER BY item_instance_id LIMIT 2001) bounded;
  IF n>max_census THEN RAISE EXCEPTION 'house census exceeds 2000' USING ERRCODE='54000'; END IF;
  -- All writers pin the property; after this lock the bounded census cannot change.
  INSERT INTO game_house_disposition_census
   SELECT op,l.item_instance_id,p.reclaim_subject_character_id,to_jsonb(l),to_jsonb(p),to_jsonb(i)
   FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE l.world_id=o.world_id AND l.house_key=o.house_key ORDER BY l.item_instance_id LIMIT 2000;
  IF (SELECT count(*) FROM game_house_disposition_census WHERE operation_id=op)<>n THEN
   RAISE EXCEPTION 'house census missing provenance' USING ERRCODE='23514'; END IF;
  UPDATE game_house_dispositions SET state=2,frozen_at=now_ms,census_count=n,
   census_digest=(SELECT sha256(coalesce(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea))
    FROM game_house_disposition_census WHERE operation_id=op) WHERE operation_id=op;
  PERFORM 1 FROM game_house_bed_revisions WHERE world_id=o.world_id AND house_key=o.house_key FOR UPDATE;
  UPDATE game_house_bed_sleepers SET freed_at=greatest(now_ms,slept_at),freed_cause='disposition'
   WHERE world_id=o.world_id AND house_key=o.house_key AND freed_at IS NULL;
  -- A negative result denotes this transaction's checkpoint without an item delivery.
  RETURN -1;
 END IF;
 IF EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE world_id=o.world_id AND runtime_scope_kind=2
   AND runtime_scope_house_key=o.house_key AND session_state IN(1,2)) OR
    EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=o.world_id AND house_key=o.house_key AND state IN(1,2)) THEN RETURN 0; END IF;
 IF EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=op AND step_number=requested_step) THEN RETURN 0; END IF;
 IF requested_step<>(SELECT coalesce(max(step_number),0)+1 FROM game_house_disposition_steps WHERE operation_id=op) THEN
  RAISE EXCEPTION 'nonconsecutive disposition step' USING ERRCODE='23514'; END IF;
 -- Items then inbox counters in deterministic order, before delivery itself locks a counter.
 FOR x IN SELECT c.item_instance_id FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL ORDER BY c.item_instance_id LIMIT 100 LOOP
  PERFORM 1 FROM game_item_instances WHERE item_instance_id=x.item_instance_id FOR NO KEY UPDATE;
 END LOOP;
 FOR x IN SELECT DISTINCT reclaim_subject AS id FROM (SELECT c.* FROM game_house_disposition_census c
  LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id) WHERE c.operation_id=op AND s.item_instance_id IS NULL
  ORDER BY c.item_instance_id LIMIT 100) batch ORDER BY id LOOP
  INSERT INTO game_character_inbox_counters(character_id) VALUES(x.id) ON CONFLICT DO NOTHING;
  PERFORM 1 FROM game_character_inbox_counters WHERE character_id=x.id FOR UPDATE;
 END LOOP;
 n:=0;
 FOR x IN SELECT c.* FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL ORDER BY c.item_instance_id LIMIT 100 LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id) WHERE l.item_instance_id=x.item_instance_id
   AND to_jsonb(l)=x.source_location AND to_jsonb(p)=x.source_provenance AND to_jsonb(i)=x.source_item) THEN
   RAISE EXCEPTION 'house frozen source changed' USING ERRCODE='23514'; END IF;
  SELECT next_ordinal INTO ord FROM game_character_inbox_counters WHERE character_id=x.reclaim_subject;
  INSERT INTO game_house_disposition_steps VALUES(op,x.item_instance_id,x.reclaim_subject,ord,requested_step,pg_current_xact_id());
  DELETE FROM game_item_house_interior_locations WHERE item_instance_id=x.item_instance_id;
  DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=x.item_instance_id;
  PERFORM game_character_inbox_deliver(x.item_instance_id,x.reclaim_subject,o.world_id,'HOUSE_DISPOSITION',op::text);
  n:=n+1;
 END LOOP;
 IF NOT EXISTS(SELECT 1 FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL) THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=o.world_id AND house_key=o.house_key) THEN
   RAISE EXCEPTION 'house drained source remains' USING ERRCODE='23514'; END IF;
  UPDATE game_house_dispositions SET state=3,drained_at=now_ms WHERE operation_id=op;
  IF n=0 THEN RETURN -1; END IF;
 END IF;
 RETURN n;
END $$;

-- The bounded runtime entry cannot commit a new command or checkpoint without
-- the registered event in the same physical transaction. Native removal proves
-- the same requirement independently for every delivered item.
CREATE FUNCTION game_house_disposition_event_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_disposition_audit_outbox WHERE operation_id=NEW.operation_id AND written_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'house disposition lacks own physical HouseOperation event' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_disposition_command_event AFTER INSERT ON game_house_disposition_commands
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_event_proven();
CREATE CONSTRAINT TRIGGER game_house_disposition_checkpoint_event AFTER UPDATE ON game_house_dispositions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_event_proven();

-- Protected immutable history; no user-controlled receipt or snapshot is a mutation permit.
DO $$ DECLARE t TEXT; f RECORD; BEGIN
 FOREACH t IN ARRAY ARRAY['game_house_disposition_commands','game_house_disposition_census','game_house_disposition_steps'] LOOP
  EXECUTE format('CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_house_dispositions','game_house_disposition_commands','game_house_disposition_census','game_house_disposition_steps'] LOOP
  EXECUTE format('CREATE TRIGGER no_truncate BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',t);
 END LOOP;
 FOR f IN SELECT p.oid::regprocedure AS sig FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
  WHERE n.nspname=current_schema() AND (p.proname LIKE 'game_house_disposition_%' OR p.proname='game_house_move_out_apply' OR p.proname='game_house_location_proven') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',f.sig,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f.sig);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_disposition_guild_fence_abi() TO oteryn_game_runtime;

-- Runtime ingress mirrors 0095's possession-bound native ports. Private producer functions,
-- manifests and steps retain no runtime/control grant; the port independently proves scope
-- and session authority, then calls them within the caller's sealed recovery transaction.
CREATE TABLE game_house_disposition_job_cursors(world_id UUID PRIMARY KEY,last_due BIGINT,last_operation UUID);
REVOKE ALL ON game_house_disposition_job_cursors FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_move_out_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID; ch UUID; sk BYTEA; o game_house_dispositions; prior game_house_disposition_commands;
 c UUID; sid UUID; cid NUMERIC; a INT; due BIGINT; now_ms BIGINT; result JSONB; binding BYTEA;
 ids JSONB; step INT; moved INT; cur game_house_disposition_job_cursors; last_row RECORD;
BEGIN
 IF mode NOT IN('command','query','candidates','advance','audit_snapshot','audit_insert') OR p IS NULL OR jsonb_typeof(p)<>'object'
  OR octet_length(p::text)>(CASE WHEN mode='audit_insert' THEN 300000 ELSE 2048 END)
  OR NOT p ?& ARRAY['world','channel','generation']
  OR EXISTS(SELECT 1 FROM jsonb_object_keys(p) k WHERE k NOT IN('world','channel','generation','house','session','character','command','connection','lease','action','due','operation','event','now','envelope'))
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'house disposition ingress refused' USING ERRCODE='23514'; END IF;
 w:=(p->>'world')::uuid;ch:=(p->>'channel')::uuid;sk:='\x01'::bytea||uuid_send(w)||uuid_send(ch);
 IF mode='command' THEN
  sid:=(p->>'session')::uuid;cid:=(p->>'command')::numeric;binding:=convert_to(p::text,'UTF8');
  IF sid IS NULL OR cid IS NULL OR cid NOT BETWEEN 1 AND 18446744073709551615 THEN RAISE EXCEPTION 'invalid command' USING ERRCODE='23514'; END IF;
  PERFORM pg_advisory_xact_lock(hashtextextended(sid::text||':'||cid::text,178));
  SELECT * INTO prior FROM game_house_disposition_commands WHERE game_session_id=sid AND command_id=cid;
  IF FOUND THEN
   IF prior.binding<>binding THEN RETURN jsonb_build_object('conflict',TRUE); END IF;
   RETURN prior.outcome;
  END IF;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN game_durability_admission_runtime_guards rg USING(scope_key)
  WHERE ra.scope_key=sk AND ra.scope_kind=1 AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
   AND ra.ownership_generation=(p->>'generation')::numeric AND ra.holder_node_id=n AND ra.holder_registration_revision=nr
   AND rg.ready AND rg.ownership_generation=ra.ownership_generation FOR SHARE OF ra,rg) THEN RETURN NULL; END IF;
 IF mode IN('command','query') THEN
  sid:=(p->>'session')::uuid;c:=(p->>'character')::uuid;
  IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id)
    JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id
    JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
    WHERE s.game_session_id=sid AND s.character_id=c AND s.account_id=r.account_id AND s.world_id=w AND r.world_id=w AND r.lifecycle=1
     AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=w AND s.runtime_scope_channel_id=ch
     AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
     AND s.current_generation=(p->>'connection')::numeric AND s.character_lease_generation=(p->>'lease')::numeric
     AND s.scope_ownership_generation=(p->>'generation')::numeric AND s.session_state IN(1,2)
     AND cg.eligible AND cg.account_id=r.account_id AND cg.world_id=w AND cg.lease_generation=s.character_lease_generation
     AND cg.holder_game_session_id=sid AND ag.presence_character_id=c AND ag.holder_game_session_id=sid
     FOR SHARE OF s,r,cg,ag) THEN RETURN NULL; END IF;
  -- Root before property. Query pins ownership too, preventing a concurrent owner change
  -- between its authorization decision and private projection.
  PERFORM 1 FROM game_character_roots WHERE character_id=c FOR UPDATE;
  PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=p->>'house' AND owner_character_id=c FOR UPDATE;
  IF NOT FOUND THEN RETURN NULL; END IF;
  IF mode='query' THEN
   IF (p->>'action')::int IS DISTINCT FROM 0 OR (p->>'due')::bigint IS DISTINCT FROM 0 THEN RETURN NULL; END IF;
   SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=p->>'house'
    ORDER BY (state<>4) DESC,noticed_at DESC,operation_id DESC LIMIT 1;
   IF NOT FOUND THEN RETURN jsonb_build_object('state',0); END IF;
   RETURN jsonb_build_object('operation',o.operation_id,'state',o.state,'due',o.due_at,'count',coalesce(o.census_count,0),
    'delivered',(SELECT count(*) FROM game_house_disposition_steps WHERE operation_id=o.operation_id));
  END IF;
  a:=(p->>'action')::int;due:=(p->>'due')::bigint;
  IF a IS NULL OR a NOT IN(1,2) OR due IS NULL OR (a=2 AND due<>0) THEN RETURN NULL; END IF;
  now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  result:=game_house_move_out_apply(w,p->>'house',c,a=1,CASE WHEN a=1 THEN due END,now_ms);
  IF result IS NULL THEN RETURN NULL; END IF;
  result:=result||jsonb_build_object('now',now_ms,'new',FALSE);
  INSERT INTO game_house_disposition_commands VALUES(sid,cid,binding,(result->>'operation')::uuid,result);
  RETURN result||jsonb_build_object('new',TRUE);
 ELSIF mode='candidates' THEN
  INSERT INTO game_house_disposition_job_cursors(world_id) VALUES(w) ON CONFLICT DO NOTHING;
  SELECT * INTO cur FROM game_house_disposition_job_cursors WHERE world_id=w FOR UPDATE;
  SELECT coalesce(jsonb_agg(operation_id ORDER BY due_at,operation_id),'[]'::jsonb) INTO ids FROM (
   SELECT operation_id,due_at FROM game_house_dispositions WHERE world_id=w AND state IN(1,2)
    AND due_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
    AND (cur.last_operation IS NULL OR (due_at,operation_id)>(cur.last_due,cur.last_operation))
    ORDER BY due_at,operation_id LIMIT 100) candidates;
  IF jsonb_array_length(ids)=0 AND cur.last_operation IS NOT NULL THEN
   SELECT coalesce(jsonb_agg(operation_id ORDER BY due_at,operation_id),'[]'::jsonb) INTO ids FROM (
    SELECT operation_id,due_at FROM game_house_dispositions WHERE world_id=w AND state IN(1,2)
     AND due_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint ORDER BY due_at,operation_id LIMIT 100) wrapped;
  END IF;
  IF jsonb_array_length(ids)=0 THEN UPDATE game_house_disposition_job_cursors SET last_due=NULL,last_operation=NULL WHERE world_id=w;
  ELSE
   SELECT due_at,operation_id INTO last_row FROM game_house_dispositions WHERE operation_id=(ids->>-1)::uuid;
   UPDATE game_house_disposition_job_cursors SET last_due=last_row.due_at,last_operation=last_row.operation_id WHERE world_id=w;
  END IF;
  RETURN ids;
 END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND operation_id=(p->>'operation')::uuid;
 IF NOT FOUND THEN RETURN NULL; END IF;
 IF mode='advance' THEN
  step:=CASE WHEN o.state=1 THEN 0 ELSE (SELECT coalesce(max(step_number),0)+1 FROM game_house_disposition_steps WHERE operation_id=o.operation_id) END;
  moved:=game_house_disposition_advance(o.operation_id,2000,100,step);
  SELECT * INTO o FROM game_house_dispositions WHERE operation_id=o.operation_id;
  RETURN to_jsonb(moved<>0);
 END IF;
 IF o.written_xact_id<>pg_current_xact_id() AND NOT EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=o.operation_id AND written_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'audit requires own physical disposition mutation' USING ERRCODE='23514'; END IF;
 IF mode='audit_snapshot' THEN
  RETURN jsonb_build_object('operation',o.operation_id,'world',w,'house',o.house_key,'owner',o.owner_character_id,
   'state',o.state,'due',o.due_at,'count',coalesce(o.census_count,0),'digest',encode(o.census_digest,'hex'),'event',game_character_uuid_v7(),
   'now',floor(extract(epoch FROM clock_timestamp())*1000)::bigint,'transfers',
   (SELECT coalesce(jsonb_agg(jsonb_build_object('item',item_instance_id,'recipient',reclaim_subject,'ordinal',ordinal::text) ORDER BY item_instance_id),'[]'::jsonb)
    FROM (SELECT * FROM game_house_disposition_steps WHERE operation_id=o.operation_id AND written_xact_id=pg_current_xact_id() ORDER BY item_instance_id LIMIT 101) batch));
 END IF;
 IF mode='audit_insert' THEN
  INSERT INTO game_house_disposition_audit_outbox(event_id,operation_id,envelope,occurred_at,expires_at,envelope_sha256)
   VALUES((p->>'event')::uuid,o.operation_id,decode(p->>'envelope','hex'),(p->>'now')::bigint,(p->>'now')::bigint+2592000000,sha256(decode(p->>'envelope','hex')));
  RETURN 'true'::jsonb;
 END IF;
 RAISE EXCEPTION 'unknown house disposition ingress' USING ERRCODE='23514';
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_move_out_port(TEXT,JSONB,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_move_out_port(TEXT,JSONB,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_move_out_port(TEXT,JSONB,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
