-- WU26-06 Temple Teleport Scroll (docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md).
-- The native item-use writer burns one scroll as ItemUseCause 481 with one paired
-- immutable use row. The daily claim gives each delivered scroll a seven-day
-- absolute deadline on the existing Item deadline owner
-- (game_spell_item_temporal_schedules, no decay target), whose Channel pass
-- removes the scroll when due exactly as it removes every other expiring Item.
-- This migration admits that deadline's daily-delivery provenance, the scroll's
-- movement with its deadline and its removal wherever it is, for an online or an
-- offline Character: expiry applies in all custodies (section 7; session-less
-- receipt custody 2 container entry, 3 equipment, 4 Inbox root, 5 depot root,
-- 6 House tile, 7 corpse entry, at any depth of nesting; Instance Ground is
-- retired whole at Instance teardown). All prior item-use causes and
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
 CONSTRAINT game_temple_scroll_uses_before_deadline
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
-- due scroll is retired by game_temple_scroll_expire_due under no session:
-- receipt custody 2 container entry (carried by an offline Character, or in a
-- container rooted in the Inbox, a depot, on Ground, on a House tile or in a
-- backpack dropped at death), 3
-- equipment of an offline Character, 4 Inbox root, 5 depot root (box or locker),
-- 6 House floor tile, 7 corpse container entry, each at any depth of nesting.
-- Ground-rooted custody belongs to the pass of its own Channel. Instance Ground
-- has no deadline pass: every Item on it, a due scroll included, is retired when
-- the Instance is torn down (activity retirement, 0630), so this seam never
-- touches Instance-rooted custody. Nothing inside bazaar custody is touched: an
-- escrowed Item has no custody here and a Character locked for a bazaar transfer
-- is skipped, so its due scroll goes on the first pass after the escrow or lock
-- ends. The Supply Stash holds stackable balances only and refuses scheduled
-- Items (0189), as do House containers (1169) and the market (0191).
DO $migration$
DECLARE c RECORD; n INTEGER:=0;
 arm CONSTANT TEXT:='(num_nonnulls(game_session_id,character_lease_generation,connection_generation)=0'
  ||' AND ((character_id IS NOT NULL'
  ||'  AND ((custody_kind=2 AND source_parent IS NOT NULL AND source_ordinal>0'
  ||'    AND num_nonnulls(source_slot,equipment_revision_before,source_stack_ordinal)=0)'
  ||'   OR (custody_kind=3 AND source_slot BETWEEN 1 AND 10 AND source_slot<>9'
  ||'    AND equipment_revision_before BETWEEN 1 AND 18446744073709551614'
  ||'    AND num_nonnulls(source_parent,source_ordinal,source_stack_ordinal)=0)'
  ||'   OR (custody_kind IN(4,5) AND source_ordinal>0'
  ||'    AND num_nonnulls(source_parent,source_slot,equipment_revision_before,source_stack_ordinal)=0)))'
  ||'  OR (character_id IS NULL'
  ||'  AND ((custody_kind IN(2,7) AND source_parent IS NOT NULL AND source_ordinal>0'
  ||'    AND num_nonnulls(source_slot,equipment_revision_before,source_stack_ordinal)=0)'
  ||'   OR (custody_kind=6 AND source_ordinal>0'
  ||'    AND num_nonnulls(source_parent,source_slot,equipment_revision_before,source_stack_ordinal)=0))))) IS TRUE';
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

