//! HOUSE-1a ordinary-property storage (HOUSE-OWN-0 §3; HOUSE-TENURE §4.2/Q1a).
//!
//! Uninstalled migration fragment: no runtime grants, migration number or writer activation.
//! The bank/escrow/Premium/event transaction is a separate incomplete acquisition boundary.
//! These constraints conserve canonical property, personal slot and the existing 0038 anchor;
//! they do not turn a receipt UUID or an administrator's fixture into acquisition authority.
//! Guild release is a separate caller-TX participant below; activation awaits the explicit
//! recipient deletion-pin lock-order amendment and the D caller composition.
//! Activation also awaits the `house_spell_acl::house_access_for_character` consumer amendment:
//! it still decodes `owner_character_id` as non-NULL, so a Guildhall or released anchor fails
//! that lookup closed (decode error, no grant) until it reads the nullable owner and routes
//! GUILD ownership.

/// Install atomically at the CP slot, before admitting any acquisition writer. Existing legacy
/// owners need explicit reconciliation: 0038 has no first-rent or paid-until facts to import.
/// Never guess those facts, delete a legacy anchor, or change its 0047 instance binding.
#[allow(
    dead_code,
    reason = "Uninstalled owned migration fragment; CP controls registration"
)]
pub(crate) const ORDINARY_PROPERTY_SCHEMA_SQL: &str = r#"
DO $$ BEGIN
 IF EXISTS (SELECT 1 FROM game_house_ownership) THEN
  RAISE EXCEPTION 'legacy House ownership requires explicit tenancy reconciliation' USING ERRCODE='23514';
 END IF;
END $$;

