-- MAX501: full, bounded cohort proofs; no authority predicate is omitted.
-- Cache entries are private TEMP rows, cleared on commit. Every dependency
-- invalidates them before AND after INSERT/UPDATE/DELETE/TRUNCATE. PostgreSQL
-- snapshots plus atomic per-World writer heads fence unlocked
-- eligibility facts, excluding unrelated Worlds/databases and locked Item facts.
-- Both original proofs remain private fallbacks and retain their row locks.
-- A relevant World-fence change disables batching until the next own write,
-- so a busy cluster cannot amplify work into repeated 501-row proof scans.
DO $inventory$
DECLARE expected RECORD;
BEGIN
 FOR expected IN SELECT * FROM (VALUES
 ('game_world_reset_item_proven(uuid)','70badec65d197d39df1eaa9bd76e8132414564a3a9b8a4283fea97fe28614e47'),
 ('game_world_reset_drained(uuid,numeric)','37a0a513317d41b3e370ad16afe09e7bf256d19dd7257611bb3e0dc805221828'),
 ('game_player_death_retirement_entry_proven(uuid)','5f440f088aeda2b6f3c262c4e844bbc168ef2bb346151abc548c26490b8df854'),
 ('game_node_registration_history_valid()','d978836a48bba0c830c16fc328ad6590ebd6c400b5d543472bd94cb1fdb0f963'),
 ('game_item_instance_guard()','5cfab2acfadeac9485fb3c34030eaf736f516fd65ea243e2a0d16538df4a3b37'),
 ('game_item_instance_change_proven()','0cbf6bf2ff48d19d20ac22270baad366860d4bae05731e3dc7112fd7379a1374'),
 ('game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint)','661b438db2416ce293a2caf43437f69e36137de09a83cf0b24681896fd34a55a')) AS v(signature,source_sha256) LOOP
  IF NOT EXISTS(SELECT 1 FROM pg_proc WHERE oid=to_regprocedure(expected.signature)
   AND encode(sha256(convert_to(prosrc,'UTF8')),'hex')=expected.source_sha256
   AND proowner=current_user::regrole) THEN
   RAISE EXCEPTION 'MAX501 proof dependency inventory changed: %',expected.signature;
  END IF;
 END LOOP;
END $inventory$;
-- Receipts are permanent. Restrict discovery to the current transaction/root
-- rather than scanning all historical retirements. xid8 has a btree opclass.
CREATE INDEX game_world_reset_retire_receipts_cohort
 ON game_item_decay_retire_receipts(created_xact_id,world_id,reset_epoch,ground_root_item_instance_id)
 WHERE cause_kind='WORLD_RESET';

-- One latest committed writer per backend TEMP namespace and World, not an append-only
-- retirement history. PostgreSQL reuses these namespaces with backend slots;
-- two live backends cannot own the same TEMP namespace. Each backend serializes
-- its own transactions, so replacing its row cannot hide a committed write.
CREATE TABLE game_world_reset_proof_writers (
 temp_namespace OID NOT NULL CHECK(temp_namespace<>0), world_id UUID, write_xact_id XID8 NOT NULL,
 UNIQUE NULLS NOT DISTINCT(temp_namespace,world_id)
);
CREATE INDEX game_world_reset_proof_writers_world ON game_world_reset_proof_writers(world_id,temp_namespace);
REVOKE ALL ON game_world_reset_proof_writers FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_world_reset_proof_visibility(p_world UUID)
RETURNS TABLE(visibility_snapshot TEXT,write_fence TEXT)
LANGUAGE SQL VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
 SELECT pg_current_snapshot()::text,
  coalesce(string_agg(temp_namespace::text||':'||coalesce(world_id::text,'*')||':'||write_xact_id::text,',' ORDER BY temp_namespace,world_id NULLS FIRST),'')
 FROM game_world_reset_proof_writers WHERE world_id=p_world OR world_id IS NULL
