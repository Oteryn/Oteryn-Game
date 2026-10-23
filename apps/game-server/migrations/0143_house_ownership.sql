-- HOUSE-1a: ordinary property spine. Disposition, rent collection and Guildhall
-- release are deliberately absent (Integrator decision 2026-10-22).
-- No catalogue or property is seeded. A VACANT anchor is not acquisition authority.
DO $$ BEGIN
 IF EXISTS (SELECT 1 FROM game_house_ownership) THEN
  RAISE EXCEPTION 'HOUSE_LEGACY_RECONCILIATION_REQUIRED: first rent and paid_until are unknown';
 END IF;
END $$;

CREATE TABLE game_house_properties (
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 kind TEXT NOT NULL CHECK(kind IN ('private_house','shop')),
 state SMALLINT NOT NULL DEFAULT 1 CHECK(state BETWEEN 1 AND 6),
 rent_gold BIGINT NOT NULL CHECK(rent_gold BETWEEN 0 AND 999999999999),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 owner_character_id UUID REFERENCES game_character_roots(character_id),
 owner_account_id UUID REFERENCES game_character_account_guards(account_id),
 ownership_generation NUMERIC(20,0) NOT NULL DEFAULT 1
  CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 acl_revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(acl_revision BETWEEN 1 AND 18446744073709551615),
 acquisition_receipt_ref UUID CHECK(game_character_is_uuid_v7(acquisition_receipt_ref)),
 paid_until TIMESTAMPTZ,
 content_fence_operation UUID,
 PRIMARY KEY(world_id,house_key),
 CHECK((owner_character_id IS NOT NULL)=(owner_account_id IS NOT NULL)),
 CHECK((state IN (3,4,5))=(owner_character_id IS NOT NULL)),
 CHECK(owner_character_id IS NULL OR (paid_until IS NOT NULL AND acquisition_receipt_ref IS NOT NULL))
);
CREATE TABLE game_house_tiles (
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position) BETWEEN 1 AND 128),
 PRIMARY KEY(world_id,spatial_position),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE TABLE game_account_housing_slots (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 PRIMARY KEY(account_id,world_id), UNIQUE(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
-- Account/World bans are consumed here; their producer belongs to HOUSE-1b.
CREATE TABLE game_account_house_bans (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 banned_until TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(account_id,world_id)
);
CREATE TABLE game_house_auctions (
 auction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(auction_id)),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 first_rent BIGINT NOT NULL CHECK(first_rent BETWEEN 0 AND 999999999999),
 ends_at TIMESTAMPTZ NOT NULL,
 next_order BIGINT NOT NULL DEFAULT 1 CHECK(next_order>0),
 settled BOOLEAN NOT NULL DEFAULT false,
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE UNIQUE INDEX game_house_one_open_auction ON game_house_auctions(world_id,house_key) WHERE NOT settled;
CREATE TABLE game_house_bids (
 auction_id UUID NOT NULL REFERENCES game_house_auctions(auction_id),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 world_id UUID NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 maximum BIGINT NOT NULL CHECK(maximum BETWEEN 0 AND 999999999999),
 first_rent BIGINT NOT NULL CHECK(first_rent BETWEEN 0 AND 999999999999),
 reached_order BIGINT NOT NULL CHECK(reached_order>0),
 state SMALLINT NOT NULL DEFAULT 1 CHECK(state IN (1,2,3)),
 escrow_gold BIGINT NOT NULL CHECK(escrow_gold BETWEEN 0 AND 999999999999),
 PRIMARY KEY(auction_id,account_id), UNIQUE(auction_id,reached_order),
 CHECK(escrow_gold=CASE state WHEN 1 THEN maximum+first_rent ELSE 0 END)
);
CREATE UNIQUE INDEX game_house_one_held_bid ON game_house_bids(account_id,world_id) WHERE state=1;
CREATE UNIQUE INDEX game_house_one_winner ON game_house_bids(auction_id) WHERE state=2;
CREATE TABLE game_house_operations (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 auction_id UUID NOT NULL REFERENCES game_house_auctions(auction_id),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 kind SMALLINT NOT NULL CHECK(kind IN (1,2,3)),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 game_session_id UUID, command_id NUMERIC(20,0),
 balance_before BIGINT NOT NULL CHECK(balance_before BETWEEN 0 AND 999999999999),
 balance_after BIGINT NOT NULL CHECK(balance_after BETWEEN 0 AND 999999999999),
 escrow_before BIGINT NOT NULL CHECK(escrow_before BETWEEN 0 AND 999999999999),
 escrow_after BIGINT NOT NULL CHECK(escrow_after BETWEEN 0 AND 999999999999),
 price BIGINT NOT NULL CHECK(price BETWEEN 0 AND 999999999999),
 first_rent BIGINT NOT NULL CHECK(first_rent BETWEEN 0 AND 999999999999),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key),
 CHECK((kind=1)=(game_session_id IS NOT NULL AND command_id IS NOT NULL)),
 CHECK((game_session_id IS NULL)=(command_id IS NULL)),
 CHECK(command_id IS NULL OR command_id BETWEEN 1 AND 18446744073709551615),
 CHECK(kind=2 OR price=0),
 CHECK(kind<>3 OR escrow_after=0),
 CHECK(balance_before+escrow_before=balance_after+escrow_after+CASE kind WHEN 2 THEN price+first_rent ELSE 0 END)
);

CREATE TABLE game_house_settlement_steps (
 auction_id UUID NOT NULL REFERENCES game_house_auctions,
 step BIGINT NOT NULL CHECK(step BETWEEN 0 AND 256),
 world_id UUID NOT NULL,house_key TEXT NOT NULL,
 PRIMARY KEY(auction_id,step),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties
);
CREATE TABLE game_house_audit_outbox (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 operation_id UUID NOT NULL UNIQUE REFERENCES game_house_operations,
 event_type_id BIGINT NOT NULL DEFAULT 8 CHECK(event_type_id=8),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 131072),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+2592000000),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 publication_state SMALLINT NOT NULL DEFAULT 1 CHECK(publication_state IN(1,2)),
 published_at BIGINT CHECK(published_at IS NULL OR published_at>=occurred_at),
 CHECK((publication_state=2)=(published_at IS NOT NULL))
);