CREATE TABLE game_house_properties (
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 state TEXT NOT NULL CHECK(state IN('VACANT','AUCTION','OWNED','MOVE_OUT_PENDING','DISPOSITION','RETIRED')),
 owner_kind TEXT CHECK(owner_kind IN('CHARACTER','GUILD')),
 owner_character_id UUID REFERENCES game_character_roots(character_id),
 owner_account_id UUID REFERENCES game_character_account_guards(account_id),
 owner_guild_id UUID CHECK(owner_guild_id IS NULL OR game_character_is_uuid_v7(owner_guild_id)),
 paid_until BIGINT, grace_until BIGINT, move_out_at BIGINT,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 acl_revision NUMERIC(20,0) NOT NULL CHECK(acl_revision BETWEEN 0 AND 18446744073709551615),
 -- Ownership generation is separate from lifecycle/ACL revisions, preserving the 0038 ABI.
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 0 AND 18446744073709551615),
 acquisition_receipt_ref UUID CHECK(acquisition_receipt_ref IS NULL OR game_character_is_uuid_v7(acquisition_receipt_ref)),
 content_fence_operation UUID CHECK(content_fence_operation IS NULL OR game_character_is_uuid_v7(content_fence_operation)),
 PRIMARY KEY(world_id,house_key),
 CHECK(((owner_kind IS NULL AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NULL
         AND state IN('VACANT','AUCTION','RETIRED') AND paid_until IS NULL
         AND grace_until IS NULL AND move_out_at IS NULL)
     OR (owner_kind='CHARACTER' AND owner_character_id IS NOT NULL AND owner_account_id IS NOT NULL AND owner_guild_id IS NULL
         AND state IN('OWNED','MOVE_OUT_PENDING','DISPOSITION') AND paid_until IS NOT NULL AND paid_until>0)
     OR (owner_kind='GUILD' AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NOT NULL
         AND state IN('OWNED','MOVE_OUT_PENDING','DISPOSITION') AND paid_until IS NOT NULL AND paid_until>0)) IS TRUE),
 CHECK((ownership_revision=0)=(acquisition_receipt_ref IS NULL)),
 CHECK(grace_until IS NULL OR (paid_until IS NOT NULL AND grace_until>paid_until)),
 CHECK(move_out_at IS NULL OR move_out_at>0)
);
-- A Guildhall is outside the personal Account/World slot, but a Guild owns at most one Hall.
CREATE UNIQUE INDEX game_house_one_guildhall ON game_house_properties(owner_guild_id) WHERE owner_guild_id IS NOT NULL;
CREATE TABLE game_house_tiles (
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 tile_x INTEGER NOT NULL CHECK(tile_x BETWEEN 0 AND 65535),
 tile_y INTEGER NOT NULL CHECK(tile_y BETWEEN 0 AND 65535),
 tile_floor SMALLINT NOT NULL CHECK(tile_floor BETWEEN -15 AND 0),
 PRIMARY KEY(world_id,house_key,tile_x,tile_y,tile_floor),
 -- A World tile cannot belong to two physical properties.
 UNIQUE(world_id,tile_x,tile_y,tile_floor),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE TABLE game_account_housing_slots (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind='PHYSICAL_HOUSE'),
 PRIMARY KEY(account_id,world_id), UNIQUE(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);

CREATE FUNCTION game_house_property_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  RAISE EXCEPTION 'House property is retained through retirement' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.state<>'VACANT' OR NEW.owner_kind IS NOT NULL OR NEW.revision<>1
     OR NEW.acl_revision<>0 OR NEW.ownership_revision<>0 OR NEW.content_fence_operation IS NOT NULL THEN
   RAISE EXCEPTION 'House property must start vacant' USING ERRCODE='23514';
  END IF;
 ELSE
  IF (NEW.world_id,NEW.house_key) IS DISTINCT FROM (OLD.world_id,OLD.house_key) THEN
   RAISE EXCEPTION 'House identity is immutable' USING ERRCODE='23514';
  END IF;
  IF OLD.state='RETIRED' AND NEW.state<>'RETIRED' THEN
   RAISE EXCEPTION 'Retired House cannot return to auction' USING ERRCODE='23514';
  END IF;
  -- ACL edits advance only the property-global ACL revision; lifecycle writes advance revision.
  IF (to_jsonb(NEW)-'acl_revision')=(to_jsonb(OLD)-'acl_revision') THEN
   IF NEW.acl_revision<>OLD.acl_revision+1 THEN
    RAISE EXCEPTION 'House ACL requires exact revision successor' USING ERRCODE='23514';
   END IF;
  ELSIF NEW.revision<>OLD.revision+1 OR (NEW.acl_revision<>OLD.acl_revision AND NOT
    (OLD.owner_kind='GUILD' AND NEW.owner_kind IS NULL AND NEW.acl_revision=OLD.acl_revision+1)) THEN
   RAISE EXCEPTION 'House lifecycle requires exact revision successor' USING ERRCODE='23514';
  END IF;
  IF (NEW.owner_kind,NEW.owner_character_id,NEW.owner_account_id,NEW.owner_guild_id)
       IS DISTINCT FROM (OLD.owner_kind,OLD.owner_character_id,OLD.owner_account_id,OLD.owner_guild_id) THEN
   IF NEW.ownership_revision<>OLD.ownership_revision+1 THEN
    RAISE EXCEPTION 'House ownership requires exact generation successor' USING ERRCODE='23514';
   END IF;
   IF NEW.owner_kind IS NOT NULL THEN
    IF OLD.owner_kind IS NOT NULL OR OLD.state<>'AUCTION' OR NEW.state<>'OWNED'
       OR NEW.acquisition_receipt_ref IS NULL
       OR NEW.acquisition_receipt_ref IS NOT DISTINCT FROM OLD.acquisition_receipt_ref THEN
     RAISE EXCEPTION 'House acquisition must succeed an auction with new receipt' USING ERRCODE='23514';
    END IF;
   ELSIF OLD.state<>'DISPOSITION' OR NEW.state NOT IN('VACANT','RETIRED')
         OR NEW.acquisition_receipt_ref IS DISTINCT FROM OLD.acquisition_receipt_ref THEN
    RAISE EXCEPTION 'House release must retain its acquisition anchor' USING ERRCODE='23514';
   END IF;
  ELSIF (NEW.ownership_revision,NEW.acquisition_receipt_ref)
       IS DISTINCT FROM (OLD.ownership_revision,OLD.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'House ownership evidence cannot change without ownership' USING ERRCODE='23514';
  END IF;
 END IF;
 IF NEW.owner_kind='CHARACTER' AND NOT EXISTS (
  SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.owner_character_id
   AND r.account_id=NEW.owner_account_id AND r.world_id=NEW.world_id AND r.lifecycle=1
 ) THEN
  RAISE EXCEPTION 'House owner must be current Character of its Account and World' USING ERRCODE='23514';
 END IF;
 IF NEW.owner_kind='GUILD' THEN
 IF NOT EXISTS (
  SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.owner_guild_id AND g.world_id=NEW.world_id
   AND (g.state=2 OR (g.state=3 AND TG_OP='UPDATE' AND OLD.owner_guild_id=NEW.owner_guild_id))
 ) THEN
  RAISE EXCEPTION 'Guildhall owner must bind its current Guild World/lifecycle' USING ERRCODE='23514';
 END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_property_guard BEFORE INSERT OR UPDATE OR DELETE
 ON game_house_properties FOR EACH ROW EXECUTE FUNCTION game_house_property_guard();

ALTER TABLE game_house_ownership ALTER COLUMN owner_character_id DROP NOT NULL;
ALTER TABLE game_house_ownership ADD COLUMN owner_guild_id UUID CHECK(owner_guild_id IS NULL OR game_character_is_uuid_v7(owner_guild_id));
CREATE OR REPLACE FUNCTION game_house_owner_in_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key) IS DISTINCT FROM (OLD.world_id,OLD.house_key)
    OR NEW.ownership_revision<>OLD.ownership_revision+1) THEN
  RAISE EXCEPTION 'House anchor requires exact ownership successor' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS (SELECT 1 FROM game_house_properties p WHERE p.world_id=NEW.world_id AND p.house_key=NEW.house_key
   AND p.owner_character_id IS NOT DISTINCT FROM NEW.owner_character_id
   AND p.owner_guild_id IS NOT DISTINCT FROM NEW.owner_guild_id
   AND p.ownership_revision=NEW.ownership_revision AND p.acquisition_receipt_ref=NEW.acquisition_receipt_ref) THEN
  RAISE EXCEPTION 'House anchor must follow canonical property in the same transaction' USING ERRCODE='23514';
 END IF;
 IF NEW.owner_character_id IS NOT NULL THEN
  IF NOT EXISTS (SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.owner_character_id
      AND r.world_id=NEW.world_id AND r.lifecycle=1)
     OR (TG_OP='UPDATE' AND NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'House acquisition requires active owner and new receipt' USING ERRCODE='23514';
  END IF;
 ELSIF NEW.owner_guild_id IS NOT NULL THEN
  IF NEW.owner_character_id IS NOT NULL OR NOT EXISTS(SELECT 1 FROM game_guilds g
    WHERE g.guild_id=NEW.owner_guild_id AND g.world_id=NEW.world_id AND g.state=2)
    OR (TG_OP='UPDATE' AND NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'Guildhall acquisition requires its active Guild and new receipt' USING ERRCODE='23514';
  END IF;
 ELSIF TG_OP<>'UPDATE' OR (OLD.owner_character_id IS NULL AND OLD.owner_guild_id IS NULL)
       OR NEW.acquisition_receipt_ref<>OLD.acquisition_receipt_ref THEN
  RAISE EXCEPTION 'House release clears owner and retains receipt' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_house_anchor_retained() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'House ownership anchor cannot be deleted' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_house_anchor_retained BEFORE DELETE ON game_house_ownership
 FOR EACH ROW EXECUTE FUNCTION game_house_anchor_retained();
-- Bounded writer: every per-list ACL insert/update/delete advances only the property-global
-- acl_revision, in the same transaction. Runtime keeps no property grant. ACL writers lock
-- anchor, ACL, then property; a crossing release aborts by deadlock detection, never stale.
CREATE FUNCTION game_house_acl_advance_property() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID; h TEXT;
BEGIN
 IF TG_OP='DELETE' THEN w:=OLD.world_id; h:=OLD.house_key; ELSE w:=NEW.world_id; h:=NEW.house_key; END IF;
 UPDATE game_house_properties SET acl_revision=acl_revision+1 WHERE world_id=w AND house_key=h;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'House ACL requires its canonical property' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND (NEW.world_id,NEW.house_key) IS DISTINCT FROM (OLD.world_id,OLD.house_key) THEN
  RAISE EXCEPTION 'House ACL identity is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_house_acl_property_revision AFTER INSERT OR UPDATE OR DELETE ON game_house_acl
 FOR EACH ROW EXECUTE FUNCTION game_house_acl_advance_property();

CREATE FUNCTION game_house_property_consistent(p_world UUID,p_house TEXT) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE p game_house_properties; a game_house_ownership;
BEGIN
 SELECT * INTO p FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'House anchor/slot has no canonical property' USING ERRCODE='23514';
 END IF;
 SELECT * INTO a FROM game_house_ownership WHERE world_id=p_world AND house_key=p_house;
 IF p.ownership_revision=0 THEN
  IF FOUND THEN RAISE EXCEPTION 'Never-owned House has legacy ownership' USING ERRCODE='23514'; END IF;
 ELSIF NOT FOUND OR a.owner_character_id IS DISTINCT FROM p.owner_character_id
       OR a.owner_guild_id IS DISTINCT FROM p.owner_guild_id
       OR a.ownership_revision<>p.ownership_revision OR a.acquisition_receipt_ref<>p.acquisition_receipt_ref THEN
  RAISE EXCEPTION 'House property and legacy ownership disagree' USING ERRCODE='23514';
 END IF;
 IF p.owner_kind='CHARACTER' THEN
  IF NOT EXISTS (SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=p_world AND s.house_key=p_house
      AND s.account_id=p.owner_account_id AND s.kind='PHYSICAL_HOUSE') THEN
   RAISE EXCEPTION 'Owned House requires its Account World slot' USING ERRCODE='23514';
  END IF;
 ELSIF EXISTS (SELECT 1 FROM game_account_housing_slots WHERE world_id=p_world AND house_key=p_house) THEN
  RAISE EXCEPTION 'Unowned House cannot retain personal slot' USING ERRCODE='23514';
 END IF;
END $$;
CREATE FUNCTION game_house_property_commit_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- An updated slot can remove the old property's slot; validate BOTH keys, not only NEW.
 IF TG_OP IN('UPDATE','DELETE') THEN PERFORM game_house_property_consistent(OLD.world_id,OLD.house_key); END IF;
 IF TG_OP IN('INSERT','UPDATE') THEN PERFORM game_house_property_consistent(NEW.world_id,NEW.house_key); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_property_commit AFTER INSERT OR UPDATE ON game_house_properties
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();
CREATE CONSTRAINT TRIGGER game_house_anchor_commit AFTER INSERT OR UPDATE ON game_house_ownership
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();
CREATE CONSTRAINT TRIGGER game_house_slot_commit AFTER INSERT OR UPDATE OR DELETE ON game_account_housing_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();

REVOKE ALL ON game_house_properties,game_house_tiles,game_account_housing_slots FROM PUBLIC,oteryn_game_runtime;
-- 0038 gave control a standalone ownership writer. It cannot outlive canonical acquisition.
REVOKE INSERT,UPDATE ON game_house_ownership FROM oteryn_game_control;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_property_guard','game_house_owner_in_world',
  'game_house_anchor_retained','game_house_property_commit_guard','game_house_acl_advance_property'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_property_consistent(UUID,TEXT) SET search_path=%I,pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_house_property_consistent(UUID,TEXT) FROM PUBLIC;
END $$;
"#;

// This child remains inside durability so it consumes D's genuine pub(super) context.
#[path = "../house/guildhall_release.rs"]
mod guildhall_release;
pub(super) use guildhall_release::{
    GuildHallReleaseError, GuildHallReleaseOutcome, GuildHallReleasedProof, RecipientReservation,
    advance_guild_disband_release_in_transaction, assert_guild_disband_released_in_transaction,
    reserve_guild_disband_recipients_in_transaction,
};

/// D1098: install only with the genuine 2141 common item/entry-fence hook and D Guild schema.
/// No runtime table/EXECUTE grant is made here. The lock-order amendment remains unaccepted.
/// These typed World-job rows deliberately contain no invented Character/session or notice.
#[allow(dead_code, reason = "Uninstalled D1098 House participant")]
pub(crate) const GUILD_DISBAND_RELEASE_SCHEMA_SQL: &str = r#"
CREATE TABLE game_house_guild_dispositions (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL,house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 owner_guild_id UUID NOT NULL CHECK(game_character_is_uuid_v7(owner_guild_id)),
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision>0),
 acquisition_receipt_ref UUID NOT NULL,
 cause TEXT NOT NULL CHECK(cause='GUILD_DISBAND'),
 state SMALLINT NOT NULL CHECK(state IN(1,2,3,5)),
 frozen_count BIGINT CHECK(frozen_count BETWEEN 0 AND 2000),
 frozen_digest BYTEA CHECK(octet_length(frozen_digest)=32),
 next_step NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(next_step BETWEEN 0 AND 18446744073709551615),
 CHECK((state=1)=(frozen_count IS NULL)),CHECK((state=1)=(frozen_digest IS NULL)),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE UNIQUE INDEX game_house_guild_disposition_live ON game_house_guild_dispositions(world_id,house_key) WHERE state IN(1,2,3);
CREATE TABLE game_house_guild_disposition_items (
 operation_id UUID NOT NULL REFERENCES game_house_guild_dispositions,item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 source_location JSONB NOT NULL,source_provenance JSONB NOT NULL,source_item JSONB NOT NULL,
 moved_step NUMERIC(20,0),moving_xact_id xid8,ordinal NUMERIC(20,0),
 PRIMARY KEY(operation_id,item_instance_id),CHECK((moved_step IS NULL)=(moving_xact_id IS NULL)),
 CHECK(ordinal IS NULL OR (ordinal>0 AND moved_step IS NOT NULL))
);
CREATE TABLE game_house_guild_disposition_steps (
 operation_id UUID NOT NULL REFERENCES game_house_guild_dispositions,step NUMERIC(20,0) NOT NULL CHECK(step>=0),
 moved_count BIGINT NOT NULL CHECK(moved_count BETWEEN 0 AND 100),created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(operation_id,step)
);
CREATE TABLE game_house_guild_recipient_reservations (
 reservation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(reservation_id)),
 operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL,owner_guild_id UUID NOT NULL,step NUMERIC(20,0) NOT NULL CHECK(step>=0),
 house_key TEXT,ownership_revision NUMERIC(20,0),property_revision NUMERIC(20,0),
 census JSONB NOT NULL CHECK(jsonb_typeof(census)='array' AND jsonb_array_length(census)<=2000),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32 AND binding=sha256(convert_to(census::text,'UTF8'))),item_ids UUID[] NOT NULL CHECK(cardinality(item_ids)<=100),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),status SMALLINT NOT NULL CHECK(status IN(1,2,3)),
 CHECK((house_key IS NULL)=(ownership_revision IS NULL)),CHECK((house_key IS NULL)=(property_revision IS NULL))
);
CREATE TABLE game_house_terminal_releases (
 operation_id UUID PRIMARY KEY REFERENCES game_house_guild_dispositions,
 world_id UUID NOT NULL,house_key TEXT NOT NULL,owner_guild_id UUID NOT NULL,
 reservation_id UUID NOT NULL UNIQUE REFERENCES game_house_guild_recipient_reservations,
 ownership_revision_before NUMERIC(20,0) NOT NULL,ownership_revision_after NUMERIC(20,0) NOT NULL,
 property_revision_before NUMERIC(20,0) NOT NULL,property_revision_after NUMERIC(20,0) NOT NULL,
 acl_revision_before NUMERIC(20,0) NOT NULL,acl_revision_after NUMERIC(20,0) NOT NULL,
 acquisition_receipt_ref UUID NOT NULL,frozen_count BIGINT NOT NULL,frozen_digest BYTEA NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(ownership_revision_after=ownership_revision_before+1),CHECK(property_revision_after=property_revision_before+1),
 CHECK(acl_revision_after=acl_revision_before+1),CHECK(frozen_count BETWEEN 0 AND 2000),CHECK(octet_length(frozen_digest)=32)
);
INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES('HOUSE_DISPOSITION') ON CONFLICT DO NOTHING;

