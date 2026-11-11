-- WU26-06 Temple Teleport Scroll (docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md).
-- The native item-use writer burns one scroll as ItemUseCause 481 with one paired
-- immutable use row. The daily claim gives each delivered scroll a seven-day
-- absolute deadline on the existing Item deadline owner
-- (game_spell_item_temporal_schedules, no decay target), whose Channel pass
-- removes the scroll when due exactly as it removes every other expiring Item.
-- This migration admits that deadline's daily-delivery provenance, the scroll's
-- movement with its deadline and its removal wherever it is, for an online or an
-- offline Character: expiry applies in all custodies (section 7; session-less
-- receipt custody 2, 3, 4 Inbox root, 5 depot root). All prior item-use causes and
-- shapes, deadline sources, custody kinds and movement guards stay admitted
-- unchanged.
--
-- Rollback: applied migrations are immutable. A successor stops new writes (the
-- dispatch branch, the claim's deadline insert and the INSERT grant) and keeps
-- every row as evidence of a burn, a deadline or a removal.

-- 1. The deadline proof finds a scroll's delivery line by its Item.
CREATE INDEX game_character_daily_reward_lines_item
 ON game_character_daily_reward_lines(item_instance_id) WHERE item_instance_id IS NOT NULL;

-- 2. ItemUseCause 481 TempleScroll: a burn only.
DO $$
DECLARE
 constraint_name TEXT;
 previous_rule TEXT;
BEGIN
 FOREACH constraint_name IN ARRAY ARRAY[
   'game_item_use_records_cause_kind_check','game_item_use_cause_shape'
 ] LOOP
   SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT previous_rule
     FROM pg_constraint c
    WHERE c.conrelid='game_item_use_records'::regclass
      AND c.conname=constraint_name AND c.contype='c';
   EXECUTE format(
     'ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind=481 AND shape=1))',
     constraint_name,constraint_name,previous_rule);
 END LOOP;
END $$;

-- 3. One use row per burn. The stamp, not the caller, reads the scroll's deadline
-- and the database clock. A due scroll is never used: the deadline pass removes
-- it, and a use racing that pass fails this row's CHECK (defensive refusal).
CREATE TABLE game_temple_scroll_uses (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id))
   REFERENCES game_item_use_records(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 -- NULL only for a scroll delivered before this migration: it has no deadline.
 expires_at_unix_ms BIGINT CHECK(expires_at_unix_ms>0),
 observed_at_unix_ms BIGINT NOT NULL CHECK(observed_at_unix_ms>0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(expires_at_unix_ms IS NULL OR observed_at_unix_ms<expires_at_unix_ms)
);
CREATE INDEX game_temple_scroll_uses_character ON game_temple_scroll_uses(character_id);
CREATE TRIGGER game_temple_scroll_uses_immutable BEFORE UPDATE OR DELETE ON game_temple_scroll_uses
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_temple_scroll_uses_no_truncate BEFORE TRUNCATE ON game_temple_scroll_uses
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_temple_scroll_use_stamp() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 NEW.observed_at_unix_ms:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 NEW.expires_at_unix_ms:=(SELECT p.expires_at_unix_ms FROM public.game_spell_item_temporal_schedules p
   WHERE p.item_instance_id=NEW.item_instance_id AND p.target_family IS NULL);
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_temple_scroll_uses_stamp BEFORE INSERT ON game_temple_scroll_uses
 FOR EACH ROW EXECUTE FUNCTION game_temple_scroll_use_stamp();

-- 4. The burn and its row commit together: the same transaction, Character and scroll.
CREATE FUNCTION game_temple_scroll_use_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s public.game_temple_scroll_uses%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_temple_scroll_uses' THEN s:=NEW;
 ELSE
   SELECT * INTO s FROM public.game_temple_scroll_uses WHERE transaction_id=NEW.transaction_id;
   IF s.transaction_id IS NULL THEN
     RAISE EXCEPTION 'temple scroll burn lacks its use row' USING ERRCODE='23514';
   END IF;
 END IF;
 IF NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u WHERE u.transaction_id=s.transaction_id
    AND u.cause_kind=481 AND u.shape=1 AND u.created_xact_id=s.created_xact_id
    AND u.character_id=s.character_id AND u.source_item_instance_id=s.item_instance_id
    AND u.source_definition_family='Item'
    AND u.source_definition_production_key='oteryn:item.tibia.i25718'
 ) THEN RAISE EXCEPTION 'temple scroll use row differs from its burn' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_temple_scroll_use_consistent AFTER INSERT ON game_temple_scroll_uses
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_temple_scroll_use_consistent();
CREATE CONSTRAINT TRIGGER game_temple_scroll_burn_consistent AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=481)
 EXECUTE FUNCTION game_temple_scroll_use_consistent();