ALTER TABLE game_account_bank_entries
 ADD COLUMN house_operation_id UUID REFERENCES game_house_operations DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN escrow_before BIGINT,
 ADD COLUMN escrow_after BIGINT,
 DROP CONSTRAINT game_account_bank_entries_kind_check,
 DROP CONSTRAINT game_account_bank_entries_check,
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 ADD CONSTRAINT game_account_bank_entries_kind_check CHECK(kind IN(1,2,3,4,5,7,8,9,10)),
 ADD CONSTRAINT game_account_bank_entries_one_reference
  CHECK(num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id)=1),
 ADD CONSTRAINT game_account_bank_entries_house_reference CHECK((kind>=7)=(house_operation_id IS NOT NULL)),
 ADD CONSTRAINT game_account_bank_entries_house_value CHECK(
  CASE WHEN kind>=7 THEN escrow_before BETWEEN 0 AND 999999999999 AND escrow_after BETWEEN 0 AND 999999999999
   AND escrow_before IS NOT NULL AND escrow_after IS NOT NULL
   AND balance_after=balance_before+CASE kind WHEN 7 THEN -amount WHEN 8 THEN amount ELSE 0 END
   AND escrow_after=escrow_before+CASE kind WHEN 7 THEN amount ELSE -amount END
  ELSE escrow_before IS NULL AND escrow_after IS NULL
   AND balance_after=balance_before+CASE WHEN kind IN(1,4) THEN amount ELSE -amount END END),
 ADD CONSTRAINT game_account_bank_entries_house_kind UNIQUE(house_operation_id,kind);
-- Preserve the ordinary Bank/Fee/NPC guard body, without copying or weakening it.
DROP TRIGGER game_account_bank_entry_consistent ON game_account_bank_entries;
CREATE CONSTRAINT TRIGGER game_account_bank_entry_consistent AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.kind<7)
 EXECUTE FUNCTION game_account_bank_entry_consistency_guard();

CREATE FUNCTION game_house_bank_entry_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_operations%ROWTYPE; p game_account_bank_entries%ROWTYPE;
BEGIN
 SELECT * INTO o FROM game_house_operations WHERE operation_id=NEW.house_operation_id;
 SELECT * INTO p FROM game_account_bank_entries WHERE entry_id=NEW.previous_entry_id;
 IF o.operation_id IS NULL OR o.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
  OR (NEW.account_id,NEW.world_id,NEW.acting_character_id)<>(o.account_id,o.world_id,o.character_id)
  OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
  OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e
   WHERE e.entry_id=NEW.previous_entry_id AND (e.account_id,e.world_id,e.balance_after)=(NEW.account_id,NEW.world_id,NEW.balance_before)))
  OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.last_entry_id=NEW.entry_id)
   OR EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.previous_entry_id=NEW.entry_id))
  OR (p.house_operation_id IS NOT DISTINCT FROM o.operation_id AND (p.escrow_after<>NEW.escrow_before OR p.kind>=NEW.kind))
  OR (p.house_operation_id IS DISTINCT FROM o.operation_id AND
   (NEW.balance_before,NEW.escrow_before)<>(o.balance_before,o.escrow_before))
  OR (NOT EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.previous_entry_id=NEW.entry_id AND e.house_operation_id=o.operation_id)
   AND (NEW.balance_after,NEW.escrow_after)<>(o.balance_after,o.escrow_after)) THEN
  RAISE EXCEPTION 'House ledger entry lacks same-transaction custody proof' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_bank_entry_proven AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.kind>=7) EXECUTE FUNCTION game_house_bank_entry_proven();