CREATE FUNCTION game_house_guild_source(w UUID,h TEXT) RETURNS JSONB LANGUAGE SQL STABLE AS $$
 SELECT COALESCE(jsonb_agg(x ORDER BY x->>'item_instance_id'),'[]'::jsonb) FROM (
  SELECT jsonb_build_object('item_instance_id',l.item_instance_id,'source_location',to_jsonb(l),
   'source_provenance',to_jsonb(p),'source_item',to_jsonb(i)) AS x
  FROM game_item_house_interior_locations l LEFT JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
  JOIN game_item_instances i USING(item_instance_id) WHERE l.world_id=w AND l.house_key=h ORDER BY l.item_instance_id LIMIT 2001
 ) q
$$;
CREATE FUNCTION game_house_guild_reservation_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.status<>1 THEN RAISE EXCEPTION 'reservation must begin before any effects' USING ERRCODE='23514';END IF;
  NEW.created_xact_id:=pg_current_xact_id();
 ELSIF TG_OP='DELETE' OR (to_jsonb(NEW)-'status') IS DISTINCT FROM (to_jsonb(OLD)-'status')
    OR OLD.created_xact_id<>pg_current_xact_id() OR NEW.status<>OLD.status+1 THEN
  RAISE EXCEPTION 'immutable physical-transaction recipient reservation' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_guild_reservation_stamp BEFORE INSERT OR UPDATE OR DELETE ON game_house_guild_recipient_reservations
 FOR EACH ROW EXECUTE FUNCTION game_house_guild_reservation_stamp();

-- Position2 only after the proposed narrow architect amendment. Roots are deletion pins,
-- not actor/session authority. Insert evidence AFTER acquiring every actual next-item pin.
CREATE FUNCTION game_house_guild_reserve(rid UUID,op UUID,st NUMERIC,g UUID,w UUID) RETURNS TEXT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_properties;s JSONB;ids UUID[];subjects UUID[];n BIGINT;e JSONB;abi INTEGER;
BEGIN
 IF to_regprocedure('game_house_disposition_guild_fence_abi()') IS NULL THEN
  RAISE EXCEPTION 'actual common Guild fence ABI missing' USING ERRCODE='OHG05';END IF;
 EXECUTE 'SELECT game_house_disposition_guild_fence_abi()' INTO abi;
 IF abi IS DISTINCT FROM 1 THEN RAISE EXCEPTION 'unsupported common Guild fence ABI' USING ERRCODE='OHG05';END IF;
 IF st<0 OR st>18446744073709551615 THEN RAISE EXCEPTION 'invalid House step' USING ERRCODE='OHG03';END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('house-guild-disposition:'||op::text,0));
 SELECT * INTO p FROM game_house_properties WHERE world_id=w AND owner_kind='GUILD' AND owner_guild_id=g;
 IF NOT FOUND THEN
  IF EXISTS(SELECT 1 FROM game_house_guild_dispositions WHERE owner_guild_id=g AND world_id=w AND state IN(1,2,3))
    OR EXISTS(SELECT 1 FROM game_house_ownership WHERE owner_guild_id=g AND world_id=w) THEN
   RAISE EXCEPTION 'incomplete Guild ownership cannot be absence proof' USING ERRCODE='OHG01';END IF;
  s:='[]';ids:='{}';
 ELSE
  s:=game_house_guild_source(w,p.house_key);
  IF jsonb_array_length(s)>2000 THEN RAISE EXCEPTION 'House item bound' USING ERRCODE='OHG03';END IF;
  FOR e IN SELECT value FROM jsonb_array_elements(s) LOOP
   IF e->'source_provenance'='null'::jsonb OR e->'source_provenance'->>'world_id'<>w::text
      OR e->'source_provenance'->>'house_key'<>p.house_key
      OR e->'source_provenance'->>'placement_transaction_id'<>e->'source_location'->>'placed_transaction_id' THEN
    RAISE EXCEPTION 'native House source lacks reclaim provenance' USING ERRCODE='OHG03';END IF;
  END LOOP;
  SELECT COALESCE(array_agg((value->>'item_instance_id')::uuid ORDER BY ord),'{}') INTO ids
   FROM jsonb_array_elements(s) WITH ORDINALITY t(value,ord) WHERE ord<=100;
  SELECT COALESCE(array_agg(subject ORDER BY subject),'{}') INTO subjects FROM (
   SELECT DISTINCT (value->'source_provenance'->>'reclaim_subject_character_id')::uuid AS subject
   FROM jsonb_array_elements(s) WITH ORDINALITY t(value,ord) WHERE ord<=100) q;
  PERFORM character_id FROM game_character_roots WHERE character_id=ANY(subjects) AND world_id=w ORDER BY character_id FOR KEY SHARE;
  GET DIAGNOSTICS n=ROW_COUNT;
  IF n<>cardinality(subjects) THEN RAISE EXCEPTION 'recipient deletion pin missing' USING ERRCODE='OHG01';END IF;
 END IF;
 INSERT INTO game_house_guild_recipient_reservations(reservation_id,operation_id,world_id,owner_guild_id,step,
  house_key,ownership_revision,property_revision,census,binding,item_ids,status)
 VALUES(rid,op,w,g,st,p.house_key,p.ownership_revision,p.revision,s,sha256(convert_to(s::text,'UTF8')),ids,1);
 RETURN pg_current_xact_id()::text;
END $$;

-- The REAL2141 common fence calls this hook for both native item and entry mutations.
CREATE FUNCTION game_house_guild_disposition_fence_check(w UUID,h TEXT,i UUID,deleting BOOLEAN) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE op UUID;
BEGIN
 SELECT operation_id INTO op FROM game_house_guild_dispositions WHERE world_id=w AND house_key=h AND state IN(2,3);
 IF FOUND AND NOT (deleting AND EXISTS(SELECT 1 FROM game_house_guild_disposition_items c
  JOIN game_house_guild_recipient_reservations r USING(operation_id)
  WHERE c.operation_id=op AND c.item_instance_id=i AND c.moved_step=r.step AND c.moving_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND r.status=2 AND i=ANY(r.item_ids))) THEN
  RAISE EXCEPTION 'Guildhall disposition content fence' USING ERRCODE='23514';END IF;
END $$;
CREATE FUNCTION game_house_guild_disposition_transfer_proven(op UUID,st NUMERIC,it UUID) RETURNS BOOLEAN
LANGUAGE SQL STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_guild_disposition_items c JOIN game_house_guild_dispositions o USING(operation_id)
 JOIN game_house_guild_disposition_steps s ON s.operation_id=c.operation_id AND s.step=c.moved_step
 JOIN game_character_inbox_deliveries d ON d.item_instance_id=c.item_instance_id AND d.world_id=o.world_id
  AND d.character_id=(c.source_provenance->>'reclaim_subject_character_id')::uuid AND d.ordinal=c.ordinal
  AND d.cause_kind='HOUSE_DISPOSITION' AND d.cause_ref=op::text AND d.created_xact_id=c.moving_xact_id
 WHERE c.operation_id=op AND c.item_instance_id=it AND c.moved_step=st AND s.created_xact_id=c.moving_xact_id
  AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=it AND world_id=o.world_id AND house_key=o.house_key)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=it AND placement_transaction_id=(c.source_provenance->>'placement_transaction_id')::uuid))
$$;
CREATE FUNCTION game_house_terminal_release_proven(op UUID) RETURNS BOOLEAN
LANGUAGE SQL STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_terminal_releases j JOIN game_house_guild_dispositions o USING(operation_id)
 JOIN game_house_guild_recipient_reservations r USING(reservation_id)
 JOIN game_house_properties p ON p.world_id=j.world_id AND p.house_key=j.house_key
 JOIN game_house_ownership a ON a.world_id=j.world_id AND a.house_key=j.house_key
 WHERE j.operation_id=op AND j.created_xact_id=pg_current_xact_id() AND r.created_xact_id=j.created_xact_id AND r.status=3
  AND r.operation_id=op AND r.owner_guild_id=j.owner_guild_id AND r.world_id=j.world_id AND r.house_key=j.house_key
  AND o.owner_guild_id=j.owner_guild_id AND o.world_id=j.world_id AND o.house_key=j.house_key AND o.cause='GUILD_DISBAND' AND o.state IN(3,5)
  AND o.ownership_revision=j.ownership_revision_before AND o.acquisition_receipt_ref=j.acquisition_receipt_ref
  AND o.frozen_count=j.frozen_count AND o.frozen_digest=j.frozen_digest
  AND p.state='VACANT' AND p.owner_kind IS NULL AND p.owner_guild_id IS NULL AND p.owner_character_id IS NULL AND p.owner_account_id IS NULL
  AND p.content_fence_operation IS NULL AND p.ownership_revision=j.ownership_revision_after AND p.revision=j.property_revision_after
  AND p.acl_revision=j.acl_revision_after AND p.acquisition_receipt_ref=j.acquisition_receipt_ref
  AND a.owner_guild_id IS NULL AND a.owner_character_id IS NULL AND a.ownership_revision=j.ownership_revision_after
  AND a.acquisition_receipt_ref=j.acquisition_receipt_ref
  AND NOT EXISTS(SELECT 1 FROM game_account_housing_slots WHERE world_id=j.world_id AND house_key=j.house_key)
  AND NOT EXISTS(SELECT 1 FROM game_house_acl WHERE world_id=j.world_id AND house_key=j.house_key)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=j.world_id AND house_key=j.house_key)
  AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE world_id=j.world_id AND house_key=j.house_key)
  AND j.frozen_count=(SELECT count(*) FROM game_house_guild_disposition_items WHERE operation_id=op)
  AND j.frozen_digest=(SELECT sha256(COALESCE(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea)) FROM game_house_guild_disposition_items WHERE operation_id=op)
  AND NOT EXISTS(SELECT 1 FROM game_house_guild_disposition_items c WHERE c.operation_id=op AND
   (c.moved_step IS NULL OR NOT game_house_guild_disposition_transfer_proven(op,c.moved_step,c.item_instance_id)))
  AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE runtime_scope_kind=2 AND runtime_scope_world_id=j.world_id AND runtime_scope_house_key=j.house_key AND session_state IN(1,2))
  AND NOT EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=j.world_id AND house_key=j.house_key AND state IN(1,2))
  AND NOT EXISTS(SELECT 1 FROM game_house_bed_sleepers WHERE world_id=j.world_id AND house_key=j.house_key AND freed_at IS NULL))
$$;