REVOKE ALL ON game_temple_scroll_uses FROM PUBLIC;
REVOKE ALL ON FUNCTION game_temple_scroll_use_stamp() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_temple_scroll_use_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_temple_scroll_uses TO oteryn_game_runtime;
GRANT SELECT ON game_temple_scroll_uses TO oteryn_game_control;

-- 5. The deadline: a third original source of the existing absolute Item deadline
-- owner, beside the spell and action mints. The claim that delivers the scroll
-- schedules it in the same transaction, 604,800,000 ms after that transaction's
-- start, with no decay target, so the scroll vanishes when due.
CREATE FUNCTION game_temple_scroll_schedule_proven(s game_spell_item_temporal_schedules)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF s.definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i25718' THEN RETURN FALSE; END IF;
 IF s.definition_family<>'Item' OR s.definition_revision<>'definition-r1'
  OR num_nonnulls(s.target_family,s.target_production_key,s.target_revision)<>0
  OR s.source_ordinal<>1 OR s.duration_millis<>604800000
  OR s.expires_at_unix_ms<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint+s.duration_millis
  OR s.content_digest<>sha256(convert_to(s.definition_production_key||'|'||s.definition_revision,'UTF8'))
  OR s.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM public.game_item_instances i
    WHERE i.item_instance_id=s.item_instance_id AND i.lifecycle=1 AND i.quantity=1
      AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
        =(s.definition_family,s.definition_production_key,s.definition_revision))
  OR NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_lines l
    WHERE l.item_instance_id=s.item_instance_id AND l.kind='TEMPLE_SCROLL' AND l.status='DELIVERED'
      AND l.delivering_transaction_id=s.source_transaction_id) THEN
  RAISE EXCEPTION 'temple scroll deadline requires its same-transaction daily delivery'
   USING ERRCODE='23514';
 END IF;
 RETURN TRUE;
END $$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
 EXECUTE overlay(body placing
  E'BEGIN\n IF game_temple_scroll_schedule_proven(NEW) THEN RETURN NULL; END IF;'
  from begin_at for 5);
END $migration$;