CREATE FUNCTION game_house_operation_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_operations%ROWTYPE; oid UUID; reserved BIGINT; released BIGINT; price BIGINT; rent BIGINT; n BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_house_operations' THEN oid:=NEW.operation_id;
 ELSE oid:=NEW.house_operation_id;END IF;
 SELECT * INTO o FROM game_house_operations WHERE operation_id=oid;
 -- UNIQUE(house_operation_id,kind) bounds this aggregate to four entries.
 SELECT count(*),coalesce(sum(amount) FILTER(WHERE kind=7),0),coalesce(sum(amount) FILTER(WHERE kind=8),0),
  coalesce(sum(amount) FILTER(WHERE kind=9),0),coalesce(sum(amount) FILTER(WHERE kind=10),0)
 INTO n,reserved,released,price,rent FROM game_account_bank_entries WHERE house_operation_id=oid;
 IF o.operation_id IS NULL OR n>3 OR NOT EXISTS(SELECT 1 FROM game_house_audit_outbox WHERE operation_id=oid)
  OR (o.kind=1 AND (n>1 OR price<>0 OR rent<>0 OR reserved-released<>o.escrow_after-o.escrow_before))
  OR (o.kind=2 AND (reserved<>0 OR price<>o.price OR rent<>o.first_rent
   OR released<>o.escrow_before-o.price-o.first_rent OR o.escrow_after<>0))
  OR (o.kind=3 AND (n>1 OR reserved<>0 OR price<>0 OR rent<>0 OR released<>o.escrow_before OR o.escrow_after<>0))
  OR o.balance_after<>o.balance_before+released-reserved THEN
  RAISE EXCEPTION 'House operation ledger and custody do not conserve value' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_operation_proven AFTER INSERT ON game_house_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_operation_proven();
CREATE CONSTRAINT TRIGGER game_house_operation_entry_proven AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.kind>=7) EXECUTE FUNCTION game_house_operation_proven();

ALTER TABLE game_house_ownership ALTER COLUMN owner_character_id DROP NOT NULL;
-- The immutable 0038 anchor is retained across release. New acquisition receipts
-- differ; releasing retains the last receipt and advances ownership only.
CREATE OR REPLACE FUNCTION game_house_owner_in_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.owner_character_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_character_roots WHERE character_id=NEW.owner_character_id
   AND world_id=NEW.world_id AND lifecycle=1) THEN
  RAISE EXCEPTION 'House owner must be an active Character of this World' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key)<>(OLD.world_id,OLD.house_key)
  OR NEW.ownership_revision<>OLD.ownership_revision+1
  OR (NEW.owner_character_id IS NULL AND NEW.acquisition_receipt_ref<>OLD.acquisition_receipt_ref)
  OR (NEW.owner_character_id IS NOT NULL AND NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref)) THEN
  RAISE EXCEPTION 'House ownership requires an exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_house_property_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'House properties are never deleted' USING ERRCODE='23514';END IF;
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key,NEW.kind)<>(OLD.world_id,OLD.house_key,OLD.kind)
  OR NEW.revision<>OLD.revision+1 OR NEW.acl_revision<OLD.acl_revision
  OR (OLD.state=6 AND NEW.state<>6)
  OR NEW.ownership_generation<>OLD.ownership_generation+
   ((NEW.owner_character_id,NEW.owner_account_id) IS DISTINCT FROM (OLD.owner_character_id,OLD.owner_account_id))::integer
  OR (OLD.owner_character_id IS NOT NULL AND NEW.owner_character_id IS NULL
   AND NEW.acquisition_receipt_ref IS DISTINCT FROM OLD.acquisition_receipt_ref)) THEN
  RAISE EXCEPTION 'House property requires an exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_property_transition BEFORE UPDATE OR DELETE ON game_house_properties
 FOR EACH ROW EXECUTE FUNCTION game_house_property_transition();