CREATE FUNCTION game_house_guild_census_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Guild census retained' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' THEN
  IF NOT EXISTS(SELECT 1 FROM game_house_guild_dispositions o JOIN game_item_house_interior_locations l ON l.world_id=o.world_id AND l.house_key=o.house_key
   JOIN game_item_house_reclaim_provenance p USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
   WHERE o.operation_id=NEW.operation_id AND l.item_instance_id=NEW.item_instance_id AND o.state=1
    AND to_jsonb(l)=NEW.source_location AND to_jsonb(p)=NEW.source_provenance AND to_jsonb(i)=NEW.source_item
    AND p.world_id=l.world_id AND p.house_key=l.house_key AND p.placement_transaction_id=l.placed_transaction_id
    AND NEW.moved_step IS NULL AND NEW.ordinal IS NULL) THEN
   RAISE EXCEPTION 'Guild census must capture native custody' USING ERRCODE='23514';END IF;
 ELSIF (to_jsonb(NEW)-ARRAY['moved_step','moving_xact_id','ordinal']) IS DISTINCT FROM
       (to_jsonb(OLD)-ARRAY['moved_step','moving_xact_id','ordinal']) OR NOT
   ((OLD.moved_step IS NULL AND NEW.moved_step IS NOT NULL AND NEW.moving_xact_id=pg_current_xact_id() AND NEW.ordinal IS NULL)
    OR(OLD.moved_step=NEW.moved_step AND OLD.moving_xact_id=pg_current_xact_id() AND NEW.moving_xact_id=OLD.moving_xact_id AND OLD.ordinal IS NULL AND NEW.ordinal IS NOT NULL)) THEN
  RAISE EXCEPTION 'Guild census is immutable except exact delivery' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_guild_census_guard BEFORE INSERT OR UPDATE OR DELETE ON game_house_guild_disposition_items
 FOR EACH ROW EXECUTE FUNCTION game_house_guild_census_guard();
CREATE FUNCTION game_house_guild_operation_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE n BIGINT;d BYTEA;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Guild disposition retained' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.state<>1 OR NEW.next_step<>0 OR NEW.frozen_count IS NOT NULL OR NEW.frozen_digest IS NOT NULL
   OR NOT EXISTS(SELECT 1 FROM game_house_properties p JOIN game_guilds g ON g.guild_id=p.owner_guild_id
    WHERE p.world_id=NEW.world_id AND p.house_key=NEW.house_key AND p.owner_kind='GUILD'
     AND p.owner_guild_id=NEW.owner_guild_id AND g.world_id=p.world_id AND g.state=3
     AND p.ownership_revision=NEW.ownership_revision AND p.acquisition_receipt_ref=NEW.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'Guild disposition requires actual current owner/source and initial state' USING ERRCODE='23514';END IF;
  RETURN NEW;
 END IF;
 IF (to_jsonb(NEW)-ARRAY['state','frozen_count','frozen_digest','next_step']) IS DISTINCT FROM
    (to_jsonb(OLD)-ARRAY['state','frozen_count','frozen_digest','next_step']) THEN
  RAISE EXCEPTION 'Guild disposition binding immutable' USING ERRCODE='23514';END IF;
 SELECT count(*),sha256(COALESCE(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea)) INTO n,d
  FROM game_house_guild_disposition_items WHERE operation_id=NEW.operation_id;
 IF NEW.frozen_count<>n OR NEW.frozen_digest<>d THEN RAISE EXCEPTION 'Guild frozen set changed' USING ERRCODE='23514';END IF;
 IF OLD.state=1 AND NEW.state=2 AND NEW.next_step=0 THEN
  IF n<>(SELECT count(*) FROM game_item_house_interior_locations WHERE world_id=NEW.world_id AND house_key=NEW.house_key)
   OR EXISTS(SELECT 1 FROM game_house_guild_disposition_items c LEFT JOIN game_item_house_interior_locations l USING(item_instance_id)
    LEFT JOIN game_item_house_reclaim_provenance p USING(item_instance_id) LEFT JOIN game_item_instances i USING(item_instance_id)
    WHERE c.operation_id=NEW.operation_id AND (to_jsonb(l) IS DISTINCT FROM c.source_location OR to_jsonb(p) IS DISTINCT FROM c.source_provenance OR to_jsonb(i) IS DISTINCT FROM c.source_item)) THEN
   RAISE EXCEPTION 'Guild freeze differs from native full source' USING ERRCODE='23514';END IF;
 ELSIF OLD.state=2 AND NEW.state IN(2,3) AND NEW.next_step=OLD.next_step+1 AND NEW.frozen_count=OLD.frozen_count AND NEW.frozen_digest=OLD.frozen_digest THEN
  IF NOT EXISTS(SELECT 1 FROM game_house_guild_disposition_steps s WHERE s.operation_id=NEW.operation_id AND s.step=OLD.next_step AND s.created_xact_id=pg_current_xact_id()
   AND s.moved_count=(SELECT count(*) FROM game_house_guild_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.moved_step=s.step AND c.moving_xact_id=s.created_xact_id))
   OR ((NEW.state=3)<>NOT EXISTS(SELECT 1 FROM game_house_guild_disposition_items WHERE operation_id=NEW.operation_id AND moved_step IS NULL)) THEN
   RAISE EXCEPTION 'Guild step lacks exact source/step proof' USING ERRCODE='23514';END IF;
 ELSIF NOT(OLD.state=3 AND NEW.state=5 AND NEW.next_step=OLD.next_step AND NEW.frozen_count=OLD.frozen_count
   AND NEW.frozen_digest=OLD.frozen_digest AND game_house_terminal_release_proven(NEW.operation_id)) THEN
  RAISE EXCEPTION 'Guild release needs actual current terminal proof' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_guild_operation_guard BEFORE INSERT OR UPDATE OR DELETE ON game_house_guild_dispositions
 FOR EACH ROW EXECUTE FUNCTION game_house_guild_operation_guard();
CREATE FUNCTION game_house_guild_retained() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'House release/step evidence immutable' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_house_terminal_retained BEFORE UPDATE OR DELETE ON game_house_terminal_releases FOR EACH ROW EXECUTE FUNCTION game_house_guild_retained();
CREATE TRIGGER game_house_guild_step_retained BEFORE UPDATE OR DELETE ON game_house_guild_disposition_steps FOR EACH ROW EXECUTE FUNCTION game_house_guild_retained();
CREATE TRIGGER game_house_terminal_stamp BEFORE INSERT ON game_house_terminal_releases FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_house_guild_step_stamp BEFORE INSERT ON game_house_guild_disposition_steps FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE FUNCTION game_house_guild_delivery_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE c game_house_guild_disposition_items;
BEGIN
 -- Deferred events hold intermediate NEW tuples; consume the actual final row, including
 -- the ordinal assigned after native delivery, rather than the earlier moving stamp.
 SELECT * INTO STRICT c FROM game_house_guild_disposition_items WHERE operation_id=NEW.operation_id AND item_instance_id=NEW.item_instance_id;
 IF c.moved_step IS NOT NULL AND (NOT game_house_guild_disposition_transfer_proven(NEW.operation_id,c.moved_step,NEW.item_instance_id)
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_character_inbox_locations l USING(item_instance_id) WHERE i.item_instance_id=NEW.item_instance_id AND to_jsonb(i)=c.source_item AND l.ordinal=c.ordinal AND l.character_id=(c.source_provenance->>'reclaim_subject_character_id')::uuid)) THEN
  RAISE EXCEPTION 'Guild item lacks real native Inbox proof' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_guild_delivery_guard AFTER UPDATE ON game_house_guild_disposition_items
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_guild_delivery_guard();
CREATE FUNCTION game_house_terminal_commit_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_house_terminal_release_proven(NEW.operation_id) OR NOT EXISTS(SELECT 1 FROM game_house_guild_dispositions WHERE operation_id=NEW.operation_id AND state=5) THEN
  RAISE EXCEPTION 'terminal House journal lacks all actual release participants' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_terminal_commit_guard AFTER INSERT ON game_house_terminal_releases
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_terminal_commit_guard();
CREATE FUNCTION game_house_guild_property_release_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_house_properties;
BEGIN
 SELECT * INTO p FROM game_house_properties WHERE world_id=NEW.world_id AND house_key=NEW.house_key;
 IF OLD.owner_guild_id IS NOT NULL AND p.owner_guild_id IS NULL AND NOT EXISTS(
  SELECT 1 FROM game_house_terminal_releases j WHERE j.world_id=p.world_id AND j.house_key=p.house_key
   AND j.owner_guild_id=OLD.owner_guild_id AND j.ownership_revision_before=OLD.ownership_revision
   AND game_house_terminal_release_proven(j.operation_id)) THEN
  RAISE EXCEPTION 'Guild owner release requires actual House terminal participants' USING ERRCODE='23514';END IF;
 IF p.owner_kind='GUILD' AND p.state='DISPOSITION' AND NOT EXISTS(
  SELECT 1 FROM game_house_guild_dispositions o WHERE o.operation_id=p.content_fence_operation
   AND o.world_id=p.world_id AND o.house_key=p.house_key AND o.owner_guild_id=p.owner_guild_id
   AND o.ownership_revision=p.ownership_revision AND o.state IN(2,3)) THEN
  RAISE EXCEPTION 'Guild property disposition requires actual common content fence' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_guild_property_release_guard AFTER UPDATE ON game_house_properties
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_guild_property_release_guard();

-- Exact bounded privileged participant, never a general SQL executor. Runtime receives
-- EXECUTE only; canonical, ACL, BED and protected evidence table grants remain revoked.
CREATE FUNCTION game_house_guild_reservation_binding(rid UUID) RETURNS BYTEA
LANGUAGE SQL STABLE SECURITY DEFINER AS $$
 SELECT binding FROM game_house_guild_recipient_reservations WHERE reservation_id=rid
 AND created_xact_id=pg_current_xact_id() AND status=1