-- The scroll's one current custody: its direct location row, the Character of
-- the custody (a container entry's own Character; for a nested Item, the
-- Character of its chain's root; none in the world), whether the chain's root is
-- unattended by a session (Inbox, depot, Ground, House tile: a backpack dropped at
-- death included) and, for Ground-rooted custody, the Channel that owns it. The
-- chain is walked through container, nested and corpse entries at any depth (the
-- 500-link bound and cycle guard of the corpse owner lock, 1152); a corpse entry
-- leaves only through a chain of corpse entries rooted live on Channel Ground.
-- No row: the scroll lies on Ground itself (the Channel pass's kind 1), in
-- bazaar custody, on or under Instance Ground (retired at teardown), or in a
-- custody no pass hosts.
CREATE FUNCTION game_temple_scroll_custody(p_item UUID)
RETURNS TABLE(kind SMALLINT,character_id UUID,parent UUID,ordinal NUMERIC,slot SMALLINT,
 equipment_revision NUMERIC,stored BOOLEAN,channel_id UUID)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 WITH RECURSIVE up(item,depth,path,corpse_chain) AS (
   SELECT p_item,0,ARRAY[p_item]::uuid[],true
   UNION ALL
   SELECT coalesce(e.parent_item_instance_id,n.parent_item_instance_id,c.parent_item_instance_id),u.depth+1,
     u.path||coalesce(e.parent_item_instance_id,n.parent_item_instance_id,c.parent_item_instance_id),
     u.corpse_chain AND num_nonnulls(e.item_instance_id,n.item_instance_id)=0 FROM up u
    LEFT JOIN public.game_item_container_entries e ON e.item_instance_id=u.item
    LEFT JOIN public.game_item_nested_entries n ON n.item_instance_id=u.item
    LEFT JOIN public.game_item_corpse_container_entries c ON c.item_instance_id=u.item
   WHERE u.depth<500 AND num_nonnulls(e.item_instance_id,n.item_instance_id,c.item_instance_id)>0
     AND NOT coalesce(e.parent_item_instance_id,n.parent_item_instance_id,c.parent_item_instance_id)=ANY(u.path)
 ), root AS (SELECT item,corpse_chain FROM up ORDER BY depth DESC LIMIT 1),
 owner AS (
   SELECT s.character_id,false AS stored,NULL::uuid AS channel_id
     FROM public.game_item_container_slots s JOIN root ON root.item=s.item_instance_id
   UNION ALL SELECT q.character_id,false,NULL::uuid
     FROM public.game_character_equipment_slots q JOIN root ON root.item=q.item_instance_id
   UNION ALL SELECT x.character_id,true,NULL::uuid
     FROM public.game_item_character_inbox_locations x JOIN root ON root.item=x.item_instance_id
   UNION ALL SELECT d.character_id,true,NULL::uuid
     FROM public.game_item_character_depot_locations d JOIN root ON root.item=d.item_instance_id
   UNION ALL SELECT NULL::uuid,true,g.channel_id
     FROM public.game_item_ground_locations g JOIN root ON root.item=g.item_instance_id
    WHERE g.scope_kind=1 AND g.channel_id IS NOT NULL
   UNION ALL SELECT NULL::uuid,true,NULL::uuid
     FROM public.game_item_house_interior_locations h JOIN root ON root.item=h.item_instance_id
 )
 SELECT 2::smallint,e.character_id,e.parent_item_instance_id,e.placement_ordinal::numeric,NULL::smallint,NULL::numeric,o.stored,o.channel_id
   FROM public.game_item_container_entries e,owner o WHERE e.item_instance_id=p_item
 UNION ALL
 SELECT 2::smallint,o.character_id,n.parent_item_instance_id,n.placement_ordinal::numeric,NULL::smallint,NULL::numeric,o.stored,o.channel_id
   FROM public.game_item_nested_entries n,owner o WHERE n.item_instance_id=p_item
 UNION ALL
 SELECT 3::smallint,q.character_id,NULL::uuid,NULL::numeric,q.slot::smallint,st.revision::numeric,false,NULL::uuid
   FROM public.game_character_equipment_slots q JOIN public.game_character_equipment_state st USING(character_id)
  WHERE q.item_instance_id=p_item
 UNION ALL
 SELECT 4::smallint,x.character_id,NULL::uuid,x.ordinal::numeric,NULL::smallint,NULL::numeric,true,NULL::uuid
   FROM public.game_item_character_inbox_locations x WHERE x.item_instance_id=p_item
 UNION ALL
 SELECT 5::smallint,d.character_id,NULL::uuid,d.ordinal::numeric,NULL::smallint,NULL::numeric,true,NULL::uuid
   FROM public.game_item_character_depot_locations d WHERE d.item_instance_id=p_item
 UNION ALL
 SELECT 6::smallint,NULL::uuid,NULL::uuid,h.stack_ordinal::numeric,NULL::smallint,NULL::numeric,true,NULL::uuid
   FROM public.game_item_house_interior_locations h WHERE h.item_instance_id=p_item
 UNION ALL
 -- A corpse entry, at any depth of corpses, leaves only while its outermost
 -- corpse lies live on Channel Ground.
 SELECT 7::smallint,NULL::uuid,c.parent_item_instance_id,c.placement_ordinal::numeric,NULL::smallint,NULL::numeric,true,o.channel_id
   FROM public.game_item_corpse_container_entries c,owner o,root
   JOIN public.game_item_instances corpse ON corpse.item_instance_id=root.item
  WHERE c.item_instance_id=p_item AND root.corpse_chain AND o.channel_id IS NOT NULL AND corpse.lifecycle=1
$$;

-- Whether no session or bazaar transfer holds this custody: a Character-less
-- custody always; a Character's custody unless it is locked for a bazaar
-- transfer, and, when carried or worn, unless a reconnect session is live.
CREATE FUNCTION game_temple_scroll_unattended(p_character UUID,p_stored BOOLEAN) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT p_character IS NULL OR (
  NOT EXISTS(SELECT 1 FROM public.game_bazaar_locks b WHERE b.character_id=p_character AND b.state='locked')
  AND (p_stored OR NOT EXISTS(SELECT 1 FROM public.game_durability_reconnect_sessions s
    WHERE s.character_id=p_character AND s.session_state IN(1,2))))
$$;

-- The predecessor of a session-less removal: the exact due scroll in the exact
-- unattended custody the receipt names.
CREATE FUNCTION game_temple_scroll_capture(o game_item_instances,p_transaction UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE
 r public.game_spell_item_temporal_receipts%ROWTYPE;
 p public.game_spell_item_temporal_schedules%ROWTYPE;
BEGIN
 SELECT * INTO r FROM public.game_spell_item_temporal_receipts WHERE transaction_id=p_transaction
   AND item_instance_id=o.item_instance_id AND created_xact_id=pg_current_xact_id()
   AND game_session_id IS NULL AND custody_kind BETWEEN 2 AND 7;
 SELECT * INTO p FROM public.game_spell_item_temporal_schedules WHERE item_instance_id=o.item_instance_id;
 IF r.transaction_id IS NULL OR p.item_instance_id IS NULL
  OR o.lifecycle<>1 OR o.world_id<>r.world_id OR o.quantity<>r.quantity_before
  OR o.state_revision<>r.state_revision_before
  OR (o.definition_family,o.definition_production_key,o.definition_revision_ref)
    <>(p.definition_family,p.definition_production_key,p.definition_revision)
  OR p.definition_production_key<>'oteryn:item.tibia.i25718' OR p.target_family IS NOT NULL
  OR r.occurred_at_unix_ms<p.expires_at_unix_ms
  OR (r.character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM public.game_character_roots c
    WHERE c.character_id=r.character_id AND c.world_id=r.world_id))
  OR (SELECT count(*) FROM public.game_temple_scroll_custody(o.item_instance_id))<>1
  OR NOT EXISTS(SELECT 1 FROM public.game_temple_scroll_custody(o.item_instance_id) k
    WHERE k.kind=r.custody_kind AND k.character_id IS NOT DISTINCT FROM r.character_id
      AND k.parent IS NOT DISTINCT FROM r.source_parent AND k.ordinal IS NOT DISTINCT FROM r.source_ordinal
      AND k.slot IS NOT DISTINCT FROM r.source_slot
      AND k.equipment_revision IS NOT DISTINCT FROM r.equipment_revision_before
      AND (k.channel_id IS NULL OR k.channel_id=r.channel_id)
      AND public.game_temple_scroll_unattended(k.character_id,k.stored)) THEN
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
  OR EXISTS(SELECT 1 FROM public.game_item_character_depot_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_house_interior_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_house_reclaim_provenance WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_item_corpse_container_entries WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM public.game_bazaar_item_locations WHERE item_instance_id=r.item_instance_id) THEN
  RAISE EXCEPTION 'temple scroll timer outcome requires exact source/custody/audit/retirement'
   USING ERRCODE='23514';
 END IF;
END $$;

-- The session-less pass: at most p_limit due scrolls, earliest deadline first.
-- The window holds only scrolls this pass may retire (custody unattended, of this
-- World, Ground-rooted ones of this Channel), so scrolls the session-bound pass
-- owns can never fill it. A scroll whose removal a native guard refuses (a House
-- under disposition, a busy lock) stays for the next pass; the others proceed.
CREATE FUNCTION game_temple_scroll_expire_due(p_world UUID,p_channel UUID,p_generation NUMERIC,p_limit INTEGER)
RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s RECORD; tx UUID; ev UUID; done INTEGER:=0; removed BIGINT; envelope BYTEA;
BEGIN
 IF p_limit IS NULL OR p_limit<=0 THEN RETURN 0; END IF;
 FOR s IN SELECT i.item_instance_id,i.state_revision,k.kind,k.character_id,k.parent,k.ordinal,k.slot,k.equipment_revision
   FROM public.game_spell_item_temporal_schedules p
   JOIN public.game_item_instances i USING(item_instance_id)
   CROSS JOIN LATERAL public.game_temple_scroll_custody(i.item_instance_id) k
   WHERE p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.lifecycle=1 AND i.quantity=1 AND i.world_id=p_world
     AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =(p.definition_family,p.definition_production_key,p.definition_revision)
     AND p.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
     AND NOT EXISTS(SELECT 1 FROM public.game_spell_item_temporal_receipts d WHERE d.item_instance_id=i.item_instance_id)
     AND (k.channel_id IS NULL OR k.channel_id=p_channel)
     AND (k.character_id IS NULL OR EXISTS(SELECT 1 FROM public.game_character_roots c
       WHERE c.character_id=k.character_id AND c.world_id=p_world))
     AND public.game_temple_scroll_unattended(k.character_id,k.stored)
   ORDER BY p.expires_at_unix_ms,i.item_instance_id LIMIT 500
   FOR UPDATE OF i,p SKIP LOCKED
 LOOP
  EXIT WHEN done>=p_limit;
  BEGIN
   tx:=public.game_character_uuid_v7(); ev:=public.game_character_uuid_v7();
   INSERT INTO public.game_spell_item_temporal_receipts(transaction_id,event_id,item_instance_id,world_id,channel_id,
     ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,character_id,
     source_parent,source_ordinal,source_slot,equipment_revision_before)
    VALUES(tx,ev,s.item_instance_id,p_world,p_channel,p_generation,s.state_revision,1,
     floor(extract(epoch FROM statement_timestamp())*1000)::bigint,s.kind,s.character_id,
     s.parent,s.ordinal,s.slot,s.equipment_revision);
   UPDATE public.game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=tx
    WHERE item_instance_id=s.item_instance_id AND state_revision=s.state_revision AND lifecycle=1;
   IF NOT FOUND THEN RAISE EXCEPTION 'temple scroll timer compare-and-change' USING ERRCODE='23514'; END IF;
   CASE s.kind
    WHEN 2 THEN
     DELETE FROM public.game_item_container_entries WHERE item_instance_id=s.item_instance_id
       AND parent_item_instance_id=s.parent AND placement_ordinal=s.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
     IF removed=0 THEN
      DELETE FROM public.game_item_nested_entries WHERE item_instance_id=s.item_instance_id
        AND parent_item_instance_id=s.parent AND placement_ordinal=s.ordinal;
      GET DIAGNOSTICS removed=ROW_COUNT;
     END IF;
    WHEN 3 THEN
     DELETE FROM public.game_character_equipment_slots WHERE item_instance_id=s.item_instance_id
       AND character_id=s.character_id AND slot=s.slot;
     GET DIAGNOSTICS removed=ROW_COUNT;
     UPDATE public.game_character_equipment_state SET revision=revision+1,last_transaction_id=tx
      WHERE character_id=s.character_id AND revision=s.equipment_revision;
     IF NOT FOUND THEN RAISE EXCEPTION 'temple scroll timer equipment epoch' USING ERRCODE='23514'; END IF;
    WHEN 4 THEN
     DELETE FROM public.game_item_character_inbox_locations WHERE item_instance_id=s.item_instance_id
       AND character_id=s.character_id AND ordinal=s.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
    WHEN 5 THEN
     DELETE FROM public.game_item_character_depot_locations WHERE item_instance_id=s.item_instance_id
       AND character_id=s.character_id AND ordinal=s.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
    WHEN 6 THEN
     DELETE FROM public.game_item_house_interior_locations WHERE item_instance_id=s.item_instance_id
       AND world_id=p_world AND stack_ordinal=s.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
     DELETE FROM public.game_item_house_reclaim_provenance WHERE item_instance_id=s.item_instance_id;
    ELSE
     DELETE FROM public.game_item_corpse_container_entries WHERE item_instance_id=s.item_instance_id
       AND parent_item_instance_id=s.parent AND placement_ordinal=s.ordinal;
     GET DIAGNOSTICS removed=ROW_COUNT;
   END CASE;
   IF removed<>1 THEN RAISE EXCEPTION 'temple scroll timer exact custody removal' USING ERRCODE='23514'; END IF;
   SELECT convert_to(jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p))::text,'UTF8') INTO envelope
     FROM public.game_spell_item_temporal_receipts r JOIN public.game_spell_item_temporal_schedules p USING(item_instance_id)
    WHERE r.transaction_id=tx;
   INSERT INTO public.game_spell_item_temporal_audit(event_id,transaction_id,envelope,envelope_sha256)
    VALUES(ev,tx,envelope,sha256(envelope));
   done:=done+1;
  EXCEPTION WHEN check_violation OR unique_violation OR lock_not_available OR raise_exception THEN
   NULL;
  END;
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
     AND scroll_receipt.game_session_id IS NULL AND scroll_receipt.custody_kind BETWEEN 2 AND 7) THEN
  PERFORM game_temple_scroll_capture(OLD,NEW.last_transaction_id);
  RETURN NEW;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_spell_item_temporal_receipt_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer outcome proof'; END IF;
 arm:=$arm$
 IF NEW.game_session_id IS NULL AND NEW.custody_kind BETWEEN 2 AND 7 THEN
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

 -- A House tile row and a corpse entry leave with their exact retired scroll. A
 -- corpse entry's removal evidence names the scope of its outermost corpse, which
 -- the session-less pass reached through a chain of corpse entries at any depth.
 body:=pg_get_functiondef('game_item_corpse_entry_removal_evidence_capture()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing corpse entry removal capture'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
   WHERE r.custody_kind=7 AND r.game_session_id IS NULL AND r.item_instance_id=OLD.item_instance_id
     AND r.world_id=OLD.world_id AND r.source_parent=OLD.parent_item_instance_id
     AND r.source_ordinal=OLD.placement_ordinal AND r.created_xact_id=pg_current_xact_id()) THEN
  WITH RECURSIVE up(id,path) AS (
    SELECT OLD.parent_item_instance_id,ARRAY[OLD.parent_item_instance_id]::uuid[]
    UNION ALL
    SELECT e.parent_item_instance_id,u.path||e.parent_item_instance_id FROM up u
     JOIN game_item_corpse_container_entries e ON e.item_instance_id=u.id
     WHERE NOT e.parent_item_instance_id=ANY(u.path) AND cardinality(u.path)<500)
  INSERT INTO game_item_corpse_entry_removal_evidence
   (item_instance_id,parent_item_instance_id,placement_ordinal,world_id,channel_id,scope_kind,instance_id)
  SELECT OLD.item_instance_id,OLD.parent_item_instance_id,OLD.placement_ordinal,
   g.world_id,g.channel_id,g.scope_kind,g.instance_id
   FROM up JOIN game_item_ground_locations g ON g.item_instance_id=up.id
   JOIN game_item_instances ci ON ci.item_instance_id=g.item_instance_id
  WHERE g.world_id=OLD.world_id AND g.scope_kind=1 AND ci.lifecycle=1;
  IF NOT FOUND THEN
   RAISE EXCEPTION 'temple scroll corpse entry removal requires its outermost corpse live on Channel Ground'
    USING ERRCODE='23514';
  END IF;
  RETURN OLD;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_house_location_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing House removal proof'; END IF;
 arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
   JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE r.custody_kind=6 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
     AND r.source_ordinal=OLD.stack_ordinal AND r.created_xact_id=pg_current_xact_id()
     AND p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
     AND i.lifecycle=2 AND i.quantity=0) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_item_corpse_entry_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing corpse entry removal proof'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
   JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE r.custody_kind=7 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
     AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
     AND r.created_xact_id=pg_current_xact_id()
     AND p.definition_production_key='oteryn:item.tibia.i25718' AND p.target_family IS NULL
     AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
     AND i.lifecycle=2 AND i.quantity=0) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

REVOKE ALL ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_moves(UUID),
 game_temple_scroll_custody(UUID),
 game_temple_scroll_unattended(UUID,BOOLEAN),
 game_temple_scroll_capture(game_item_instances,UUID),
 game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts),
 game_temple_scroll_expire_due(UUID,UUID,NUMERIC,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_temple_scroll_schedule_proven(game_spell_item_temporal_schedules),
 game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts),
 game_temple_scroll_expire_due(UUID,UUID,NUMERIC,INTEGER) TO oteryn_game_runtime;