CREATE FUNCTION game_house_property_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_house_properties%ROWTYPE; a game_house_ownership%ROWTYPE; w UUID; h TEXT;
BEGIN
 -- Slot re-keying must check both the old and the new house.
 FOR w,h IN SELECT DISTINCT x.world_id,x.house_key FROM (
  SELECT OLD.world_id,OLD.house_key WHERE TG_OP<>'INSERT'
  UNION ALL SELECT NEW.world_id,NEW.house_key WHERE TG_OP<>'DELETE') x LOOP
  SELECT * INTO p FROM game_house_properties WHERE world_id=w AND house_key=h;
  SELECT * INTO a FROM game_house_ownership WHERE world_id=w AND house_key=h;
 IF p.world_id IS NULL OR a.world_id IS NULL OR a.owner_character_id IS DISTINCT FROM p.owner_character_id
   OR a.ownership_revision<>p.ownership_generation
   OR (p.acquisition_receipt_ref IS NOT NULL AND a.acquisition_receipt_ref<>p.acquisition_receipt_ref)
   OR (p.owner_character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_roots r
    WHERE r.character_id=p.owner_character_id AND r.account_id=p.owner_account_id AND r.world_id=w AND r.lifecycle=1))
   OR (SELECT count(*) FROM game_account_housing_slots s WHERE s.world_id=w AND s.house_key=h)<>
    (p.owner_account_id IS NOT NULL)::integer
   OR EXISTS(SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=w AND s.house_key=h
    AND s.account_id IS DISTINCT FROM p.owner_account_id) THEN
   RAISE EXCEPTION 'House canonical owner, legacy anchor and slot disagree' USING ERRCODE='23514';
  END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_property_proven AFTER INSERT OR UPDATE ON game_house_properties
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_proven();
CREATE CONSTRAINT TRIGGER game_house_anchor_proven AFTER INSERT OR UPDATE OR DELETE ON game_house_ownership
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_proven();
CREATE CONSTRAINT TRIGGER game_house_slot_proven AFTER INSERT OR UPDATE OR DELETE ON game_account_housing_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_proven();
CREATE TRIGGER game_house_anchor_undeletable BEFORE DELETE ON game_house_ownership
 FOR EACH ROW EXECUTE FUNCTION game_house_property_transition();

-- Shared credit seam. Market and Guildhall have no registered value writer on
-- this baseline; their owners must extend this same sum when their storage lands.
CREATE FUNCTION game_account_bank_credit_headroom(p_account UUID,p_world UUID)
RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,999999999999-coalesce((SELECT balance FROM game_account_bank_balances
  WHERE account_id=p_account AND world_id=p_world),0)-coalesce((SELECT sum(escrow_gold)
  FROM game_house_bids WHERE account_id=p_account AND world_id=p_world AND state=1),0))::bigint
$$;
CREATE FUNCTION game_house_headroom_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1)
  AND EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=NEW.account_id AND world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'House bid and personal slot are mutually exclusive' USING ERRCODE='23514';
 END IF;
 IF coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=NEW.account_id AND world_id=NEW.world_id),0)
  +coalesce((SELECT sum(escrow_gold) FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1),0)>999999999999 THEN
  RAISE EXCEPTION 'HOUSE_BANK_HEADROOM' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_headroom_proven AFTER INSERT OR UPDATE ON game_house_bids
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_headroom_proven();
CREATE CONSTRAINT TRIGGER game_bank_house_headroom_proven AFTER INSERT OR UPDATE ON game_account_bank_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_headroom_proven();
CREATE CONSTRAINT TRIGGER game_slot_house_headroom_proven AFTER INSERT OR UPDATE ON game_account_housing_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_headroom_proven();

CREATE FUNCTION game_house_ground_tile_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_tiles WHERE world_id=NEW.world_id AND spatial_position=NEW.spatial_position) THEN
  RAISE EXCEPTION 'Ground location is a World-global House tile' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_ground_tile_guard BEFORE INSERT OR UPDATE ON game_item_ground_locations
 FOR EACH ROW EXECUTE FUNCTION game_house_ground_tile_guard();

CREATE FUNCTION game_house_acl_revision() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 UPDATE game_house_properties SET acl_revision=acl_revision+1,revision=revision+1
  WHERE world_id=OLD.world_id AND house_key=OLD.house_key;
 IF TG_OP='INSERT' THEN
  UPDATE game_house_properties SET acl_revision=acl_revision+1,revision=revision+1
   WHERE world_id=NEW.world_id AND house_key=NEW.house_key;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_house_acl_revision AFTER INSERT OR UPDATE OR DELETE ON game_house_acl
 FOR EACH ROW EXECUTE FUNCTION game_house_acl_revision();