-- 6. The scroll moves with its immutable deadline (Inbox retrieval, carried tree,
-- depot), as an action-minted Item does; every other scheduled Item keeps its guard.
CREATE FUNCTION game_temple_scroll_moves(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT EXISTS(SELECT 1 FROM public.game_spell_item_temporal_schedules p
  WHERE p.item_instance_id=p_item AND p.definition_family='Item'
    AND p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL)
$$;

DO $migration$
DECLARE body TEXT;
 original CONSTANT TEXT:='EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)';
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'unexpected absolute Item timer movement guard count'; END IF;
 EXECUTE replace(body,original,'('||original||' AND NOT game_temple_scroll_moves(NEW.item_instance_id))');
END $migration$;

-- 7. Expiry applies in every custody the scroll can occupy, for an online or an
-- offline Character. The Channel pass keeps retiring a scroll that its current
-- session carries (Ground kind 1, carried kinds 2 and 3, unchanged). Every other
-- due scroll (in the Inbox or a depot, in a container rooted there, or carried by
-- a Character with no live session) is retired by game_temple_scroll_expire_due
-- under no session: receipt custody 2 carried container entry, 3 equipment,
-- 4 Inbox root, 5 depot root (box or locker). The Supply Stash holds stackable
-- balances only and refuses scheduled Items (0189), as do House containers (1169)
-- and the market (0191), so the scroll never enters them.
DO $migration$
DECLARE c RECORD; n INTEGER:=0;
 arm CONSTANT TEXT:='(character_id IS NOT NULL'
  ||' AND num_nonnulls(game_session_id,character_lease_generation,connection_generation)=0'
  ||' AND ((custody_kind=2 AND source_parent IS NOT NULL AND source_ordinal>0'
  ||'   AND num_nonnulls(source_slot,equipment_revision_before,source_stack_ordinal)=0)'
  ||'  OR (custody_kind=3 AND source_slot BETWEEN 1 AND 10 AND source_slot<>9'
  ||'   AND equipment_revision_before BETWEEN 1 AND 18446744073709551614'
  ||'   AND num_nonnulls(source_parent,source_ordinal,source_stack_ordinal)=0)'
  ||'  OR (custody_kind IN(4,5) AND source_ordinal>0'
  ||'   AND num_nonnulls(source_parent,source_slot,equipment_revision_before,source_stack_ordinal)=0))) IS TRUE';
BEGIN
 FOR c IN SELECT conname,pg_get_expr(conbin,conrelid) AS rule FROM pg_constraint
   WHERE conrelid='game_spell_item_temporal_receipts'::regclass AND contype='c'
     AND pg_get_expr(conbin,conrelid) LIKE '%custody_kind%' LOOP
  EXECUTE format('ALTER TABLE game_spell_item_temporal_receipts DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (%s))',
   c.conname,c.conname,c.rule,arm);
  n:=n+1;
 END LOOP;
 IF n<>2 THEN RAISE EXCEPTION 'unexpected Item timer custody constraints: %',n; END IF;
END $migration$;

-- The scroll's one current custody: the direct location row, its Character (for a
-- nested Item, the Character of the chain's root) and whether that root is stored
-- (Inbox or depot) rather than carried. No row: the scroll is on Ground or in a
-- custody without a Character, which the Channel's own pass or no pass hosts.
CREATE FUNCTION game_temple_scroll_custody(p_item UUID)
RETURNS TABLE(kind SMALLINT,character_id UUID,parent UUID,ordinal NUMERIC,slot SMALLINT,
 equipment_revision NUMERIC,stored BOOLEAN)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 WITH RECURSIVE up(item,depth) AS (
   SELECT n.parent_item_instance_id,1 FROM public.game_item_nested_entries n WHERE n.item_instance_id=p_item
   UNION ALL
   SELECT n.parent_item_instance_id,u.depth+1 FROM up u
    JOIN public.game_item_nested_entries n ON n.item_instance_id=u.item WHERE u.depth<16
 ), root AS (SELECT item FROM up ORDER BY depth DESC LIMIT 1),
 owner AS (
   SELECT coalesce(
     (SELECT s.character_id FROM public.game_item_container_slots s JOIN root ON root.item=s.item_instance_id),
     (SELECT e.character_id FROM public.game_item_container_entries e JOIN root ON root.item=e.item_instance_id),
     (SELECT q.character_id FROM public.game_character_equipment_slots q JOIN root ON root.item=q.item_instance_id),
     (SELECT x.character_id FROM public.game_item_character_inbox_locations x JOIN root ON root.item=x.item_instance_id),
     (SELECT d.character_id FROM public.game_item_character_depot_locations d JOIN root ON root.item=d.item_instance_id)
   ) AS character_id,
   EXISTS(SELECT 1 FROM public.game_item_character_inbox_locations x JOIN root ON root.item=x.item_instance_id)
   OR EXISTS(SELECT 1 FROM public.game_item_character_depot_locations d JOIN root ON root.item=d.item_instance_id) AS stored
 )
 SELECT 2::smallint,e.character_id,e.parent_item_instance_id,e.placement_ordinal::numeric,NULL::smallint,NULL::numeric,false
   FROM public.game_item_container_entries e WHERE e.item_instance_id=p_item
 UNION ALL
 SELECT 2::smallint,o.character_id,n.parent_item_instance_id,n.placement_ordinal::numeric,NULL::smallint,NULL::numeric,o.stored
   FROM public.game_item_nested_entries n,owner o WHERE n.item_instance_id=p_item AND o.character_id IS NOT NULL
 UNION ALL
 SELECT 3::smallint,q.character_id,NULL::uuid,NULL::numeric,q.slot::smallint,st.revision::numeric,false
   FROM public.game_character_equipment_slots q JOIN public.game_character_equipment_state st USING(character_id)
  WHERE q.item_instance_id=p_item
 UNION ALL
 SELECT 4::smallint,x.character_id,NULL::uuid,x.ordinal::numeric,NULL::smallint,NULL::numeric,true
   FROM public.game_item_character_inbox_locations x WHERE x.item_instance_id=p_item
 UNION ALL
 SELECT 5::smallint,d.character_id,NULL::uuid,d.ordinal::numeric,NULL::smallint,NULL::numeric,true
   FROM public.game_item_character_depot_locations d WHERE d.item_instance_id=p_item
$$;

-- A Character is live while a reconnect session is active or reconnectable.
CREATE FUNCTION game_temple_scroll_owner_live(p_character UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT EXISTS(SELECT 1 FROM public.game_durability_reconnect_sessions s
  WHERE s.character_id=p_character AND s.session_state IN(1,2))
$$;

-- The predecessor of a session-less removal: the exact due scroll in the exact
-- custody the receipt names, stored or of an offline Character of this World.
CREATE FUNCTION game_temple_scroll_capture(o game_item_instances,p_transaction UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE
 r public.game_spell_item_temporal_receipts%ROWTYPE;
 p public.game_spell_item_temporal_schedules%ROWTYPE;
BEGIN
 SELECT * INTO r FROM public.game_spell_item_temporal_receipts WHERE transaction_id=p_transaction
   AND item_instance_id=o.item_instance_id AND created_xact_id=pg_current_xact_id()
   AND game_session_id IS NULL AND custody_kind IN(2,3,4,5);
 SELECT * INTO p FROM public.game_spell_item_temporal_schedules WHERE item_instance_id=o.item_instance_id;
 IF r.transaction_id IS NULL OR p.item_instance_id IS NULL
  OR o.lifecycle<>1 OR o.world_id<>r.world_id OR o.quantity<>r.quantity_before
  OR o.state_revision<>r.state_revision_before
  OR (o.definition_family,o.definition_production_key,o.definition_revision_ref)
    <>(p.definition_family,p.definition_production_key,p.definition_revision)
  OR p.definition_production_key<>'oteryn:item.tibia.i25718' OR p.target_family IS NOT NULL
  OR r.occurred_at_unix_ms<p.expires_at_unix_ms
  OR NOT EXISTS(SELECT 1 FROM public.game_character_roots c
    WHERE c.character_id=r.character_id AND c.world_id=r.world_id)
  OR (SELECT count(*) FROM public.game_temple_scroll_custody(o.item_instance_id))<>1
  OR NOT EXISTS(SELECT 1 FROM public.game_temple_scroll_custody(o.item_instance_id) k
    WHERE k.kind=r.custody_kind AND k.character_id=r.character_id
      AND k.parent IS NOT DISTINCT FROM r.source_parent AND k.ordinal IS NOT DISTINCT FROM r.source_ordinal
      AND k.slot IS NOT DISTINCT FROM r.source_slot
      AND k.equipment_revision IS NOT DISTINCT FROM r.equipment_revision_before
      AND (k.stored OR NOT public.game_temple_scroll_owner_live(r.character_id))) THEN
  RAISE EXCEPTION 'temple scroll timer lacks its exact due predecessor custody' USING ERRCODE='23514';
 END IF;
 INSERT INTO public.game_spell_item_temporal_before VALUES(r.transaction_id,o.item_instance_id,
  o.state_revision,o.quantity,o.definition_family,o.definition_production_key,o.definition_revision_ref,
  r.custody_kind,r.character_id,r.source_parent,r.source_ordinal,r.source_slot,r.source_stack_ordinal,
  pg_current_xact_id());
END $$;

-- The outcome of a session-less removal, in place of the session-bound proof: the
-- same source, predecessor, audit and retirement, and no custody left anywhere.
CREATE FUNCTION game_temple_scroll_receipt_proven(r game_spell_item_temporal_receipts)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE p public.game_spell_item_temporal_schedules%ROWTYPE; i public.game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO p FROM public.game_spell_item_temporal_schedules WHERE item_instance_id=r.item_instance_id;
 SELECT * INTO i FROM public.game_item_instances WHERE item_instance_id=r.item_instance_id;
 IF p.item_instance_id IS NULL OR i.item_instance_id IS NULL
  OR p.definition_production_key<>'oteryn:item.tibia.i25718' OR p.target_family IS NOT NULL
  OR r.occurred_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
  OR r.created_xact_id<>pg_current_xact_id() OR r.occurred_at_unix_ms<p.expires_at_unix_ms
  OR i.last_transaction_id<>r.transaction_id OR i.state_revision<>r.state_revision_before+1
  OR i.lifecycle<>2 OR i.quantity<>0
  OR NOT EXISTS(SELECT 1 FROM public.game_spell_item_temporal_before b WHERE b.transaction_id=r.transaction_id
    AND b.item_instance_id=r.item_instance_id AND b.state_revision_before=r.state_revision_before
    AND b.quantity_before=r.quantity_before AND b.custody_kind=r.custody_kind AND b.created_xact_id=r.created_xact_id)
  OR NOT EXISTS(SELECT 1 FROM public.game_spell_item_temporal_audit a WHERE a.transaction_id=r.transaction_id
    AND a.event_id=r.event_id AND a.created_xact_id=r.created_xact_id
    AND convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p)))
  OR (r.custody_kind=3 AND NOT EXISTS(SELECT 1 FROM public.game_character_equipment_state e
    WHERE e.character_id=r.character_id AND e.revision=r.equipment_revision_before+1
      AND e.last_transaction_id=r.transaction_id))
  OR EXISTS(SELECT 1 FROM public.game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_container_entries WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_nested_entries WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_character_equipment_slots WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_character_inbox_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_character_depot_locations WHERE item_instance_id=r.item_instance_id) THEN
  RAISE EXCEPTION 'temple scroll timer outcome requires exact source/custody/audit/retirement'
   USING ERRCODE='23514';
 END IF;
END $$;

-- The session-less pass: each due scroll this pass may retire, at most p_limit.
CREATE FUNCTION game_temple_scroll_expire_due(p_world UUID,p_channel UUID,p_generation NUMERIC,p_limit INTEGER)
RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s RECORD; k RECORD; tx UUID; ev UUID; done INTEGER:=0; removed BIGINT; envelope BYTEA;
BEGIN
 IF p_limit IS NULL OR p_limit<=0 THEN RETURN 0; END IF;
 FOR s IN SELECT i.item_instance_id,i.state_revision FROM public.game_spell_item_temporal_schedules p
   JOIN public.game_item_instances i USING(item_instance_id)
   WHERE p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.lifecycle=1 AND i.quantity=1 AND i.world_id=p_world
     AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =(p.definition_family,p.definition_production_key,p.definition_revision)
     AND p.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
     AND NOT EXISTS(SELECT 1 FROM public.game_spell_item_temporal_receipts d WHERE d.item_instance_id=i.item_instance_id)
   ORDER BY p.expires_at_unix_ms,i.item_instance_id LIMIT 500
   FOR UPDATE OF i,p SKIP LOCKED
 LOOP
  EXIT WHEN done>=p_limit;
  IF (SELECT count(*) FROM public.game_temple_scroll_custody(s.item_instance_id))<>1 THEN CONTINUE; END IF;
  SELECT * INTO k FROM public.game_temple_scroll_custody(s.item_instance_id);
  IF NOT EXISTS(SELECT 1 FROM public.game_character_roots c WHERE c.character_id=k.character_id AND c.world_id=p_world)
   OR (NOT k.stored AND public.game_temple_scroll_owner_live(k.character_id)) THEN CONTINUE; END IF;
  tx:=public.game_character_uuid_v7(); ev:=public.game_character_uuid_v7();
  INSERT INTO public.game_spell_item_temporal_receipts(transaction_id,event_id,item_instance_id,world_id,channel_id,
    ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,character_id,
    source_parent,source_ordinal,source_slot,equipment_revision_before)
   VALUES(tx,ev,s.item_instance_id,p_world,p_channel,p_generation,s.state_revision,1,
    floor(extract(epoch FROM statement_timestamp())*1000)::bigint,k.kind,k.character_id,
    k.parent,k.ordinal,k.slot,k.equipment_revision);
  UPDATE public.game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=tx
   WHERE item_instance_id=s.item_instance_id AND state_revision=s.state_revision AND lifecycle=1;
  IF NOT FOUND THEN RAISE EXCEPTION 'temple scroll timer compare-and-change' USING ERRCODE='23514'; END IF;
  CASE k.kind
   WHEN 2 THEN
    DELETE FROM public.game_item_container_entries WHERE item_instance_id=s.item_instance_id
      AND parent_item_instance_id=k.parent AND placement_ordinal=k.ordinal;
    GET DIAGNOSTICS removed=ROW_COUNT;
    IF removed=0 THEN
     DELETE FROM public.game_item_nested_entries WHERE item_instance_id=s.item_instance_id
       AND parent_item_instance_id=k.parent AND placement_ordinal=k.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
    END IF;
   WHEN 3 THEN
    DELETE FROM public.game_character_equipment_slots WHERE item_instance_id=s.item_instance_id
      AND character_id=k.character_id AND slot=k.slot;
    GET DIAGNOSTICS removed=ROW_COUNT;
    UPDATE public.game_character_equipment_state SET revision=revision+1,last_transaction_id=tx
     WHERE character_id=k.character_id AND revision=k.equipment_revision;
    IF NOT FOUND THEN RAISE EXCEPTION 'temple scroll timer equipment epoch' USING ERRCODE='23514'; END IF;
   WHEN 4 THEN
    DELETE FROM public.game_item_character_inbox_locations WHERE item_instance_id=s.item_instance_id
      AND character_id=k.character_id AND ordinal=k.ordinal;
    GET DIAGNOSTICS removed=ROW_COUNT;
   ELSE
    DELETE FROM public.game_item_character_depot_locations WHERE item_instance_id=s.item_instance_id
      AND character_id=k.character_id AND ordinal=k.ordinal;
    GET DIAGNOSTICS removed=ROW_COUNT;
  END CASE;
  IF removed<>1 THEN RAISE EXCEPTION 'temple scroll timer exact custody removal' USING ERRCODE='23514'; END IF;
  SELECT convert_to(jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p))::text,'UTF8') INTO envelope
    FROM public.game_spell_item_temporal_receipts r JOIN public.game_spell_item_temporal_schedules p USING(item_instance_id)
   WHERE r.transaction_id=tx;
  INSERT INTO public.game_spell_item_temporal_audit(event_id,transaction_id,envelope,envelope_sha256)
   VALUES(ev,tx,envelope,sha256(envelope));
  done:=done+1;
 END LOOP;
 RETURN done;