$$;
REVOKE ALL ON FUNCTION game_world_reset_proof_visibility(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE OR REPLACE FUNCTION game_world_reset_items_proven_set(p_item UUID) RETURNS UUID[]
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $setproof$
DECLARE r0 game_item_decay_retire_receipts%ROWTYPE; j game_world_reset_jobs%ROWTYPE; ids UUID[]; proven UUID[]; drain_ok BOOLEAN; root_item_id UUID;
BEGIN
 SELECT * INTO r0 FROM game_item_decay_retire_receipts WHERE item_instance_id=p_item AND cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN ARRAY[]::UUID[]; END IF;
 SELECT ARRAY(SELECT item_instance_id FROM game_item_decay_retire_receipts WHERE cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id() AND world_id=r0.world_id AND reset_epoch=r0.reset_epoch AND ground_root_item_instance_id=r0.ground_root_item_instance_id ORDER BY item_instance_id LIMIT 502) INTO ids;
 IF cardinality(ids)>501 OR EXISTS(
  SELECT 1 FROM unnest(ids) candidate(item_id)
  WHERE (SELECT count(*) FROM (SELECT 1 FROM game_item_decay_retire_receipts r
   WHERE r.item_instance_id=candidate.item_id AND r.cause_kind='WORLD_RESET'
    AND r.created_xact_id=pg_current_xact_id() LIMIT 2) matches)>1
 ) THEN RETURN NULL; END IF;
 -- Check terminal-root bounds exactly once, before any cohort result can be
 -- reused. Refuse immediately on false, preserving the original 23514 gate;
 -- a failed root must not be rechecked by the fallback or a later callback.
 FOR root_item_id IN SELECT item_instance_id FROM game_item_decay_retire_receipts
  WHERE item_instance_id=ANY(ids) AND cause_kind='WORLD_RESET'
   AND created_xact_id=pg_current_xact_id() AND placement_ordinal IS NULL LOOP
  IF NOT game_player_death_retirement_root_bounds_proven(root_item_id) THEN
   RAISE EXCEPTION 'death Ground retirement tree exceeds admitted bounds' USING ERRCODE='23514';
  END IF;
 END LOOP;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=r0.world_id AND reset_epoch=r0.reset_epoch FOR UPDATE;
 IF NOT FOUND THEN RETURN ARRAY[]::UUID[]; END IF;
 -- Production already holds these locks (death_contents.rs / ground_admission.rs).
 -- Reassert them so even a direct owner probe cannot cache unlocked Item facts.
 -- Deleted custody rows are locked by this transaction; new custody/parent
 -- references need FK KEY SHARE on these Items, conflicting with FOR UPDATE.
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=ANY(ids) ORDER BY item_instance_id FOR UPDATE;
 drain_ok:=game_world_reset_drained(r0.world_id,r0.reset_epoch);
 SELECT coalesce(array_agg(r.item_instance_id),ARRAY[]::UUID[]) INTO proven
 FROM game_item_decay_retire_receipts r
 WHERE r.item_instance_id=ANY(ids) AND r.cause_kind='WORLD_RESET' AND r.created_xact_id=pg_current_xact_id()
 AND (j.phase<>'RETIRING' OR j.target_digest<>r.target_digest OR j.retirement_binding<>r.retirement_binding
  OR NOT drain_ok
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=r.item_instance_id AND i.world_id=r.world_id
   AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=r.item_instance_id)
  OR (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=r.item_instance_id))
  OR (EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=r.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=r.item_instance_id))
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=r.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=r.item_instance_id)
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev WHERE ev.item_instance_id=r.item_instance_id
   AND ev.transaction_id=r.transaction_id AND ev.quantity_before=r.quantity_before)
  OR NOT EXISTS(SELECT 1 FROM game_item_decay_retire_reservations v
   JOIN game_runtime_scope_assignments a ON a.world_id=v.world_id AND a.channel_id=v.channel_id
   WHERE (v.item_instance_id,v.fence_scope_ownership_generation,v.world_id,v.channel_id,v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.cause_kind,v.reset_epoch,v.retirement_binding,v.target_digest,v.ground_root_item_instance_id)
       =(r.item_instance_id,r.fence_scope_ownership_generation,r.world_id,r.channel_id,r.transaction_id,r.event_id,r.quantity_before,r.occurred_at,r.cause_kind,r.reset_epoch,r.retirement_binding,r.target_digest,r.ground_root_item_instance_id)
    AND v.placement_ordinal IS NOT DISTINCT FROM r.placement_ordinal AND sha256(v.envelope)=r.envelope_sha256
    AND a.ownership_generation=v.fence_scope_ownership_generation AND a.holder_node_id=v.fence_holder_node_id
    AND a.holder_registration_revision=v.fence_holder_registration_revision)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.item_instance_id=r.item_instance_id AND o.event_id=r.event_id
   AND o.transaction_id=r.transaction_id AND o.occurred_at=r.occurred_at AND o.envelope_sha256=r.envelope_sha256
   AND o.created_xact_id=pg_current_xact_id() AND o.publication_state=1 AND o.published_at IS NULL)
  OR (r.placement_ordinal IS NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence e WHERE e.item_instance_id=r.item_instance_id AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=r.item_instance_id)))
  OR (r.placement_ordinal IS NOT NULL AND NOT (-- Select the one current edge before joining its shared root. Without this
 -- boundary, the planner can scan every sibling through the root index for
 -- each deferred callback, despite the edge's primary-key Item binding.
 WITH edge AS MATERIALIZED (
  SELECT * FROM game_player_death_retirement_edges
  WHERE item_instance_id=r.item_instance_id AND created_xact_id=pg_current_xact_id()
 )
 SELECT EXISTS(
  SELECT 1 FROM edge e
  JOIN game_item_decay_retire_receipts entry_receipt ON entry_receipt.item_instance_id=e.item_instance_id
  JOIN game_item_decay_retire_receipts parent ON parent.item_instance_id=e.parent_item_instance_id
  JOIN game_item_decay_retire_receipts root ON root.item_instance_id=e.ground_root_item_instance_id
  JOIN game_item_instances root_item ON root_item.item_instance_id=root.item_instance_id
  JOIN game_item_ground_removal_evidence g ON g.item_instance_id=root.item_instance_id
  JOIN game_player_death_item_records d ON d.transaction_id=e.death_transaction_id
  WHERE e.item_instance_id=r.item_instance_id AND e.created_xact_id=pg_current_xact_id()
   AND entry_receipt.created_xact_id=pg_current_xact_id() AND parent.created_xact_id=pg_current_xact_id()
   AND root.created_xact_id=pg_current_xact_id() AND g.created_xact_id=pg_current_xact_id()
   AND entry_receipt.cause_kind IN('WORLD_RESET','ACTIVITY_RETIREMENT')
   AND e.ground_root_item_instance_id=entry_receipt.ground_root_item_instance_id
   AND e.placement_ordinal=entry_receipt.placement_ordinal AND e.world_id=entry_receipt.world_id
   AND (e.scope_kind,e.channel_id,e.instance_id) IS NOT DISTINCT FROM (entry_receipt.scope_kind,entry_receipt.channel_id,entry_receipt.instance_id)
   AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (entry_receipt.world_id,entry_receipt.scope_kind,entry_receipt.channel_id,entry_receipt.instance_id)
   AND d.item_instance_id=root.item_instance_id AND d.operation=1 AND d.source_slot IN(0,6,9,10)
   AND (d.world_id,d.scope_kind,d.channel_id,d.instance_id) IS NOT DISTINCT FROM (entry_receipt.world_id,entry_receipt.scope_kind,entry_receipt.channel_id,entry_receipt.instance_id)
   AND e.depth<=CASE d.source_slot WHEN 9 THEN 8 WHEN 0 THEN 7 ELSE 1 END
   AND (d.source_slot NOT IN(6,10) OR root_item.definition_production_key IN(
    'oteryn:item.tibia.i35562','oteryn:item.tibia.i35848','oteryn:item.tibia.i35849','oteryn:item.tibia.i35524',
    'oteryn:item.tibia.i36666','oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'))
   AND root.ground_root_item_instance_id=root.item_instance_id AND root.placement_ordinal IS NULL
   AND parent.ground_root_item_instance_id=root.item_instance_id
   AND (root.world_id,root.scope_kind,root.channel_id,root.instance_id,root.cause_kind,
        root.reset_epoch,root.retirement_binding,root.target_digest,root.activity_handoff_id,root.activity_ownership_generation,
        root.fence_scope_ownership_generation)
    IS NOT DISTINCT FROM
       (entry_receipt.world_id,entry_receipt.scope_kind,entry_receipt.channel_id,entry_receipt.instance_id,entry_receipt.cause_kind,
        entry_receipt.reset_epoch,entry_receipt.retirement_binding,entry_receipt.target_digest,entry_receipt.activity_handoff_id,entry_receipt.activity_ownership_generation,
        entry_receipt.fence_scope_ownership_generation)
   AND (parent.world_id,parent.scope_kind,parent.channel_id,parent.instance_id,parent.cause_kind,
        parent.reset_epoch,parent.retirement_binding,parent.target_digest,parent.activity_handoff_id,parent.activity_ownership_generation,
        parent.fence_scope_ownership_generation)
    IS NOT DISTINCT FROM
       (entry_receipt.world_id,entry_receipt.scope_kind,entry_receipt.channel_id,entry_receipt.instance_id,entry_receipt.cause_kind,
        entry_receipt.reset_epoch,entry_receipt.retirement_binding,entry_receipt.target_digest,entry_receipt.activity_handoff_id,entry_receipt.activity_ownership_generation,
        entry_receipt.fence_scope_ownership_generation)
 )) AND (
   NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence e WHERE e.item_instance_id=r.item_instance_id
    AND e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal=r.placement_ordinal
    AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
    WHERE g.item_instance_id=r.ground_root_item_instance_id AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND i.lifecycle=1
     AND NOT EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.item_instance_id=g.item_instance_id AND d.operation=1 AND d.source_slot=9
      AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(d.transaction_id)))
   OR EXISTS(SELECT 1 FROM game_item_decay_retire_receipts root
    JOIN game_player_death_item_records d ON d.item_instance_id=root.item_instance_id
    JOIN game_item_ground_removal_evidence g ON g.item_instance_id=root.item_instance_id
    WHERE root.item_instance_id=r.ground_root_item_instance_id AND root.ground_root_item_instance_id=root.item_instance_id
     AND root.cause_kind='WORLD_RESET' AND root.world_id=r.world_id AND root.channel_id=r.channel_id
     AND root.reset_epoch=r.reset_epoch AND root.retirement_binding=r.retirement_binding AND root.target_digest=r.target_digest
     AND root.fence_scope_ownership_generation=r.fence_scope_ownership_generation
     AND root.created_xact_id=pg_current_xact_id() AND g.created_xact_id=pg_current_xact_id()
     AND d.operation=1 AND d.source_slot=9))
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal<r.placement_ordinal)))) IS NOT TRUE;
 RETURN proven;
