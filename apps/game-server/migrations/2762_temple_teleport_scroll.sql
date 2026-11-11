-- WU26-06 Temple Teleport Scroll (docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md).
-- The native item-use writer burns one scroll as ItemUseCause 481 with one paired
-- immutable use row. The daily claim gives each delivered scroll a seven-day
-- absolute deadline on the existing Item deadline owner
-- (game_spell_item_temporal_schedules, no decay target), whose Channel pass
-- removes the scroll when due exactly as it removes every other expiring Item.
-- This migration admits that deadline's daily-delivery provenance, the scroll's
-- movement with its deadline and its removal from the Character Inbox (receipt
-- custody kind 4). All prior item-use causes and shapes, deadline sources,
-- custody kinds and movement guards stay admitted unchanged.
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

-- 7. The deadline pass also removes a due scroll from its Character's Inbox root
-- (custody kind 4), under the same current Character/session/scope proof as the
-- carried kinds 2 and 3.
DO $migration$
DECLARE c RECORD; n INTEGER:=0;
 arm CONSTANT TEXT:='(custody_kind=4 AND character_id IS NOT NULL AND game_session_id IS NOT NULL'
  ||' AND character_lease_generation>0 AND connection_generation>0 AND source_ordinal>0'
  ||' AND num_nonnulls(source_parent,source_slot,equipment_revision_before,source_stack_ordinal)=0) IS TRUE';
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

-- The predecessor of an Inbox removal: the exact due scroll at its Inbox ordinal.
CREATE FUNCTION game_temple_scroll_inbox_capture(o game_item_instances,p_transaction UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE
 r public.game_spell_item_temporal_receipts%ROWTYPE;
 p public.game_spell_item_temporal_schedules%ROWTYPE;
BEGIN
 SELECT * INTO r FROM public.game_spell_item_temporal_receipts WHERE transaction_id=p_transaction
   AND item_instance_id=o.item_instance_id AND created_xact_id=pg_current_xact_id() AND custody_kind=4;
 SELECT * INTO p FROM public.game_spell_item_temporal_schedules WHERE item_instance_id=o.item_instance_id;
 IF r.transaction_id IS NULL OR p.item_instance_id IS NULL
  OR o.lifecycle<>1 OR o.world_id<>r.world_id OR o.quantity<>r.quantity_before
  OR o.state_revision<>r.state_revision_before
  OR (o.definition_family,o.definition_production_key,o.definition_revision_ref)
    <>(p.definition_family,p.definition_production_key,p.definition_revision)
  OR p.definition_production_key<>'oteryn:item.tibia.i25718' OR p.target_family IS NOT NULL
  OR r.occurred_at_unix_ms<p.expires_at_unix_ms
  OR NOT EXISTS(SELECT 1 FROM public.game_item_character_inbox_locations l
    WHERE l.item_instance_id=o.item_instance_id AND l.world_id=r.world_id
      AND l.character_id=r.character_id AND l.ordinal=r.source_ordinal) THEN
  RAISE EXCEPTION 'temple scroll Inbox timer lacks its exact due predecessor' USING ERRCODE='23514';
 END IF;
 INSERT INTO public.game_spell_item_temporal_before VALUES(r.transaction_id,o.item_instance_id,
  o.state_revision,o.quantity,o.definition_family,o.definition_production_key,o.definition_revision_ref,
  r.custody_kind,r.character_id,r.source_parent,r.source_ordinal,r.source_slot,r.source_stack_ordinal,
  pg_current_xact_id());
END $$;

-- The outcome of an Inbox removal: the retired scroll keeps no Inbox location.
CREATE FUNCTION game_temple_scroll_inbox_receipt_valid(r game_spell_item_temporal_receipts)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT game_temple_scroll_moves(r.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM public.game_item_character_inbox_locations l
   WHERE l.item_instance_id=r.item_instance_id)
$$;

-- The one runtime seam that deletes the Inbox row, only for this transaction's
-- kind-4 receipt; the Inbox otherwise stays closed to runtime DML.
CREATE FUNCTION game_temple_scroll_inbox_expire(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,public AS $$
DECLARE r public.game_spell_item_temporal_receipts%ROWTYPE;
BEGIN
 SELECT * INTO r FROM public.game_spell_item_temporal_receipts WHERE transaction_id=p_transaction
   AND created_xact_id=pg_current_xact_id() AND custody_kind=4;
 IF r.transaction_id IS NULL THEN
  RAISE EXCEPTION 'temple scroll Inbox removal lacks its timer receipt' USING ERRCODE='23514';
 END IF;
 DELETE FROM public.game_item_character_inbox_locations WHERE item_instance_id=r.item_instance_id
   AND world_id=r.world_id AND character_id=r.character_id AND ordinal=r.source_ordinal;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'temple scroll Inbox location changed' USING ERRCODE='23514';
 END IF;
END $$;

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_capture()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer predecessor capture'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts inbox_receipt
   WHERE inbox_receipt.transaction_id=NEW.last_transaction_id
     AND inbox_receipt.item_instance_id=NEW.item_instance_id
     AND inbox_receipt.created_xact_id=pg_current_xact_id() AND inbox_receipt.custody_kind=4) THEN
  PERFORM game_temple_scroll_inbox_capture(OLD,NEW.last_transaction_id);
  RETURN NEW;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_spell_item_temporal_receipt_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer outcome proof'; END IF;
 arm:=$arm$
 IF NEW.custody_kind=4 AND NOT game_temple_scroll_inbox_receipt_valid(NEW) THEN
  RAISE EXCEPTION 'temple scroll Inbox timer requires its exact retired scroll' USING ERRCODE='23514';
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

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
END $migration$;

REVOKE ALL ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_moves(UUID),
 game_temple_scroll_inbox_capture(game_item_instances,UUID),
 game_temple_scroll_inbox_receipt_valid(game_spell_item_temporal_receipts),
 game_temple_scroll_inbox_expire(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_inbox_receipt_valid(game_spell_item_temporal_receipts),
 game_temple_scroll_inbox_expire(UUID) TO oteryn_game_runtime;