$$;
-- Root/D provided expectations, never reconstructed from current database records.
-- Secret possession is checked natively; only its digest is retained in the stamp.
CREATE TABLE game_house_guild_authority_stamps (
 reservation_id UUID PRIMARY KEY REFERENCES game_house_guild_recipient_reservations,
 recovery JSONB NOT NULL,node_id UUID NOT NULL,node_revision NUMERIC(20,0) NOT NULL,
 node_secret_digest BYTEA NOT NULL CHECK(octet_length(node_secret_digest)=32),
 assignment_scope_key BYTEA NOT NULL,assignment_generation NUMERIC(20,0) NOT NULL,
 guild_revision NUMERIC(20,0) NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TRIGGER game_house_guild_authority_retained BEFORE UPDATE OR DELETE ON game_house_guild_authority_stamps
 FOR EACH ROW EXECUTE FUNCTION game_house_guild_retained();
CREATE TRIGGER game_house_guild_authority_stamp BEFORE INSERT ON game_house_guild_authority_stamps
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE FUNCTION game_house_guild_current_recovery() RETURNS JSONB LANGUAGE SQL VOLATILE SECURITY DEFINER AS $$
 SELECT jsonb_build_object('authority_scope_id',authority_scope_id,'recovery_generation',recovery_generation::TEXT,
 'recovery_event_id',recovery_event_id::TEXT,'predecessor_generation',predecessor_generation::TEXT,
 'predecessor_digest',encode(predecessor_digest,'hex'),'issued_at',issued_at::TEXT,'issuer_identity',issuer_identity)
 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE
$$;
CREATE FUNCTION game_house_guild_admission_locks() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE rel TEXT;
BEGIN
 FOREACH rel IN ARRAY ARRAY[
 'game_durability_admission_account_guards','game_durability_admission_character_guards',
 'game_durability_admission_guard_history','game_durability_admission_lifecycle_receipts',
 'game_durability_admission_runtime_guards','game_durability_admission_signing_trust_guards',
 'game_durability_control_loss_continuity','game_durability_executor_custody',
 'game_durability_fresh_admission_receipts','game_durability_reconnect_attempts',
 'game_durability_reconnect_pending_commands','game_durability_reconnect_sessions',
 'game_durability_recovery_grant_consumptions','game_durability_session_replacements',
 'game_durability_session_use_ledgers','game_durability_session_use_memberships',
 'game_durability_transport_ref_reservations'] LOOP
  EXECUTE format('LOCK TABLE %I.%I IN EXCLUSIVE MODE',current_schema(),rel);
 END LOOP;
END $$;
CREATE FUNCTION game_house_guild_authority_current(rid UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE s game_house_guild_authority_stamps;r game_house_guild_recipient_reservations;
BEGIN
 SELECT * INTO s FROM game_house_guild_authority_stamps WHERE reservation_id=rid AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE;END IF;
 SELECT * INTO r FROM game_house_guild_recipient_reservations WHERE reservation_id=rid AND created_xact_id=s.created_xact_id;
 IF NOT FOUND OR game_house_guild_current_recovery() IS DISTINCT FROM s.recovery THEN RETURN FALSE;END IF;
 PERFORM game_house_guild_admission_locks();
 -- A protected sameTX stamp proves possession checked before all subsequent locks.
 -- Recheck current registration/endings/history against that original supplied proof.
 IF NOT game_node_registration_history_valid() THEN RETURN FALSE;END IF;
 PERFORM node_id FROM game_node_registrations reg WHERE reg.node_id=s.node_id AND reg.registration_revision=s.node_revision
  AND reg.state=1 AND reg.authorization_digest=s.node_secret_digest
  AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings e WHERE e.node_id=reg.node_id) FOR SHARE;
 IF NOT FOUND THEN RETURN FALSE;END IF;
 PERFORM scope_key FROM game_runtime_scope_assignments a WHERE a.scope_key=s.assignment_scope_key AND a.world_id=r.world_id
  AND a.scope_kind=1 AND a.state=1 AND a.holder_node_id=s.node_id AND a.holder_registration_revision=s.node_revision
  AND a.ownership_generation=s.assignment_generation FOR SHARE;
 IF NOT FOUND THEN RETURN FALSE;END IF;
 RETURN EXISTS(SELECT 1 FROM game_guilds guild WHERE guild.guild_id=r.owner_guild_id AND guild.world_id=r.world_id
  AND ((guild.state=3 AND guild.revision=s.guild_revision) OR
   (guild.revision=s.guild_revision+1 AND EXISTS(SELECT 1 FROM game_guild_disband_steps ds
    JOIN game_guild_disband_step_events ev USING(guild_id,step)
    WHERE ds.guild_id=guild.guild_id AND ds.world_id=guild.world_id AND ds.previous_revision=s.guild_revision
     AND ds.revision=guild.revision AND ds.created_xact_id=s.created_xact_id AND ev.created_xact_id=s.created_xact_id
     AND guild.state=CASE WHEN ds.phase=6 THEN 4 ELSE 3 END
     AND ev.payload_sha256=sha256(ev.payload) AND convert_from(ev.payload,'UTF8')::jsonb=game_guild_disband_step_payload(ds)))));
END $$;
CREATE FUNCTION game_house_guild_lock_spine(rid UUID,op UUID,st NUMERIC,g UUID,w UUID,b BYTEA,x TEXT,
 a JSONB,n UUID,nr NUMERIC,secret BYTEA) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE assigned RECORD;gr NUMERIC;
BEGIN
 PERFORM reservation_id FROM game_house_guild_recipient_reservations rr WHERE rr.reservation_id=rid AND rr.operation_id=op
  AND rr.step=st AND rr.owner_guild_id=g AND rr.world_id=w AND rr.binding=b AND rr.created_xact_id=pg_current_xact_id()
  AND rr.created_xact_id::text=x AND rr.status=1 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'reservation no longer current' USING ERRCODE='OHG02';END IF;
 IF a IS NULL OR a IS DISTINCT FROM game_house_guild_current_recovery() THEN
  RAISE EXCEPTION 'Root recovery fence is no longer current' USING ERRCODE='OHG02';END IF;
 PERFORM game_house_guild_admission_locks();
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Root node incarnation possession is no longer current' USING ERRCODE='OHG02';END IF;
 SELECT scope_key,ownership_generation INTO assigned FROM game_runtime_scope_assignments
  WHERE world_id=w AND scope_kind=1 AND state=1 AND holder_node_id=n AND holder_registration_revision=nr
  ORDER BY scope_key LIMIT 1 FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'node has no current assignment for this World' USING ERRCODE='OHG02';END IF;
 SELECT revision INTO gr FROM game_guilds WHERE guild_id=g AND world_id=w AND state=3 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'Guild no longer current' USING ERRCODE='OHG02';END IF;
 PERFORM character_id FROM game_guild_members WHERE guild_id=g ORDER BY character_id FOR UPDATE;
 PERFORM character_id FROM game_guild_invitations WHERE guild_id=g ORDER BY character_id FOR UPDATE;
 PERFORM account_id FROM game_account_guild_leadership WHERE guild_id=g ORDER BY account_id FOR UPDATE;
 -- LAST: rollback losing any authority/spine lock also removes this protected marker.
 INSERT INTO game_house_guild_authority_stamps(reservation_id,recovery,node_id,node_revision,node_secret_digest,
  assignment_scope_key,assignment_generation,guild_revision)
 VALUES(rid,a,n,nr,sha256(secret),assigned.scope_key,assigned.ownership_generation,gr);
END $$;
CREATE FUNCTION game_house_guild_assert_released(rid UUID,op UUID,st NUMERIC,g UUID,w UUID,b BYTEA,x TEXT) RETURNS BOOLEAN
LANGUAGE SQL VOLATILE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_guild_recipient_reservations r JOIN game_guilds guild
  ON guild.guild_id=r.owner_guild_id AND guild.world_id=r.world_id AND guild.state IN(3,4)
 WHERE r.reservation_id=rid AND r.operation_id=op AND r.step=st AND r.owner_guild_id=g AND r.world_id=w
 AND game_house_guild_authority_current(r.reservation_id) AND r.binding=b AND r.created_xact_id=pg_current_xact_id() AND r.created_xact_id::text=x AND r.status=3
 AND NOT EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=g AND world_id=w)
 AND NOT EXISTS(SELECT 1 FROM game_house_ownership WHERE owner_guild_id=g AND world_id=w)
 AND NOT EXISTS(SELECT 1 FROM game_house_guild_dispositions WHERE owner_guild_id=g AND world_id=w AND state IN(1,2,3))
 AND (r.house_key IS NULL OR (game_house_terminal_release_proven(op)
  AND EXISTS(SELECT 1 FROM game_house_guild_dispositions WHERE operation_id=op AND state=5))))
$$;
CREATE FUNCTION game_house_guild_disband_released_proven(g UUID,w UUID) RETURNS BOOLEAN
LANGUAGE SQL VOLATILE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_guild_recipient_reservations r WHERE r.owner_guild_id=g AND r.world_id=w
 AND game_house_guild_assert_released(r.reservation_id,r.operation_id,r.step,g,w,r.binding,r.created_xact_id::text))
$$;
CREATE FUNCTION game_house_guild_advance(rid UUID,op UUID,st NUMERIC,g UUID,w UUID,b BYTEA,x TEXT)
RETURNS TABLE(moved BIGINT,next_step NUMERIC,released BOOLEAN) LANGUAGE plpgsql SECURITY DEFINER AS $$
#variable_conflict use_variable
DECLARE r game_house_guild_recipient_reservations;p game_house_properties;o game_house_guild_dispositions;
 c game_house_guild_disposition_items;it UUID;subject UUID;ord NUMERIC;n BIGINT:=0;abi INTEGER;bed_revision NUMERIC;freed BIGINT;