END $setproof$;
REVOKE ALL ON FUNCTION game_world_reset_items_proven_set(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE OR REPLACE FUNCTION game_world_reset_proof_cache_init() RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') IS NULL THEN
  CREATE TEMP TABLE game_world_reset_proof_cache_state_v1 (
   kind SMALLINT PRIMARY KEY CHECK(kind IN(1,2)), xact_id XID8 NOT NULL,
   visibility_snapshot TEXT COLLATE "C" NOT NULL, write_fence TEXT COLLATE "C" NOT NULL, item_id UUID, world_id UUID, reset_epoch NUMERIC, proven_items UUID[] CHECK(cardinality(proven_items)<=501), batch_allowed BOOLEAN NOT NULL DEFAULT true
  ) ON COMMIT DELETE ROWS;
  REVOKE ALL ON pg_temp.game_world_reset_proof_cache_state_v1 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
 ELSIF NOT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') AND relpersistence='t' AND relowner=current_user::regrole) THEN
  RAISE EXCEPTION 'untrusted World reset proof cache' USING ERRCODE='42501';
 END IF;
 IF to_regclass('pg_temp.game_world_reset_proof_writer_v1') IS NULL THEN
  CREATE TEMP TABLE game_world_reset_proof_writer_v1(xact_id XID8 NOT NULL,world_id UUID,UNIQUE NULLS NOT DISTINCT(xact_id,world_id)) ON COMMIT DELETE ROWS;
  REVOKE ALL ON pg_temp.game_world_reset_proof_writer_v1 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
 ELSIF NOT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass('pg_temp.game_world_reset_proof_writer_v1') AND relpersistence='t' AND relowner=current_user::regrole) THEN
  RAISE EXCEPTION 'untrusted World reset writer marker' USING ERRCODE='42501';
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_proof_cache_init() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
-- Item/custody rows are protected by the retirement's Item FOR UPDATE locks,
-- deletion locks and FK key-share conflicts. Receipt/evidence rows are immutable
-- or current-xid-owned. Registration history is fenced by its existing counter
-- FOR SHARE lock. They need OWN-write invalidation, not cross-session logging.
CREATE OR REPLACE FUNCTION game_world_reset_proof_cache_invalidate() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') IS NOT NULL
 AND EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') AND relpersistence='t' AND relowner=current_user::regrole) THEN
  DELETE FROM pg_temp.game_world_reset_proof_cache_state_v1;
 END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_proof_cache_invalidate() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_world_reset_proof_touch_world(p_world UUID) RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE xid XID8:=pg_current_xact_id();