END $$;

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_capture()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer predecessor capture'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts scroll_receipt
   WHERE scroll_receipt.transaction_id=NEW.last_transaction_id
     AND scroll_receipt.item_instance_id=NEW.item_instance_id
     AND scroll_receipt.created_xact_id=pg_current_xact_id()
     AND scroll_receipt.game_session_id IS NULL AND scroll_receipt.custody_kind IN(2,3,4,5)) THEN
  PERFORM game_temple_scroll_capture(OLD,NEW.last_transaction_id);
  RETURN NEW;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_spell_item_temporal_receipt_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer outcome proof'; END IF;
 arm:=$arm$
 IF NEW.game_session_id IS NULL AND NEW.custody_kind IN(2,3,4,5) THEN
  PERFORM game_temple_scroll_receipt_proven(NEW);
  RETURN NULL;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 -- The Inbox and depot rows leave with their exact retired scroll; the depot
 -- keeps its box counter as a depot transfer does.
 body:=pg_get_functiondef('game_character_inbox_remove()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Inbox removal guard'; END IF;
 arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
   JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE r.custody_kind=4 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
     AND r.character_id=OLD.character_id AND r.source_ordinal=OLD.ordinal
     AND r.created_xact_id=pg_current_xact_id()
     AND p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
     AND i.lifecycle=2 AND i.quantity=0) THEN
  PERFORM 1 FROM game_character_roots WHERE character_id=OLD.character_id FOR UPDATE;
  RETURN OLD;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_character_depot_remove()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing depot removal guard'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
   JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE r.custody_kind=5 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
     AND r.character_id=OLD.character_id AND r.source_ordinal=OLD.ordinal
     AND r.created_xact_id=pg_current_xact_id()
     AND p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
     AND i.lifecycle=2 AND i.quantity=0) THEN
  PERFORM 1 FROM game_character_roots WHERE character_id=OLD.character_id FOR UPDATE;
  UPDATE game_character_depot_box_counters SET committed=committed-1
   WHERE character_id=OLD.character_id AND town_id=OLD.town_id AND box_number=OLD.box_number;
  RETURN OLD;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

REVOKE ALL ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_moves(UUID),
 game_temple_scroll_custody(UUID),
 game_temple_scroll_owner_live(UUID),
 game_temple_scroll_capture(game_item_instances,UUID),
 game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts),
 game_temple_scroll_expire_due(UUID,UUID,NUMERIC,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts),
 game_temple_scroll_expire_due(UUID,UUID,NUMERIC,INTEGER) TO oteryn_game_runtime;