BEGIN
 IF to_regprocedure('game_house_disposition_guild_fence_abi()') IS NULL THEN
  RAISE EXCEPTION 'common Guild fence ABI missing' USING ERRCODE='OHG05';END IF;
 EXECUTE 'SELECT game_house_disposition_guild_fence_abi()' INTO abi;
 IF abi IS DISTINCT FROM 1 THEN RAISE EXCEPTION 'common Guild fence ABI unsupported' USING ERRCODE='OHG05';END IF;
 SELECT * INTO r FROM game_house_guild_recipient_reservations rr WHERE rr.reservation_id=rid AND rr.operation_id=op
  AND rr.step=st AND rr.owner_guild_id=g AND rr.world_id=w AND rr.binding=b AND rr.created_xact_id=pg_current_xact_id()
  AND rr.created_xact_id::text=x AND rr.status=1 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'reservation no longer current' USING ERRCODE='OHG02';END IF;
 IF NOT game_house_guild_authority_current(rid) OR NOT EXISTS(SELECT 1 FROM game_house_guild_authority_stamps auth
  JOIN game_guilds guild ON guild.guild_id=g AND guild.world_id=w AND guild.state=3 AND guild.revision=auth.guild_revision
  WHERE auth.reservation_id=rid AND auth.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Root authority/spine stamp no longer current' USING ERRCODE='OHG02';END IF;
 -- Reacquire actual D spine: a genuine D in-memory xid can survive rollback of its locks.
 PERFORM guild_id FROM game_guilds WHERE guild_id=g AND world_id=w AND state=3 FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'Guild no longer disbanding in this World' USING ERRCODE='OHG02';END IF;
 PERFORM character_id FROM game_guild_members WHERE guild_id=g ORDER BY character_id FOR UPDATE;
 PERFORM character_id FROM game_guild_invitations WHERE guild_id=g ORDER BY character_id FOR UPDATE;
 PERFORM account_id FROM game_account_guild_leadership WHERE guild_id=g ORDER BY account_id FOR UPDATE;
 IF r.house_key IS NULL THEN
  IF EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=g AND world_id=w)
   OR EXISTS(SELECT 1 FROM game_house_ownership WHERE owner_guild_id=g AND world_id=w)
   OR EXISTS(SELECT 1 FROM game_house_guild_dispositions WHERE owner_guild_id=g AND world_id=w AND state IN(1,2,3)) THEN
   RAISE EXCEPTION 'Hall absence changed' USING ERRCODE='OHG01';END IF;
  UPDATE game_house_guild_recipient_reservations SET status=2 WHERE reservation_id=rid;
  UPDATE game_house_guild_recipient_reservations SET status=3 WHERE reservation_id=rid;
  RETURN QUERY SELECT 0::BIGINT,st,TRUE;RETURN;
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=w AND house_key=r.house_key AND owner_kind='GUILD' AND owner_guild_id=g FOR UPDATE;
 IF NOT FOUND OR p.revision<>r.property_revision OR p.ownership_revision<>r.ownership_revision THEN
  RAISE EXCEPTION 'canonical owner generation changed' USING ERRCODE='OHG01';END IF;
 PERFORM world_id FROM game_house_ownership WHERE world_id=w AND house_key=r.house_key AND owner_guild_id=g
  AND owner_character_id IS NULL AND ownership_revision=p.ownership_revision AND acquisition_receipt_ref=p.acquisition_receipt_ref FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'legacy anchor does not bind canonical owner' USING ERRCODE='OHG03';END IF;
 IF r.census IS DISTINCT FROM game_house_guild_source(w,r.house_key) THEN
  RAISE EXCEPTION 'source census changed; rollback and reserve again' USING ERRCODE='OHG01';END IF;
 IF (SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal AND tgenabled IN('O','A') AND
  ((tgrelid='game_item_house_interior_locations'::regclass AND tgname='a_house_disposition_content_fence' AND tgfoid='game_house_disposition_content_fence()'::regprocedure AND tgtype=31)
   OR(tgrelid='game_item_house_reclaim_provenance'::regclass AND tgname='a_house_disposition_provenance_fence' AND tgfoid='game_house_disposition_content_fence()'::regprocedure AND tgtype=31)
   OR(tgrelid='game_item_instances'::regclass AND tgname='a_house_disposition_item_fence' AND tgfoid='game_house_disposition_item_fence()'::regprocedure AND tgtype=27)
   OR(tgrelid='game_house_scope_handoffs'::regclass AND tgname='a_house_disposition_entry_fence' AND tgfoid='game_house_disposition_entry_fence()'::regprocedure AND tgtype=23)))<>4 THEN
  RAISE EXCEPTION 'actual native item/entry hooks absent' USING ERRCODE='OHG05';END IF;
 IF EXISTS(SELECT 1 FROM game_house_dispositions WHERE world_id=w AND house_key=r.house_key AND state IN(2,3)) THEN
  RAISE EXCEPTION 'ordinary disposition retains its independent fence' USING ERRCODE='OHG05';END IF;
 IF EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE runtime_scope_kind=2 AND runtime_scope_world_id=w AND runtime_scope_house_key=r.house_key AND session_state IN(1,2))
  OR EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=w AND house_key=r.house_key AND state IN(1,2)) THEN
  RAISE EXCEPTION 'Hall has current occupants or admission reservations' USING ERRCODE='OHG04';END IF;
 SELECT * INTO o FROM game_house_guild_dispositions WHERE operation_id=op FOR UPDATE;
 IF FOUND AND (o.world_id<>w OR o.house_key<>r.house_key OR o.owner_guild_id<>g OR o.ownership_revision<>p.ownership_revision
  OR o.acquisition_receipt_ref<>p.acquisition_receipt_ref OR o.next_step<>st) THEN
  RAISE EXCEPTION 'operation binding changed' USING ERRCODE='OHG01';END IF;
 UPDATE game_house_guild_recipient_reservations SET status=2 WHERE reservation_id=rid;
 IF o.operation_id IS NULL THEN
  IF st<>0 OR p.state NOT IN('OWNED','MOVE_OUT_PENDING') THEN RAISE EXCEPTION 'initial lifecycle changed' USING ERRCODE='OHG01';END IF;
  INSERT INTO game_house_guild_dispositions(operation_id,world_id,house_key,owner_guild_id,ownership_revision,acquisition_receipt_ref,cause,state)
   VALUES(op,w,r.house_key,g,p.ownership_revision,p.acquisition_receipt_ref,'GUILD_DISBAND',1);
  INSERT INTO game_house_guild_disposition_items(operation_id,item_instance_id,source_location,source_provenance,source_item)
   SELECT op,l.item_instance_id,to_jsonb(l),to_jsonb(pr),to_jsonb(i) FROM game_item_house_interior_locations l
   JOIN game_item_house_reclaim_provenance pr USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
   WHERE l.world_id=w AND l.house_key=r.house_key;
  SELECT revision INTO bed_revision FROM game_house_bed_revisions WHERE world_id=w AND house_key=r.house_key FOR UPDATE;
  IF NOT FOUND AND EXISTS(SELECT 1 FROM game_house_bed_sleepers WHERE world_id=w AND house_key=r.house_key) THEN
   RAISE EXCEPTION 'native BED revision missing' USING ERRCODE='OHG03';END IF;
  UPDATE game_house_bed_sleepers SET freed_at=floor(extract(epoch FROM clock_timestamp())*1000)::BIGINT,freed_cause='disposition'
   WHERE world_id=w AND house_key=r.house_key AND freed_at IS NULL;
  -- The native BED per-row sleeper trigger advances the locked occupancy revision; prove it.
  GET DIAGNOSTICS freed=ROW_COUNT;
  IF freed>0 AND NOT EXISTS(SELECT 1 FROM game_house_bed_revisions WHERE world_id=w AND house_key=r.house_key AND revision>bed_revision) THEN
   RAISE EXCEPTION 'native BED occupancy revision hook absent' USING ERRCODE='OHG05';END IF;
  UPDATE game_house_properties SET state='DISPOSITION',content_fence_operation=op,revision=revision+1 WHERE world_id=w AND house_key=r.house_key;
  UPDATE game_house_guild_dispositions SET state=2,frozen_count=(SELECT count(*) FROM game_house_guild_disposition_items WHERE operation_id=op),
   frozen_digest=(SELECT sha256(COALESCE(string_agg(uuid_send(item_instance_id),''::BYTEA ORDER BY item_instance_id),''::BYTEA)) FROM game_house_guild_disposition_items WHERE operation_id=op)
   WHERE operation_id=op;
  SELECT * INTO o FROM game_house_guild_dispositions WHERE operation_id=op;
 END IF;
 IF o.state=2 THEN
  -- All actual ItemInstance locks precede the sorted Inbox counters. The native House
  -- custody guard forbids containers with children; no invented container mail path.
  PERFORM i.item_instance_id FROM game_item_instances i JOIN game_house_guild_disposition_items ci USING(item_instance_id)
   WHERE ci.operation_id=op AND ci.moved_step IS NULL AND ci.item_instance_id=ANY(r.item_ids) ORDER BY i.item_instance_id FOR NO KEY UPDATE OF i;
  INSERT INTO game_character_inbox_counters(character_id) SELECT DISTINCT (source_provenance->>'reclaim_subject_character_id')::UUID
   FROM game_house_guild_disposition_items WHERE operation_id=op AND moved_step IS NULL AND item_instance_id=ANY(r.item_ids) ORDER BY 1 ON CONFLICT DO NOTHING;
  PERFORM character_id FROM game_character_inbox_counters WHERE character_id IN(
   SELECT (source_provenance->>'reclaim_subject_character_id')::UUID FROM game_house_guild_disposition_items
    WHERE operation_id=op AND moved_step IS NULL AND item_instance_id=ANY(r.item_ids)) ORDER BY character_id FOR UPDATE;
  FOR c IN SELECT * FROM game_house_guild_disposition_items WHERE operation_id=op AND moved_step IS NULL AND item_instance_id=ANY(r.item_ids) ORDER BY item_instance_id FOR UPDATE LOOP
   IF NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance pr USING(item_instance_id)
    JOIN game_item_instances i USING(item_instance_id) WHERE l.item_instance_id=c.item_instance_id
    AND to_jsonb(l)=c.source_location AND to_jsonb(pr)=c.source_provenance AND to_jsonb(i)=c.source_item) THEN
    RAISE EXCEPTION 'native custody changed' USING ERRCODE='OHG03';END IF;
   UPDATE game_house_guild_disposition_items SET moved_step=st,moving_xact_id=pg_current_xact_id() WHERE operation_id=op AND item_instance_id=c.item_instance_id;
   DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=c.item_instance_id;
   DELETE FROM game_item_house_interior_locations WHERE item_instance_id=c.item_instance_id;
   SELECT d.ordinal INTO ord FROM game_character_inbox_deliver(c.item_instance_id,(c.source_provenance->>'reclaim_subject_character_id')::UUID,w,'HOUSE_DISPOSITION',op::TEXT) d;
   UPDATE game_house_guild_disposition_items SET ordinal=ord WHERE operation_id=op AND item_instance_id=c.item_instance_id;
   n:=n+1;
  END LOOP;
  INSERT INTO game_house_guild_disposition_steps(operation_id,step,moved_count) VALUES(op,st,n);
  UPDATE game_house_guild_dispositions AS live SET next_step=live.next_step+1,state=CASE WHEN EXISTS(
   SELECT 1 FROM game_house_guild_disposition_items WHERE operation_id=op AND moved_step IS NULL) THEN 2 ELSE 3 END WHERE operation_id=op;
 ELSIF o.state<>3 THEN RAISE EXCEPTION 'invalid frozen lifecycle' USING ERRCODE='OHG03';END IF;
 IF EXISTS(SELECT 1 FROM game_house_guild_disposition_items WHERE operation_id=op AND moved_step IS NULL) THEN
  UPDATE game_house_guild_recipient_reservations SET status=3 WHERE reservation_id=rid;
  RETURN QUERY SELECT n,st+1,FALSE;RETURN;
 END IF;
 IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=w AND house_key=r.house_key)
  OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE world_id=w AND house_key=r.house_key)
  OR EXISTS(SELECT 1 FROM game_account_housing_slots WHERE world_id=w AND house_key=r.house_key)
  OR EXISTS(SELECT 1 FROM game_house_bed_sleepers WHERE world_id=w AND house_key=r.house_key AND freed_at IS NULL) THEN
  RAISE EXCEPTION 'terminal custody/slot/BED participants incomplete' USING ERRCODE='OHG03';END IF;
 DELETE FROM game_house_acl WHERE world_id=w AND house_key=r.house_key;
 INSERT INTO game_house_terminal_releases(operation_id,world_id,house_key,owner_guild_id,reservation_id,ownership_revision_before,ownership_revision_after,
  property_revision_before,property_revision_after,acl_revision_before,acl_revision_after,acquisition_receipt_ref,frozen_count,frozen_digest)
 SELECT op,w,r.house_key,g,rid,pp.ownership_revision,pp.ownership_revision+1,pp.revision,pp.revision+1,pp.acl_revision,pp.acl_revision+1,pp.acquisition_receipt_ref,oo.frozen_count,oo.frozen_digest
 FROM game_house_properties pp JOIN game_house_guild_dispositions oo USING(world_id,house_key) WHERE pp.world_id=w AND pp.house_key=r.house_key AND oo.operation_id=op AND oo.state=3 AND pp.content_fence_operation=op;
 UPDATE game_house_properties SET state='VACANT',owner_kind=NULL,owner_character_id=NULL,owner_account_id=NULL,owner_guild_id=NULL,
  paid_until=NULL,grace_until=NULL,move_out_at=NULL,content_fence_operation=NULL,revision=revision+1,acl_revision=acl_revision+1,ownership_revision=ownership_revision+1
  WHERE world_id=w AND house_key=r.house_key AND owner_guild_id=g;
 UPDATE game_house_ownership SET owner_guild_id=NULL,owner_character_id=NULL,ownership_revision=ownership_revision+1 WHERE world_id=w AND house_key=r.house_key AND owner_guild_id=g;
 UPDATE game_house_guild_recipient_reservations SET status=3 WHERE reservation_id=rid;
 UPDATE game_house_guild_dispositions SET state=5 WHERE operation_id=op AND state=3;
 IF NOT game_house_terminal_release_proven(op) THEN RAISE EXCEPTION 'actual terminal release proof absent' USING ERRCODE='OHG03';END IF;
 RETURN QUERY SELECT n,st+1,TRUE;