BEGIN
 PERFORM game_world_reset_proof_cache_init();
 IF NOT EXISTS(SELECT 1 FROM pg_temp.game_world_reset_proof_writer_v1 WHERE xact_id=xid AND world_id IS NOT DISTINCT FROM p_world) THEN
  -- Only this backend owns this TEMP namespace. Other sessions never contend
  -- for its head row. The head and World facts become visible in one commit.
  INSERT INTO game_world_reset_proof_writers VALUES(pg_my_temp_schema(),p_world,xid)
   ON CONFLICT(temp_namespace,world_id) DO UPDATE SET write_xact_id=EXCLUDED.write_xact_id;
  INSERT INTO pg_temp.game_world_reset_proof_writer_v1 VALUES(xid,p_world);
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_proof_touch_world(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_world_reset_proof_world_change() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE old_world UUID; new_world UUID;
BEGIN
 IF TG_OP='TRUNCATE' THEN
  -- NULL is an all-Worlds fence, only for a whole-table truncate.
  PERFORM game_world_reset_proof_touch_world(NULL); RETURN NULL;
 END IF;
 IF TG_TABLE_NAME='game_durability_admission_runtime_guards' THEN
  IF TG_OP='UPDATE' AND (NEW.scope_key,NEW.ready,NEW.ownership_generation) IS NOT DISTINCT FROM (OLD.scope_key,OLD.ready,OLD.ownership_generation) THEN RETURN NULL; END IF;
  IF TG_OP IN('UPDATE','DELETE') THEN SELECT world_id INTO old_world FROM game_runtime_scope_assignments WHERE scope_key=OLD.scope_key; END IF;
  IF TG_OP IN('UPDATE','INSERT') THEN SELECT world_id INTO new_world FROM game_runtime_scope_assignments WHERE scope_key=NEW.scope_key; END IF;
 ELSE
  IF TG_OP='UPDATE' THEN
   IF TG_TABLE_NAME='game_durability_reconnect_sessions' THEN
    IF (NEW.world_id,NEW.session_state) IS NOT DISTINCT FROM (OLD.world_id,OLD.session_state) THEN RETURN NULL; END IF;
   END IF;
   IF TG_TABLE_NAME='game_runtime_scope_assignments' THEN
    IF (NEW.scope_key,NEW.scope_kind,NEW.world_id,NEW.channel_id,NEW.state,NEW.ownership_generation,NEW.holder_node_id,NEW.holder_registration_revision) IS NOT DISTINCT FROM (OLD.scope_key,OLD.scope_kind,OLD.world_id,OLD.channel_id,OLD.state,OLD.ownership_generation,OLD.holder_node_id,OLD.holder_registration_revision) THEN RETURN NULL; END IF;
   END IF;
  END IF;
  IF TG_OP IN('UPDATE','DELETE') THEN old_world:=OLD.world_id; END IF;
  IF TG_OP IN('UPDATE','INSERT') THEN new_world:=NEW.world_id; END IF;
 END IF;
 IF old_world IS NOT NULL THEN PERFORM game_world_reset_proof_touch_world(old_world); END IF;
 IF new_world IS NOT NULL AND new_world IS DISTINCT FROM old_world THEN PERFORM game_world_reset_proof_touch_world(new_world); END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_proof_world_change() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE OR REPLACE FUNCTION public.game_world_reset_drained_uncached(p_world uuid, p_epoch numeric)
 RETURNS boolean
 LANGUAGE sql
 SECURITY DEFINER
 SET search_path TO 'public', 'pg_temp'
AS $function$
 SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j JOIN game_world_reset_active a USING(world_id,reset_epoch)
               WHERE j.world_id=p_world AND j.reset_epoch=p_epoch AND j.phase='RETIRING')
 AND (SELECT count(*) FROM game_world_reset_channels WHERE world_id=p_world AND reset_epoch=p_epoch) BETWEEN 1 AND 128
 AND NOT EXISTS(SELECT 1 FROM (SELECT * FROM game_runtime_scope_assignments WHERE scope_kind=1 AND world_id=p_world) a
  FULL JOIN (SELECT * FROM game_world_reset_channels WHERE world_id=p_world AND reset_epoch=p_epoch) c
   ON a.world_id=c.world_id AND a.channel_id=c.channel_id
  LEFT JOIN game_durability_admission_runtime_guards g ON g.scope_key=a.scope_key
  WHERE (a.world_id=p_world OR c.world_id=p_world) AND
   (a.scope_key IS NULL OR c.channel_id IS NULL OR a.state<>1 OR a.ownership_generation<=c.predecessor_generation
    OR g.scope_key IS NULL OR g.ready OR g.ownership_generation<>a.ownership_generation
    OR NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision)))
 AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE world_id=p_world AND session_state<>3)