-- The runtime calls these through the existing House access owner. ACL-1 and
-- HOUSE-1b replace only their own grant and fence functions, respectively.
CREATE FUNCTION game_house_access_grant(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$ SELECT NULL::text $$;
CREATE FUNCTION game_house_access_fence(p_world UUID,p_house TEXT)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT content_fence_operation IS NOT NULL OR state IN (5,6)
  FROM game_house_properties WHERE world_id=p_world AND house_key=p_house
$$;
CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID)
RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_properties%ROWTYPE; grant_role TEXT;
BEGIN
 SELECT * INTO p FROM game_house_properties WHERE world_id=p_world_id AND house_key=p_house_key FOR SHARE;
 IF NOT FOUND OR p.owner_character_id IS NULL THEN RETURN;END IF;
 grant_role:=CASE WHEN p.owner_character_id=p_character_id THEN 'OWNER'
  ELSE game_house_access_grant(p_world_id,p_house_key,p_character_id) END;
 IF grant_role IS NULL THEN RETURN;END IF;
 RETURN QUERY SELECT grant_role,game_house_access_fence(p_world_id,p_house_key),p.acl_revision,NULL::jsonb;
END $$;
CREATE OR REPLACE FUNCTION game_house_access_tile(p_world UUID,p_house TEXT,p_position BYTEA)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_tiles WHERE world_id=p_world AND house_key=p_house AND spatial_position=p_position)
$$;

REVOKE ALL ON game_house_properties,game_house_tiles,game_account_housing_slots,
 game_house_auctions,game_house_bids,game_house_operations,game_account_house_bans FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT ON game_house_properties,game_house_tiles,game_account_housing_slots,
 game_house_auctions,game_house_bids,game_house_operations,game_account_house_bans TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_house_properties,game_house_tiles,game_account_housing_slots,
 game_house_auctions,game_house_bids,game_account_house_bans TO oteryn_game_control;