END $$;

DO $$ DECLARE f RECORD; t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_house_guild_dispositions','game_house_guild_disposition_items','game_house_guild_disposition_steps',
  'game_house_guild_recipient_reservations','game_house_terminal_releases','game_house_guild_authority_stamps'] LOOP
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',t);
 END LOOP;
 FOR f IN SELECT oid::regprocedure AS signature FROM pg_proc WHERE pronamespace=current_schema()::regnamespace
  AND (proname LIKE 'game_house_guild_%' OR proname LIKE 'game_house_terminal_%') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f.signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_guild_reserve(UUID,UUID,NUMERIC,UUID,UUID),
 game_house_guild_reservation_binding(UUID),game_house_guild_lock_spine(UUID,UUID,NUMERIC,UUID,UUID,BYTEA,TEXT,JSONB,UUID,NUMERIC,BYTEA),
 game_house_guild_advance(UUID,UUID,NUMERIC,UUID,UUID,BYTEA,TEXT),
 game_house_guild_assert_released(UUID,UUID,NUMERIC,UUID,UUID,BYTEA,TEXT),
 game_house_guild_disband_released_proven(UUID,UUID) TO oteryn_game_runtime;
"#;

#[cfg(test)]
mod tests {
    use super::ORDINARY_PROPERTY_SCHEMA_SQL;
    use sqlx::{Connection, Executor, PgConnection, Postgres, Row, Transaction};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
    const HOUSE: &str = "oteryn:content.house.thais_1";
    const OTHER_HOUSE: &str = "oteryn:content.house.thais_2";

    struct Database {
        admin_url: String,
        name: String,
        url: String,
    }