$function$
;
REVOKE ALL ON FUNCTION game_world_reset_drained_uncached(uuid,numeric) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE OR REPLACE FUNCTION public.game_world_reset_drained(p_world uuid, p_epoch numeric)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE snap TEXT; fence TEXT; xid XID8; proven BOOLEAN; cached RECORD;
BEGIN
 -- Row callbacks within writes and WITH HOLD materialization cannot cache.
 IF pg_trigger_depth()>0 AND (pg_trigger_depth()<>1 OR current_query()<>'COMMIT' OR EXISTS(SELECT 1 FROM pg_cursors WHERE is_holdable)) THEN
  RETURN game_world_reset_drained_uncached(p_world,p_epoch);
 END IF;
 xid:=pg_current_xact_id();
 SELECT pg_current_snapshot()::text INTO snap;
 IF to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') IS NOT NULL
 AND EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') AND relpersistence='t' AND relowner=current_user::regrole) THEN
  SELECT c.* INTO cached FROM pg_temp.game_world_reset_proof_cache_state_v1 c
   WHERE c.kind=1 AND c.xact_id=xid AND c.world_id=p_world AND c.reset_epoch=p_epoch;
  IF FOUND THEN
   IF cached.visibility_snapshot COLLATE "C"=snap COLLATE "C" THEN RETURN TRUE; END IF;
   SELECT * INTO snap,fence FROM game_world_reset_proof_visibility(p_world);
   IF cached.write_fence COLLATE "C"=fence COLLATE "C" THEN
    UPDATE pg_temp.game_world_reset_proof_cache_state_v1 SET visibility_snapshot=snap WHERE kind=1;
    RETURN TRUE;
   END IF;
  END IF;
 END IF;
 -- Capture BEFORE the proof: never label older facts with a newer writer fence.
 IF fence IS NULL THEN SELECT * INTO snap,fence FROM game_world_reset_proof_visibility(p_world); END IF;
 proven:=game_world_reset_drained_uncached(p_world,p_epoch);
 IF proven THEN
  PERFORM game_world_reset_proof_cache_init();
  INSERT INTO pg_temp.game_world_reset_proof_cache_state_v1(kind,xact_id,visibility_snapshot,write_fence,item_id,world_id,reset_epoch)
   VALUES(1,xid,snap,fence,NULL,p_world,p_epoch)
   ON CONFLICT(kind) DO UPDATE SET xact_id=EXCLUDED.xact_id,visibility_snapshot=EXCLUDED.visibility_snapshot,
    write_fence=EXCLUDED.write_fence,item_id=EXCLUDED.item_id,world_id=EXCLUDED.world_id,reset_epoch=EXCLUDED.reset_epoch;
 END IF;
 RETURN proven;