GRANT DELETE ON game_account_housing_slots,game_house_tiles TO oteryn_game_control;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_owner_in_world','game_house_property_transition',
  'game_house_property_proven','game_house_headroom_proven','game_house_ground_tile_guard',
  'game_house_acl_revision'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_account_bank_credit_headroom(UUID,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_tile(UUID,TEXT,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_grant(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_fence(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_account_bank_credit_headroom(UUID,UUID),game_house_access_grant(UUID,TEXT,UUID),
 game_house_access_fence(UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_bank_credit_headroom(UUID,UUID) TO oteryn_game_runtime;

-- Caller-transaction native writer: only EXECUTE is granted, never protected DML.
-- The caller has resolved Premium and pinned recovery/admission and House rows.
-- Possession remains a separate SQL parameter, absent from JSON and audit bytes.
CREATE FUNCTION game_house_value_apply(p JSONB,p_node UUID,p_node_revision NUMERIC,p_secret BYTEA,p_envelope BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a game_house_auctions%ROWTYPE; b game_house_bids%ROWTYPE; v game_account_bank_balances%ROWTYPE;
 o UUID:=(p->>'operation')::uuid; w UUID:=(p->>'world')::uuid; acct UUID:=(p->>'account')::uuid;
 c UUID:=(p->>'character')::uuid; k SMALLINT:=(p->>'kind')::smallint; l JSONB;
 bal BIGINT; esc BIGINT; prev UUID; last_id UUID; binding BYTEA; n INTEGER:=0; burned BIGINT:=0;
BEGIN
 IF p IS NULL OR k IS NULL OR k NOT IN(1,2,3) OR NOT game_character_is_uuid_v7(o)
  OR game_node_prove_current_incarnation(p_node,p_node_revision,p_secret) IS DISTINCT FROM true
  OR jsonb_typeof(p->'lines') IS DISTINCT FROM 'array' OR jsonb_array_length(p->'lines')>3
  OR jsonb_typeof(p->'binding') IS DISTINCT FROM 'array' OR jsonb_array_length(p->'binding')<>32
  OR p_envelope IS NULL OR octet_length(p_envelope) NOT BETWEEN 1 AND 131072 THEN
  RAISE EXCEPTION 'HOUSE_INVALID_INPUT' USING ERRCODE='23514';
 END IF;
 IF k=1 AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
  JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id
  JOIN game_durability_admission_account_guards ag ON ag.account_id=s.account_id
  JOIN game_runtime_scope_assignments r ON r.world_id=s.world_id AND r.channel_id=s.runtime_scope_channel_id
  WHERE s.game_session_id=(p->>'session')::uuid AND s.character_id=c AND s.account_id=acct AND s.world_id=w
   AND s.runtime_scope_kind=1 AND s.session_state IN(1,2)
   AND s.current_generation=(p->>'connection_generation')::numeric
   AND s.character_lease_generation=(p->>'lease_generation')::numeric
   AND s.scope_ownership_generation=(p->>'scope_generation')::numeric
   AND cg.eligible AND cg.lease_generation=s.character_lease_generation
   AND cg.holder_game_session_id=s.game_session_id AND ag.holder_game_session_id=s.game_session_id
   AND ag.presence_character_id=c AND r.scope_kind=1 AND r.state=1
   AND r.holder_node_id=p_node AND r.holder_registration_revision=p_node_revision
   AND r.ownership_generation=s.scope_ownership_generation) THEN
  RAISE EXCEPTION 'HOUSE_NOT_CURRENT' USING ERRCODE='23514';
 END IF;
 IF k<>1 AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=w
  AND scope_kind=1 AND state=1 AND holder_node_id=p_node AND holder_registration_revision=p_node_revision) THEN
  RAISE EXCEPTION 'HOUSE_NOT_CURRENT' USING ERRCODE='23514';
 END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=w AND house_key=p->>'house' FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'HOUSE_NOT_CURRENT' USING ERRCODE='23514';END IF;
 SELECT * INTO a FROM game_house_auctions WHERE auction_id=(p->>'auction')::uuid FOR UPDATE;
 IF a.auction_id IS NULL OR (a.world_id,a.house_key)<>(w,p->>'house') OR a.first_rent<>(p->>'first_rent')::bigint
  OR (k=1 AND (a.settled OR clock_timestamp()>=a.ends_at))
  OR (k<>1 AND clock_timestamp()<a.ends_at) THEN
  RAISE EXCEPTION 'HOUSE_AUCTION_CLOSED' USING ERRCODE='23514';
 END IF;
 SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND account_id=acct FOR UPDATE;
 IF (b.auction_id IS NULL AND (k<>1 OR (p->>'escrow_before')::bigint<>0))
  OR (b.auction_id IS NOT NULL AND (b.state<>1 OR b.character_id<>c OR b.world_id<>w
   OR b.escrow_gold<>(p->>'escrow_before')::bigint OR b.first_rent<>a.first_rent))
  OR (k<>1 AND b.maximum<>(p->>'maximum')::bigint) THEN
  RAISE EXCEPTION 'HOUSE_BID_CONFLICT' USING ERRCODE='23514';
 END IF;
 SELECT * INTO v FROM game_account_bank_balances WHERE account_id=acct AND world_id=w FOR UPDATE;
 IF v.account_id IS NULL OR v.balance<>(p->>'balance_before')::bigint THEN
  RAISE EXCEPTION 'HOUSE_BANK_CONFLICT' USING ERRCODE='23514';
 END IF;
 SELECT decode(string_agg(lpad(to_hex(value::integer),2,'0'),'' ORDER BY ord),'hex') INTO binding
  FROM jsonb_array_elements_text(p->'binding') WITH ORDINALITY AS x(value,ord);
 INSERT INTO game_house_operations(operation_id,world_id,house_key,auction_id,account_id,character_id,
  kind,request_binding,game_session_id,command_id,balance_before,balance_after,escrow_before,escrow_after,price,first_rent)
 VALUES(o,w,p->>'house',a.auction_id,acct,c,k,binding,(p->>'session')::uuid,(p->>'command')::numeric,
  v.balance,(p->>'balance_after')::bigint,(p->>'escrow_before')::bigint,(p->>'escrow_after')::bigint,
  (p->>'price')::bigint,a.first_rent);
 bal:=v.balance;esc:=(p->>'escrow_before')::bigint;prev:=v.last_entry_id;
 FOR l IN SELECT value FROM jsonb_array_elements(p->'lines') LOOP
  n:=n+1;last_id:=(l->>'entry')::uuid;
  IF (l->>'balance_before')::bigint<>bal OR (l->>'escrow_before')::bigint<>esc
   OR (l->>'kind')::integer NOT IN(7,8,9,10)
   OR (k<>2 AND (l->>'kind')::integer IN(9,10))
   OR (k=2 AND (l->>'kind')::integer=7)
   OR (k=3 AND (l->>'kind')::integer<>8) THEN
   RAISE EXCEPTION 'HOUSE_VALUE_CHAIN' USING ERRCODE='23514';
  END IF;
  INSERT INTO game_account_bank_entries(entry_id,house_operation_id,account_id,world_id,kind,amount,
   balance_before,balance_after,previous_entry_id,acting_character_id,escrow_before,escrow_after)
  VALUES(last_id,o,acct,w,(l->>'kind')::smallint,(l->>'amount')::bigint,bal,(l->>'balance_after')::bigint,
   prev,c,esc,(l->>'escrow_after')::bigint);
  IF (l->>'kind')::integer IN(9,10) THEN
   IF (l->>'amount')::bigint<>(CASE (l->>'kind')::integer WHEN 9 THEN (p->>'price')::bigint ELSE a.first_rent END) THEN
    RAISE EXCEPTION 'HOUSE_BURN_MISMATCH' USING ERRCODE='23514';END IF;
   burned:=burned+(l->>'amount')::bigint;
  END IF;
  UPDATE game_account_bank_balances SET balance=(l->>'balance_after')::bigint,last_entry_id=last_id
   WHERE account_id=acct AND world_id=w;
  bal:=(l->>'balance_after')::bigint;esc:=(l->>'escrow_after')::bigint;prev:=last_id;
 END LOOP;
 IF bal<>(p->>'balance_after')::bigint OR esc<>(p->>'escrow_after')::bigint
  OR (k=2 AND burned<>(p->>'price')::bigint+a.first_rent) THEN
  RAISE EXCEPTION 'HOUSE_VALUE_TOTAL' USING ERRCODE='23514';
 END IF;
 IF k=1 THEN
  IF b.auction_id IS NULL AND (SELECT count(*) FROM game_house_bids WHERE auction_id=a.auction_id)>=256 THEN
   RAISE EXCEPTION 'HOUSE_AUCTION_FULL' USING ERRCODE='23514';END IF;
  IF (b.auction_id IS NULL OR b.maximum<>(p->>'maximum')::bigint) AND (p->>'reached')::bigint<>a.next_order THEN
   RAISE EXCEPTION 'HOUSE_BID_ORDER' USING ERRCODE='23514';END IF;
  IF b.auction_id IS NOT NULL AND b.maximum=(p->>'maximum')::bigint AND (p->>'reached')::bigint<>b.reached_order THEN
   RAISE EXCEPTION 'HOUSE_BID_ORDER' USING ERRCODE='23514';END IF;
  INSERT INTO game_house_bids(auction_id,account_id,world_id,character_id,maximum,first_rent,reached_order,escrow_gold)
  VALUES(a.auction_id,acct,w,c,(p->>'maximum')::bigint,a.first_rent,(p->>'reached')::bigint,esc)
  ON CONFLICT(auction_id,account_id) DO UPDATE SET maximum=EXCLUDED.maximum,reached_order=EXCLUDED.reached_order,escrow_gold=EXCLUDED.escrow_gold;
  UPDATE game_house_auctions SET next_order=greatest(next_order,(p->>'reached')::bigint+1),
   ends_at=to_timestamp((p->>'ends_at_us')::numeric/1000000) WHERE auction_id=a.auction_id;
 ELSE
  UPDATE game_house_bids SET state=CASE k WHEN 2 THEN 2 ELSE 3 END,escrow_gold=0
   WHERE auction_id=a.auction_id AND account_id=acct;
  IF k=2 THEN
   IF a.settled OR EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=acct AND world_id=w)
    OR NOT EXISTS(SELECT 1 FROM game_house_properties WHERE world_id=w AND house_key=a.house_key AND state=2) THEN
    RAISE EXCEPTION 'HOUSE_SLOT_OCCUPIED' USING ERRCODE='23514';END IF;
   UPDATE game_house_properties SET state=3,owner_character_id=c,owner_account_id=acct,
    ownership_generation=ownership_generation+1,revision=revision+1,acquisition_receipt_ref=o,
    paid_until=to_timestamp(((p->>'now_us')::bigint+2592000000000)::numeric/1000000) WHERE world_id=w AND house_key=a.house_key;
   UPDATE game_house_ownership SET owner_character_id=c,ownership_revision=ownership_revision+1,acquisition_receipt_ref=o
    WHERE world_id=w AND house_key=a.house_key;
   INSERT INTO game_account_housing_slots VALUES(acct,w,a.house_key);
  END IF;
 END IF;
 INSERT INTO game_house_audit_outbox(event_id,operation_id,envelope,occurred_at,expires_at,envelope_sha256)
 VALUES((p->>'event')::uuid,o,p_envelope,(p->>'now_us')::bigint/1000,(p->>'now_us')::bigint/1000+2592000000,sha256(p_envelope));
END $$;

CREATE FUNCTION game_house_settlement_step(p_auction UUID,p_step BIGINT,p_world UUID,p_house TEXT,p_empty BOOLEAN,
 p_node UUID,p_node_revision NUMERIC,p_secret BYTEA) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a game_house_auctions%ROWTYPE;
BEGIN
 IF game_node_prove_current_incarnation(p_node,p_node_revision,p_secret) IS DISTINCT FROM true THEN
  RAISE EXCEPTION 'HOUSE_NOT_CURRENT' USING ERRCODE='23514';END IF;
 SELECT * INTO a FROM game_house_auctions WHERE auction_id=p_auction FOR UPDATE;
 IF a.auction_id IS NULL OR (a.world_id,a.house_key)<>(p_world,p_house) OR clock_timestamp()<a.ends_at
  OR (p_step>0 AND NOT EXISTS(SELECT 1 FROM game_house_settlement_steps WHERE auction_id=p_auction AND step=p_step-1)) THEN
  RAISE EXCEPTION 'HOUSE_SETTLEMENT_ORDER' USING ERRCODE='23514';END IF;
 IF p_step=0 THEN
  IF (p_empty AND EXISTS(SELECT 1 FROM game_house_bids WHERE auction_id=p_auction AND state=2))
   OR (NOT p_empty AND NOT EXISTS(SELECT 1 FROM game_house_operations WHERE auction_id=p_auction
    AND kind=2 AND created_xact_id=pg_current_xact_id())) THEN
   RAISE EXCEPTION 'HOUSE_SETTLEMENT_WINNER_PROOF' USING ERRCODE='23514';END IF;
  UPDATE game_house_auctions SET settled=true WHERE auction_id=p_auction;
  IF p_empty THEN UPDATE game_house_properties SET state=1,revision=revision+1
   WHERE world_id=p_world AND house_key=p_house AND state=2;END IF;
 END IF;
 INSERT INTO game_house_settlement_steps VALUES(p_auction,p_step,p_world,p_house);
END $$;

CREATE TRIGGER game_house_operation_immutable BEFORE UPDATE OR DELETE ON game_house_operations
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_house_step_immutable BEFORE UPDATE OR DELETE ON game_house_settlement_steps
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
-- Reuse the Bank's publication/expiry guard and its event-keyed legal holds;
-- the shared hold relation has no Bank-only foreign key.
CREATE TRIGGER game_house_audit_guard BEFORE UPDATE OR DELETE ON game_house_audit_outbox
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_audit_guard();
REVOKE ALL ON game_house_settlement_steps,game_house_audit_outbox FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT ON game_house_settlement_steps TO oteryn_game_runtime;
GRANT SELECT,DELETE ON game_house_audit_outbox TO oteryn_game_runtime;
GRANT UPDATE(publication_state,published_at) ON game_house_audit_outbox TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_bank_entry_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_operation_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_value_apply(JSONB,UUID,NUMERIC,BYTEA,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_settlement_step(UUID,BIGINT,UUID,TEXT,BOOLEAN,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bank_entry_proven(),game_house_operation_proven(),game_house_value_apply(JSONB,UUID,NUMERIC,BYTEA,BYTEA),
 game_house_settlement_step(UUID,BIGINT,UUID,TEXT,BOOLEAN,UUID,NUMERIC,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_value_apply(JSONB,UUID,NUMERIC,BYTEA,BYTEA),
 game_house_settlement_step(UUID,BIGINT,UUID,TEXT,BOOLEAN,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;

-- Row locking requires UPDATE privilege in PostgreSQL. Keep that privilege
-- inside these bounded read/lock ports instead of granting runtime table DML.
CREATE FUNCTION game_house_property_lock(w UUID,h TEXT) RETURNS SETOF game_house_properties
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT * FROM game_house_properties WHERE world_id=w AND house_key=h FOR UPDATE $$;
CREATE FUNCTION game_house_auction_lock(w UUID,h TEXT,a UUID) RETURNS SETOF game_house_auctions
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT * FROM game_house_auctions WHERE world_id=w AND house_key=h
  AND ((a IS NULL AND NOT settled) OR auction_id=a) LIMIT 1 FOR UPDATE $$;
CREATE FUNCTION game_house_bids_lock(a UUID) RETURNS SETOF game_house_bids
 LANGUAGE sql SECURITY DEFINER AS $$ SELECT * FROM game_house_bids WHERE auction_id=a AND state=1 ORDER BY account_id LIMIT 257 FOR UPDATE $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_property_lock(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_auction_lock(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_bids_lock(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_property_lock(UUID,TEXT),game_house_auction_lock(UUID,TEXT,UUID),game_house_bids_lock(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_property_lock(UUID,TEXT),game_house_auction_lock(UUID,TEXT,UUID),game_house_bids_lock(UUID) TO oteryn_game_runtime;