    impl Database {
        async fn create(admin_url: String) -> TestResult<Self> {
            if !admin_url.starts_with("postgresql://oteryn_test_admin:")
                || !admin_url.ends_with("@127.0.0.1:5432/postgres")
            {
                return Err("unsafe PostgreSQL test admin URL".into());
            }
            let mut admin = PgConnection::connect(&admin_url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut admin)
                .await?;
            if version != "170006" {
                return Err("House ownership qualification requires PostgreSQL17.6".into());
            }
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("house_ownership_{}_{suffix}", std::process::id());
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name}"
                ))))
                .await?;
            admin.close().await?;
            let prefix = admin_url.strip_suffix("/postgres").ok_or("admin URL")?;
            let url = format!("{prefix}/{name}");
            Ok(Self {
                admin_url,
                name,
                url,
            })
        }

        async fn cleanup(self) -> TestResult {
            let mut admin = PgConnection::connect(&self.admin_url).await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE {} WITH (FORCE)",
                    self.name
                ))))
                .await?;
            admin.close().await?;
            Ok(())
        }
    }

    fn run(
        install: bool,
        case: impl AsyncFnOnce(&mut PgConnection, &str) -> TestResult,
    ) -> TestResult {
        let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL").map_err(
            |_| "House ownership PostgreSQL case was not executed: approved admin URL is required",
        )?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async move {
                let database = Database::create(admin).await?;
                let outcome = async {
                    let mut connection = PgConnection::connect(&database.url).await?;
                    sqlx::migrate!("./migrations").run(&mut connection).await?;
                    if install {
                        let mut tx = connection.begin().await?;
                        tx.execute(ORDINARY_PROPERTY_SCHEMA_SQL).await?;
                        tx.commit().await?;
                    }
                    case(&mut connection, &database.url).await
                }
                .await;
                database.cleanup().await?;
                outcome
            })
    }

    struct Character {
        world: String,
        account: String,
        character: String,
    }

    /// Administrative source fixture only, never a session, Premium or acquisition proof.
    /// Native identity tables/migrations are unchanged; only root seeding bypasses triggers.
    async fn seed_character(connection: &mut PgConnection) -> TestResult<Character> {
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica")
            .await?;
        let world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut *tx)
            .await?;
        let account: String = sqlx::query_scalar(
            "INSERT INTO game_character_account_guards(account_id) VALUES(game_character_uuid_v7()) RETURNING account_id::text",
        ).fetch_one(&mut *tx).await?;
        let character: String = sqlx::query_scalar(
            "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name)
             VALUES(game_character_uuid_v7(),$1::uuid,$2::uuid,1,1,'p1','r1','c1','s1','House'||translate(substr(md5(random()::text),1,12),'0123456789','ghijklmnop')) RETURNING character_id::text",
        ).bind(&account).bind(&world).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(Character {
            world,
            account,
            character,
        })
    }

    async fn acquire(
        tx: &mut Transaction<'_, Postgres>,
        actor: &Character,
        house: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,state,revision,acl_revision,ownership_revision) VALUES($1::uuid,$2,'VACANT',1,0,0)")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_house_properties SET state='AUCTION',revision=2 WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_house_properties SET state='OWNED',owner_kind='CHARACTER',owner_character_id=$3::uuid,owner_account_id=$4::uuid,paid_until=2592000001,revision=3,ownership_revision=1,acquisition_receipt_ref=game_character_uuid_v7() WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).bind(&actor.character).bind(&actor.account).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_account_housing_slots(account_id,world_id,house_key,kind) VALUES($1::uuid,$2::uuid,$3,'PHYSICAL_HOUSE')")
            .bind(&actor.account).bind(&actor.world).bind(house).execute(&mut **tx).await?;
        Ok(())
    }

    fn sqlstate<T>(outcome: Result<T, sqlx::Error>) -> TestResult<String> {
        match outcome {
            Ok(_) => Err("expected database refusal".into()),
            Err(error) => error
                .as_database_error()
                .and_then(|database| database.code())
                .map(|code| code.into_owned())
                .ok_or_else(|| format!("not a database error: {error}").into()),
        }
    }

    #[test]
    fn house_ownership_release_retains_instance_and_receipt_after_reload() -> TestResult {
        run(true, async |connection, url| {
            let actor = seed_character(connection).await?;
            let mut tx = connection.begin().await?;
            acquire(&mut tx, &actor, HOUSE).await?;
            tx.commit().await?;
            sqlx::query("INSERT INTO game_world_house_instances(world_id,house_key,house_revision,instance_id,compatible_content_digest,qualified_placement_digest,binding_receipt_ref) VALUES($1::uuid,$2,'r1',game_character_uuid_v7(),decode(repeat('00',32),'hex'),decode(repeat('01',32),'hex'),game_character_uuid_v7())")
                .bind(&actor.world).bind(HOUSE).execute(&mut *connection).await?;
            sqlx::query("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(game_character_uuid_v7(),$1::uuid,$2,-1,$3::uuid,game_character_uuid_v7(),1,1,0,1,false,ARRAY[]::uuid[])")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).execute(&mut *connection).await?;
            let mut tx = connection.begin().await?;
            sqlx::query("UPDATE game_house_properties SET state='DISPOSITION',revision=4 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_properties SET state='VACANT',owner_kind=NULL,owner_character_id=NULL,owner_account_id=NULL,paid_until=NULL,revision=5,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_ownership SET owner_character_id=NULL,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query(
                "DELETE FROM game_account_housing_slots WHERE world_id=$1::uuid AND house_key=$2",
            )
            .bind(&actor.world)
            .bind(HOUSE)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            let mut reopened = PgConnection::connect(url).await?;
            let row=sqlx::query("SELECT p.state,p.owner_character_id IS NULL AND a.owner_character_id IS NULL AS cleared,p.ownership_revision=a.ownership_revision AND a.ownership_revision=2 AS generation,p.acquisition_receipt_ref=a.acquisition_receipt_ref AS receipt,(SELECT count(*) FROM game_world_house_instances) AS bindings,(SELECT count(*) FROM game_house_acl_receipts) AS receipts,(SELECT count(*) FROM game_account_housing_slots) AS slots,r.character_revision::text FROM game_house_properties p JOIN game_house_ownership a USING(world_id,house_key) CROSS JOIN game_character_roots r WHERE p.world_id=$1::uuid AND p.house_key=$2 AND r.character_id=$3::uuid")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).fetch_one(&mut reopened).await?;
            assert_eq!(row.try_get::<String, _>("state")?, "VACANT");
            for field in ["cleared", "generation", "receipt"] {
                assert!(row.try_get::<bool, _>(field)?);
            }
            assert_eq!(row.try_get::<i64, _>("bindings")?, 1);
            assert_eq!(row.try_get::<i64, _>("receipts")?, 1);
            assert_eq!(row.try_get::<i64, _>("slots")?, 0);
            assert_eq!(row.try_get::<String, _>("character_revision")?, "1");
            Ok(())
        })
    }

    #[test]
    fn house_ownership_unsynchronized_release_rolls_back() -> TestResult {
        run(true, async |connection, _| {
            let actor = seed_character(connection).await?;
            let mut tx = connection.begin().await?;
            acquire(&mut tx, &actor, HOUSE).await?;
            tx.commit().await?;
            let mut tx = connection.begin().await?;
            sqlx::query("UPDATE game_house_properties SET state='DISPOSITION',revision=4 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_properties SET state='VACANT',owner_kind=NULL,owner_character_id=NULL,owner_account_id=NULL,paid_until=NULL,revision=5,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            assert_eq!(sqlstate(tx.commit().await)?, "23514");
            let retained: bool=sqlx::query_scalar("SELECT p.state='OWNED' AND p.revision=3 AND p.ownership_revision=1 AND p.owner_character_id=a.owner_character_id AND EXISTS(SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=p.world_id AND s.house_key=p.house_key) FROM game_house_properties p JOIN game_house_ownership a USING(world_id,house_key) WHERE p.world_id=$1::uuid AND p.house_key=$2")
                .bind(&actor.world).bind(HOUSE).fetch_one(&mut *connection).await?;
            assert!(retained);
            let mut tx = connection.begin().await?;
            let change=sqlx::query("UPDATE game_house_ownership SET owner_character_id=NULL,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await;
            assert_eq!(sqlstate(change)?, "23514");
            tx.rollback().await?;
            let deletion = sqlx::query(
                "DELETE FROM game_house_ownership WHERE world_id=$1::uuid AND house_key=$2",
            )
            .bind(&actor.world)
            .bind(HOUSE)
            .execute(&mut *connection)
            .await;
            assert_eq!(sqlstate(deletion)?, "23514");
            Ok(())
        })
    }

    #[test]
    fn house_ownership_legacy_acl_revision_is_atomic_and_has_no_runtime_property_grant()
    -> TestResult {
        run(true, async |connection, _| {
            let actor = seed_character(connection).await?;
            let mut tx = connection.begin().await?;
            acquire(&mut tx, &actor, HOUSE).await?;
            tx.commit().await?;

            for (before, allowed) in [(0_u64, true), (1, false)] {
                let mut tx = connection.begin().await?;
                tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
                let receipt: String = sqlx::query_scalar(
                    "INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(game_character_uuid_v7(),$1::uuid,$2,-1,$3::uuid,game_character_uuid_v7(),1,1,$4::text::numeric,$5::text::numeric,$6,ARRAY[]::uuid[]) RETURNING receipt_id::text",
                )
                .bind(&actor.world)
                .bind(HOUSE)
                .bind(&actor.character)
                .bind(before.to_string())
                .bind((before + 1).to_string())
                .bind(allowed)
                .fetch_one(&mut *tx)
                .await?;
                if before == 0 {
                    sqlx::query("INSERT INTO game_house_acl(world_id,house_key,list_id,revision,allow_everyone,members,last_receipt) VALUES($1::uuid,$2,-1,1,$3,ARRAY[]::uuid[],$4::uuid)")
                        .bind(&actor.world).bind(HOUSE).bind(allowed).bind(&receipt).execute(&mut *tx).await?;
                } else {
                    sqlx::query("UPDATE game_house_acl SET revision=2,allow_everyone=$3,last_receipt=$4::uuid WHERE world_id=$1::uuid AND house_key=$2 AND list_id=-1")
                        .bind(&actor.world).bind(HOUSE).bind(allowed).bind(&receipt).execute(&mut *tx).await?;
                }
                tx.execute("RESET ROLE").await?;
                let observed: String = sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
                    .bind(&actor.world).bind(HOUSE).fetch_one(&mut *tx).await?;
                assert_eq!(observed, (before + 1).to_string());
                tx.commit().await?;

                // The real 0038 deferred receipt guard rejects a changed list without a
                // new same-transaction receipt. Its earlier property revision update must
                // roll back with that rejected ACL write.
                let mut refused = connection.begin().await?;
                refused
                    .execute("SET LOCAL ROLE oteryn_game_runtime")
                    .await?;
                sqlx::query("UPDATE game_house_acl SET revision=revision+1,allow_everyone=NOT allow_everyone WHERE world_id=$1::uuid AND house_key=$2 AND list_id=-1")
                    .bind(&actor.world).bind(HOUSE).execute(&mut *refused).await?;
                assert_eq!(sqlstate(refused.commit().await)?, "23514");
                let retained: (String, String, bool, i64) = sqlx::query_as("SELECT p.acl_revision::text,a.revision::text,a.allow_everyone,(SELECT count(*) FROM game_house_acl_receipts) FROM game_house_properties p JOIN game_house_acl a USING(world_id,house_key) WHERE p.world_id=$1::uuid AND p.house_key=$2")
                    .bind(&actor.world).bind(HOUSE).fetch_one(&mut *connection).await?;
                assert_eq!(
                    retained,
                    (
                        (before + 1).to_string(),
                        (before + 1).to_string(),
                        allowed,
                        (before + 1) as i64
                    )
                );
            }

            sqlx::query("DELETE FROM game_house_acl WHERE world_id=$1::uuid AND house_key=$2 AND list_id=-1")
                .bind(&actor.world).bind(HOUSE).execute(&mut *connection).await?;
            let retained: (String, String, String, i64, i64) = sqlx::query_as("SELECT acl_revision::text,revision::text,ownership_revision::text,(SELECT count(*) FROM game_house_acl),(SELECT count(*) FROM game_house_acl_receipts) FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).fetch_one(&mut *connection).await?;
            assert_eq!(retained, ("3".into(), "3".into(), "1".into(), 0, 2));

            let mut refused = connection.begin().await?;
            refused
                .execute("SET LOCAL ROLE oteryn_game_runtime")
                .await?;
            let direct = sqlx::query("UPDATE game_house_properties SET acl_revision=acl_revision+1 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *refused).await;
            assert_eq!(sqlstate(direct)?, "42501");
            refused.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn house_ownership_concurrent_properties_compete_for_one_account_world_slot() -> TestResult {
        run(true, async |connection, url| {
            let actor = seed_character(connection).await?;
            let attempt = async |house: &str| -> Result<(), sqlx::Error> {
                let mut writer = PgConnection::connect(url).await?;
                let mut tx = writer.begin().await?;
                acquire(&mut tx, &actor, house).await?;
                tx.commit().await
            };
            let (left, right) = tokio::join!(attempt(HOUSE), attempt(OTHER_HOUSE));
            match (left, right) {
                (Ok(()), loser) | (loser, Ok(())) => assert_eq!(sqlstate(loser)?, "23505"),
                _ => return Err("expected exactly one committed property acquisition".into()),
            }
            let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_house_properties),(SELECT count(*) FROM game_house_ownership),(SELECT count(*) FROM game_account_housing_slots)")
                .fetch_one(&mut *connection).await?;
            assert_eq!(counts, (1, 1, 1));
            Ok(())
        })
    }

    #[test]
    fn house_ownership_retired_property_cannot_be_reopened() -> TestResult {
        run(true, async |connection, _| {
            let world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut *connection)
                .await?;
            sqlx::query("INSERT INTO game_house_properties(world_id,house_key,state,revision,acl_revision,ownership_revision) VALUES($1::uuid,$2,'VACANT',1,0,0)")
                .bind(&world).bind(HOUSE).execute(&mut *connection).await?;
            sqlx::query("UPDATE game_house_properties SET state='RETIRED',revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&world).bind(HOUSE).execute(&mut *connection).await?;
            for state in ["VACANT", "AUCTION"] {
                let changed=sqlx::query("UPDATE game_house_properties SET state=$3,revision=3 WHERE world_id=$1::uuid AND house_key=$2")
                    .bind(&world).bind(HOUSE).bind(state).execute(&mut *connection).await;
                assert_eq!(sqlstate(changed)?, "23514");
            }
            let retained:bool=sqlx::query_scalar("SELECT state='RETIRED' AND revision=2 AND ownership_revision=0 FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&world).bind(HOUSE).fetch_one(&mut *connection).await?;
            assert!(retained);
            Ok(())
        })
    }

    #[test]
    fn house_ownership_refuses_unreconciled_legacy_rows_without_guessing_rent() -> TestResult {
        run(false, async |connection, _| {
            let actor = seed_character(connection).await?;
            sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES($1::uuid,$2,$3::uuid,1,game_character_uuid_v7())")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).execute(&mut *connection).await?;
            let mut tx = connection.begin().await?;
            assert_eq!(
                sqlstate(tx.execute(ORDINARY_PROPERTY_SCHEMA_SQL).await)?,
                "23514"
            );
            tx.rollback().await?;
            let retained:bool=sqlx::query_scalar("SELECT to_regclass('game_house_properties') IS NULL AND EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=$1::uuid AND house_key=$2 AND owner_character_id=$3::uuid AND ownership_revision=1)")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).fetch_one(&mut *connection).await?;
            assert!(retained);
            Ok(())
        })
    }
}