END $$;

CREATE OR REPLACE FUNCTION public.game_world_reset_item_proven_uncached(p_item uuid)
 RETURNS boolean
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $function$
DECLARE r game_item_decay_retire_receipts%ROWTYPE; j game_world_reset_jobs%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_decay_retire_receipts
  WHERE item_instance_id=p_item AND cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 IF r.placement_ordinal IS NULL THEN
  IF NOT game_player_death_retirement_root_bounds_proven(p_item) THEN
   RAISE EXCEPTION 'death Ground retirement tree exceeds admitted bounds' USING ERRCODE='23514';
  END IF;
 END IF;

 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=r.world_id AND reset_epoch=r.reset_epoch FOR UPDATE;
 IF NOT FOUND OR j.phase<>'RETIRING' OR j.target_digest<>r.target_digest OR j.retirement_binding<>r.retirement_binding
  OR NOT game_world_reset_drained(r.world_id,r.reset_epoch)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.world_id=r.world_id
   AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item)
  OR (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p_item) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=p_item))
  OR (EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=p_item))
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item)
  OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev WHERE ev.item_instance_id=p_item
   AND ev.transaction_id=r.transaction_id AND ev.quantity_before=r.quantity_before)
  OR NOT EXISTS(SELECT 1 FROM game_item_decay_retire_reservations v
   JOIN game_runtime_scope_assignments a ON a.world_id=v.world_id AND a.channel_id=v.channel_id
   WHERE (v.item_instance_id,v.fence_scope_ownership_generation,v.world_id,v.channel_id,v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.cause_kind,v.reset_epoch,v.retirement_binding,v.target_digest,v.ground_root_item_instance_id)
       =(r.item_instance_id,r.fence_scope_ownership_generation,r.world_id,r.channel_id,r.transaction_id,r.event_id,r.quantity_before,r.occurred_at,r.cause_kind,r.reset_epoch,r.retirement_binding,r.target_digest,r.ground_root_item_instance_id)
    AND v.placement_ordinal IS NOT DISTINCT FROM r.placement_ordinal AND sha256(v.envelope)=r.envelope_sha256
    AND a.ownership_generation=v.fence_scope_ownership_generation AND a.holder_node_id=v.fence_holder_node_id
    AND a.holder_registration_revision=v.fence_holder_registration_revision)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox o WHERE o.item_instance_id=p_item AND o.event_id=r.event_id
   AND o.transaction_id=r.transaction_id AND o.occurred_at=r.occurred_at AND o.envelope_sha256=r.envelope_sha256
   AND o.created_xact_id=pg_current_xact_id() AND o.publication_state=1 AND o.published_at IS NULL)
  OR (r.placement_ordinal IS NULL AND (
   NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence e WHERE e.item_instance_id=p_item AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p_item)))
  OR (r.placement_ordinal IS NOT NULL AND NOT game_player_death_retirement_entry_proven(p_item) AND (
   NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence e WHERE e.item_instance_id=p_item
    AND e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal=r.placement_ordinal
    AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.created_xact_id=pg_current_xact_id())
   OR NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
    WHERE g.item_instance_id=r.ground_root_item_instance_id AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND i.lifecycle=1
     AND NOT EXISTS(SELECT 1 FROM game_player_death_item_records d WHERE d.item_instance_id=g.item_instance_id AND d.operation=1 AND d.source_slot=9
      AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(d.transaction_id)))
   OR EXISTS(SELECT 1 FROM game_item_decay_retire_receipts root
    JOIN game_player_death_item_records d ON d.item_instance_id=root.item_instance_id
    JOIN game_item_ground_removal_evidence g ON g.item_instance_id=root.item_instance_id
    WHERE root.item_instance_id=r.ground_root_item_instance_id AND root.ground_root_item_instance_id=root.item_instance_id
     AND root.cause_kind='WORLD_RESET' AND root.world_id=r.world_id AND root.channel_id=r.channel_id
     AND root.reset_epoch=r.reset_epoch AND root.retirement_binding=r.retirement_binding AND root.target_digest=r.target_digest
     AND root.fence_scope_ownership_generation=r.fence_scope_ownership_generation
     AND root.created_xact_id=pg_current_xact_id() AND g.created_xact_id=pg_current_xact_id()
     AND d.operation=1 AND d.source_slot=9))
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=r.ground_root_item_instance_id AND e.placement_ordinal<r.placement_ordinal))) THEN
  RAISE EXCEPTION 'World reset requires current drain, conserved item, removal, reservation and audit in one transaction' USING ERRCODE='23514';
 END IF;
 RETURN TRUE;
END $function$
;
REVOKE ALL ON FUNCTION game_world_reset_item_proven_uncached(uuid) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE OR REPLACE FUNCTION public.game_world_reset_item_proven(p_item uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE snap TEXT; fence TEXT; xid XID8; proven BOOLEAN; items UUID[]; allow_batch BOOLEAN:=true; cached RECORD; scope_world UUID; scope_epoch NUMERIC;
BEGIN
 IF pg_trigger_depth()>0 AND (pg_trigger_depth()<>1 OR current_query()<>'COMMIT' OR EXISTS(SELECT 1 FROM pg_cursors WHERE is_holdable)) THEN
  RETURN game_world_reset_item_proven_uncached(p_item);
 END IF;
 xid:=pg_current_xact_id();
 SELECT pg_current_snapshot()::text INTO snap;
 IF to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') IS NOT NULL
 AND EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass('pg_temp.game_world_reset_proof_cache_state_v1') AND relpersistence='t' AND relowner=current_user::regrole) THEN
  SELECT c.* INTO cached FROM pg_temp.game_world_reset_proof_cache_state_v1 c WHERE c.kind=2 AND c.xact_id=xid;
  IF FOUND THEN
   allow_batch:=cached.batch_allowed;
   IF p_item=ANY(cached.proven_items) THEN
    IF cached.visibility_snapshot COLLATE "C"=snap COLLATE "C" THEN
    IF p_item=ANY(cached.proven_items) THEN RETURN TRUE; END IF;
   ELSE
    SELECT * INTO snap,fence FROM game_world_reset_proof_visibility(cached.world_id);
    IF cached.write_fence COLLATE "C"=fence COLLATE "C" THEN
     -- An unrelated commit changed the global snapshot, not this World
     -- or its locked/immutable authority facts. Retain the proven cohort and advance only its snapshot.
     UPDATE pg_temp.game_world_reset_proof_cache_state_v1 SET visibility_snapshot=snap WHERE kind=2;
     IF p_item=ANY(cached.proven_items) THEN RETURN TRUE; END IF;
    ELSE
     allow_batch:=false;
    END IF;
   END IF;
   END IF;
  END IF;
 END IF;
 SELECT world_id,reset_epoch INTO scope_world,scope_epoch FROM game_item_decay_retire_receipts
  WHERE item_instance_id=p_item AND cause_kind='WORLD_RESET' AND created_xact_id=xid;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 IF fence IS NULL THEN SELECT * INTO snap,fence FROM game_world_reset_proof_visibility(scope_world); END IF;
 IF allow_batch THEN items:=game_world_reset_items_proven_set(p_item); END IF;
 proven:=p_item=ANY(items);
 IF proven IS NOT TRUE THEN
  proven:=game_world_reset_item_proven_uncached(p_item);
  items:=NULL; allow_batch:=false;
 END IF;
 IF proven THEN
  PERFORM game_world_reset_proof_cache_init();
  INSERT INTO pg_temp.game_world_reset_proof_cache_state_v1(kind,xact_id,visibility_snapshot,write_fence,item_id,world_id,reset_epoch,proven_items,batch_allowed)
   VALUES(2,xid,snap,fence,p_item,scope_world,scope_epoch,items,allow_batch)
   ON CONFLICT(kind) DO UPDATE SET xact_id=EXCLUDED.xact_id,visibility_snapshot=EXCLUDED.visibility_snapshot,
    write_fence=EXCLUDED.write_fence,item_id=EXCLUDED.item_id,world_id=EXCLUDED.world_id,reset_epoch=EXCLUDED.reset_epoch,proven_items=EXCLUDED.proven_items,batch_allowed=EXCLUDED.batch_allowed;
 END IF;
 RETURN proven;
END $$;
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_decay_retire_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_decay_retire_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_jobs FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_jobs FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_active FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_active FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_channels FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_world_reset_channels FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_runtime_scope_assignments FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_runtime_scope_assignments FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_durability_admission_runtime_guards FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_durability_admission_runtime_guards FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_durability_reconnect_sessions FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_durability_reconnect_sessions FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registration_writer FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registration_writer FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registrations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registrations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registration_endings FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_node_registration_endings FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_instances FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_instances FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_ground_locations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_ground_locations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_corpse_container_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_corpse_container_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_container_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_container_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_nested_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_nested_entries FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_container_slots FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_container_slots FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_house_interior_locations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_house_interior_locations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_transfer_quantity_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_transfer_quantity_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_decay_retire_reservations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_decay_retire_reservations FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_audit_outbox FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_audit_outbox FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_ground_removal_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_ground_removal_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_corpse_entry_removal_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_item_corpse_entry_removal_evidence FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_player_death_retirement_edges FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_player_death_retirement_edges FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_before BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_player_death_item_records FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();
CREATE TRIGGER game_world_reset_proof_cache_after AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON game_player_death_item_records FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_cache_invalidate();

CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_world_reset_jobs FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_world_reset_jobs FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_world_reset_active FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_world_reset_active FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_world_reset_channels FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_world_reset_channels FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_runtime_scope_assignments FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_runtime_scope_assignments FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_durability_admission_runtime_guards FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_durability_admission_runtime_guards FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_change AFTER INSERT OR UPDATE OR DELETE ON game_durability_reconnect_sessions FOR EACH ROW EXECUTE FUNCTION game_world_reset_proof_world_change();
CREATE TRIGGER game_world_reset_world_truncate AFTER TRUNCATE ON game_durability_reconnect_sessions FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_proof_world_change();

-- Keep the ordered per-child writer, but reserve its already-frozen inputs in one statement.
DO $change$
DECLARE def TEXT; body TEXT; original TEXT; loop_at INT; insert_at INT; call_at INT; ins TEXT; vals TEXT;
BEGIN
 def:=pg_get_functiondef('game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint)'::regprocedure);
 SELECT prosrc INTO original FROM pg_proc WHERE oid='game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint)'::regprocedure;
 body:=original;
 loop_at:=strpos(body,' FOR k IN 1..n LOOP');
 insert_at:=strpos(body,'  INSERT INTO game_item_decay_retire_reservations');
 call_at:=strpos(body,'  IF NOT game_player_death_retire_entry');
 IF loop_at=0 OR insert_at=0 OR call_at=0 THEN RAISE EXCEPTION 'unexpected ordered batch'; END IF;
 ins:=substring(body FROM insert_at FOR call_at-insert_at);
 vals:=substring(ins FROM 'VALUES\((.*)\);');
 IF vals IS NULL THEN RAISE EXCEPTION 'unexpected reservation values'; END IF;
 ins:=substring(ins FROM 1 FOR strpos(ins,'VALUES(')-1)||'SELECT '||replace(vals,'[k]','[batch.position]')||' FROM generate_series(1,n) AS batch(position) ORDER BY batch.position;'||E'\n';
 body:=substring(body FROM 1 FOR loop_at-1)||ins||E' FOR k IN 1..n LOOP\n'||substring(body FROM call_at);
 EXECUTE replace(def,original,body);
END $change$;
-- Compare the complete typed immutable record, without serializing it twice.
DO $change$
DECLARE name TEXT; def TEXT; original TEXT; body TEXT; anchor TEXT; cut_at INT;
 expected TEXT:=E' IF TG_OP=''UPDATE'' THEN\n  max501_expected:=OLD;\n  max501_expected.quantity:=NEW.quantity; max501_expected.lifecycle:=NEW.lifecycle;\n  max501_expected.last_transaction_id:=NEW.last_transaction_id; max501_expected.state_revision:=NEW.state_revision;\n END IF;\n';
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven'] LOOP
  def:=pg_get_functiondef((name||'()')::regprocedure);
  SELECT prosrc INTO original FROM pg_proc WHERE oid=(name||'()')::regprocedure;
  body:=original;
  IF name='game_item_instance_guard' THEN
   body:=regexp_replace(body,'\mDECLARE\M','DECLARE max501_expected game_item_instances%ROWTYPE;');
   anchor:=E'    IF TG_OP = ''UPDATE'' THEN\n        v_instance_guard_old_json := to_jsonb(OLD);\n        v_instance_guard_new_json := to_jsonb(NEW);\n    END IF;';
   IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing typed guard anchor'; END IF;
   body:=replace(body,anchor,expected);
   body:=replace(body,E'(v_instance_guard_new_json-ARRAY[''quantity'',''lifecycle'',''last_transaction_id'',''state_revision''])\n           =(v_instance_guard_old_json-ARRAY[''quantity'',''lifecycle'',''last_transaction_id'',''state_revision''])','NEW IS NOT DISTINCT FROM max501_expected');
   cut_at:=strpos(body,'-- Exactly the shared terminal arm');
   IF cut_at=0 THEN RAISE EXCEPTION 'missing terminal return'; END IF;
   cut_at:=cut_at+strpos(substring(body FROM cut_at),E'    END IF;')-1+length(E'    END IF;');
   body:=overlay(body placing E'\n'||anchor FROM cut_at FOR 0);
  ELSE
   body:=regexp_replace(body,'\mBEGIN\M',E'DECLARE max501_expected game_item_instances%ROWTYPE;\nBEGIN\n'||expected);
   anchor:=E'(to_jsonb(NEW)-ARRAY[''quantity'',''lifecycle'',''last_transaction_id'',''state_revision''])\n      =(to_jsonb(OLD)-ARRAY[''quantity'',''lifecycle'',''last_transaction_id'',''state_revision''])';
   IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing terminal record comparison'; END IF;
   body:=replace(body,anchor,'NEW IS NOT DISTINCT FROM max501_expected');
  END IF;
  EXECUTE replace(def,original,body);
 END LOOP;
END $change$;
