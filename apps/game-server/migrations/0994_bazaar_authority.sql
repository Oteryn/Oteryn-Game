CREATE FUNCTION game_bazaar_source_revision(raw BYTEA) RETURNS NUMERIC LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE v TEXT;BEGIN v:=convert_from(raw,'UTF8')::jsonb->>'source_revision';
 IF v IS NULL OR v !~ '^[1-9][0-9]{0,19}$' OR v::numeric>18446744073709551615 THEN RETURN NULL;END IF;
 RETURN v::numeric;EXCEPTION WHEN OTHERS THEN RETURN NULL;END $$;
-- GPROD-BAZAAR: authority decisions and custody survive flag rollback. No TTL
-- releases a lock, item or gold hold. The operator explicitly supplies policy.
CREATE TABLE game_bazaar_configuration (
 singleton BOOLEAN PRIMARY KEY DEFAULT true CHECK(singleton),
 enabled BOOLEAN NOT NULL DEFAULT true,
 policy JSONB NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(policy)='object' AND octet_length(policy::text)<=8192)
);
INSERT INTO game_bazaar_configuration DEFAULT VALUES;
CREATE TABLE game_bazaar_decisions (
 issuer TEXT NOT NULL CHECK(issuer IN ('OTERYN_PLATFORM_BAZAAR_AUTHORITY','OTERYN_PLATFORM_CHARACTER_AUTHORITY')),
 operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(operation_id)),
 server_build_id TEXT NOT NULL DEFAULT 'oteryn-game-server/0.1.0' CHECK(octet_length(server_build_id) BETWEEN 1 AND 128),
 command TEXT NOT NULL, intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 4096),
 intent_digest BYTEA GENERATED ALWAYS AS (sha256(intent)) STORED,
 source_revision NUMERIC(20) GENERATED ALWAYS AS (game_bazaar_source_revision(intent)) STORED,
 receipt BYTEA NOT NULL CHECK(octet_length(receipt) BETWEEN 1 AND 270336),
 receipt_digest BYTEA GENERATED ALWAYS AS (sha256(receipt)) STORED,
 result_code TEXT NOT NULL CHECK(result_code ~ '^CHAR_CMD_[A-Z_]+$'),
 recovery_generation NUMERIC(20) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
 decided_at BIGINT NOT NULL CHECK(decided_at>0),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 world_id UUID NULL CHECK(game_character_is_uuid_v7(world_id)),
 payload BYTEA NULL CHECK(octet_length(payload) BETWEEN 1 AND 79),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(issuer,operation_id)
);
CREATE UNIQUE INDEX game_bazaar_transfer_source_revision ON game_bazaar_decisions(source_revision) WHERE issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND command='TransferCharacterOwnership' AND source_revision IS NOT NULL AND result_code<>'CHAR_CMD_OPERATION_CONFLICT';
CREATE TABLE game_bazaar_outbox (
 issuer TEXT NOT NULL, operation_id UUID NOT NULL,
 receipt_acknowledged BOOLEAN NOT NULL DEFAULT false,
 audit_acknowledged BOOLEAN NOT NULL DEFAULT false,
 stopped BOOLEAN NOT NULL DEFAULT false,
 PRIMARY KEY(issuer,operation_id),
 FOREIGN KEY(issuer,operation_id) REFERENCES game_bazaar_decisions(issuer,operation_id)
);
CREATE INDEX game_bazaar_outbox_pending ON game_bazaar_outbox(issuer,operation_id) WHERE NOT receipt_acknowledged AND NOT stopped;
CREATE TABLE game_bazaar_character_transitions (
 operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(operation_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 old_account UUID NOT NULL, new_account UUID NOT NULL, old_lifecycle SMALLINT NOT NULL, new_lifecycle SMALLINT NOT NULL,
 old_revision NUMERIC(20) NOT NULL, new_revision NUMERIC(20) NOT NULL CHECK(new_revision=old_revision+1),
 issuer TEXT NOT NULL,
 PRIMARY KEY(issuer,operation_id),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(issuer,operation_id) REFERENCES game_bazaar_decisions(issuer,operation_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_bazaar_character_transitions_character ON game_bazaar_character_transitions(character_id,new_revision DESC);
CREATE TABLE game_bazaar_locks (
 lock_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(lock_id)),
 listing_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(listing_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 seller_account_id UUID NOT NULL, world_id UUID NOT NULL,
 revision NUMERIC(20) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 state TEXT NOT NULL CHECK(state IN ('locked','UNLOCKED','INVALID')),
 snapshot BYTEA NOT NULL CHECK(octet_length(snapshot) BETWEEN 1 AND 262144),
 snapshot_hash BYTEA NOT NULL CHECK(snapshot_hash=sha256(snapshot)),
 signed_snapshot BYTEA NOT NULL CHECK(octet_length(signed_snapshot) BETWEEN 1 AND 270000),
 transfer_operation_id UUID NULL, transfer_digest BYTEA NULL CHECK(octet_length(transfer_digest)=32),
 transfer_purpose TEXT NULL, transfer_deadline BIGINT NULL,
 terminal_owner UUID NULL, last_operation_id UUID NOT NULL,
 CHECK((transfer_operation_id IS NULL)=(transfer_digest IS NULL))
);
CREATE UNIQUE INDEX game_bazaar_one_active_character_lock ON game_bazaar_locks(character_id) WHERE state IN('locked','INVALID');
CREATE TABLE game_bazaar_escrows (
 escrow_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(escrow_id)),
 listing_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(listing_id)),
 seller_account_id UUID NOT NULL, seller_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL, item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 revision NUMERIC(20) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 state TEXT NOT NULL CHECK(state IN ('ESCROWED','DELIVERED','RETURNED','INVALID')),
 item BYTEA NOT NULL CHECK(octet_length(item) BETWEEN 1 AND 4096),
 attributes BYTEA NOT NULL CHECK(octet_length(attributes)<=4096), attributes_hash BYTEA NOT NULL CHECK(attributes_hash=sha256(attributes)),
 escrow_receipt_digest BYTEA NULL CHECK(octet_length(escrow_receipt_digest)=32),
 destination_account_id UUID NULL, destination_character_id UUID NULL,
 sale_id UUID NULL, funding BYTEA NULL, last_operation_id UUID NOT NULL
);
CREATE UNIQUE INDEX game_bazaar_one_active_item_escrow ON game_bazaar_escrows(item_instance_id) WHERE state IN('ESCROWED','INVALID');
CREATE INDEX game_bazaar_escrows_seller_character ON game_bazaar_escrows(seller_character_id);
CREATE TABLE game_bazaar_item_locations (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 escrow_id UUID NOT NULL UNIQUE REFERENCES game_bazaar_escrows(escrow_id)
);
CREATE TABLE game_bazaar_gold_holds (
 hold_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(hold_id)),
 listing_id UUID NOT NULL, offer_id UUID NOT NULL, buyer_account_id UUID NOT NULL, seller_account_id UUID NOT NULL, world_id UUID NOT NULL,
 purpose TEXT NOT NULL CHECK(purpose IN ('OFFER','RESERVE')),
 revision NUMERIC(20) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 999999999999),
 incoming_net BIGINT NOT NULL CHECK(incoming_net BETWEEN 0 AND 999999999999),
 state TEXT NOT NULL CHECK(state IN ('ACTIVE','RELEASED','DISPOSITION_PENDING','AWAITING_PAYOUT','PAID')),
 sale_id UUID NULL, gross_amount BIGINT NULL, delivery_operation_id UUID NULL, delivery_receipt_digest BYTEA NULL,
 UNIQUE(listing_id,offer_id,buyer_account_id,world_id), CHECK(buyer_account_id<>seller_account_id),
 CHECK((state='RELEASED')=(amount=0))
);
CREATE INDEX game_bazaar_gold_holds_buyer ON game_bazaar_gold_holds(buyer_account_id,world_id) WHERE amount>0;
CREATE INDEX game_bazaar_gold_holds_seller ON game_bazaar_gold_holds(seller_account_id,world_id) WHERE incoming_net>0;
CREATE TABLE game_bazaar_publication_outbox (
 subject TEXT PRIMARY KEY, epoch NUMERIC(20) NOT NULL, revision NUMERIC(20) NOT NULL,
 body BYTEA NOT NULL CHECK(octet_length(body) BETWEEN 1 AND 262144),
 committed_at BIGINT NOT NULL, acknowledged BOOLEAN NOT NULL DEFAULT false,
 operation TEXT NOT NULL CHECK(operation IN ('PublishBazaarListingStatusV1','PublishBazaarTransferabilityV1'))
);
CREATE INDEX game_bazaar_publication_pending ON game_bazaar_publication_outbox(subject) WHERE NOT acknowledged;

-- Add an exact same-transaction proof arm to the existing root guard. The
-- original revision-only arm remains byte-for-byte after the new arm.
ALTER TABLE game_character_roots DROP CONSTRAINT game_character_roots_lifecycle_check;
ALTER TABLE game_character_roots ADD CONSTRAINT game_character_roots_lifecycle_check CHECK(lifecycle IN(1,2));
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_root_revision_guard()'::regprocedure) INTO body;
 body:=replace(body,E'BEGIN\n',E'BEGIN\n    IF TG_OP=\'UPDATE\' AND NEW.character_id=OLD.character_id AND NEW.world_id=OLD.world_id\n       AND NEW.profile_revision=OLD.profile_revision AND NEW.ruleset_revision=OLD.ruleset_revision\n       AND NEW.content_revision=OLD.content_revision AND NEW.starter_template_revision=OLD.starter_template_revision\n       AND NEW.name=OLD.name AND NEW.sex IS NOT DISTINCT FROM OLD.sex\n       AND EXISTS(SELECT 1 FROM game_bazaar_character_transitions t WHERE t.character_id=OLD.character_id\n          AND t.old_account=OLD.account_id AND t.new_account=NEW.account_id\n          AND t.old_lifecycle=OLD.lifecycle AND t.new_lifecycle=NEW.lifecycle\n          AND t.old_revision=OLD.character_revision AND t.new_revision=NEW.character_revision\n          AND t.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;\n    IF OLD.lifecycle=2 THEN RAISE EXCEPTION \'Bazaar locked Character denies mutation\' USING ERRCODE=\'23514\'; END IF;\n');
 EXECUTE body;
END $$;
-- Admission and its current evidence share the existing admission relation lock.
-- No stale Platform eligibility can reopen a locked root or an old owner.
CREATE FUNCTION game_bazaar_admission_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- Add the Bazaar fence only to locked or Bazaar-transitioned roots. Other
 -- admission evidence retains its existing owner/World validation path.
 IF NEW.eligible AND EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id
 AND (r.lifecycle<>1 OR EXISTS(SELECT 1 FROM game_bazaar_character_transitions t WHERE t.character_id=r.character_id))
 AND (r.lifecycle<>1 OR r.account_id<>NEW.account_id OR r.world_id<>NEW.world_id)) THEN
 RAISE EXCEPTION 'Bazaar admission owner/lifecycle fence' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_bazaar_admission_guard BEFORE INSERT OR UPDATE ON game_durability_admission_character_guards FOR EACH ROW EXECUTE FUNCTION game_bazaar_admission_guard();

-- Add custody to the complete location union, retaining every existing arm.
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_location_exclusive(uuid)'::regprocedure) INTO body;
 IF position('IF v_locations <>' in body)=0 THEN RAISE EXCEPTION 'Bazaar location union anchor missing';END IF;
 body:=replace(body,'IF v_locations <>','v_locations := v_locations + (SELECT count(*) FROM game_bazaar_item_locations WHERE item_instance_id=p_item); IF v_locations <>');
 EXECUTE body;
END $$;
CREATE FUNCTION game_bazaar_location_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' THEN PERFORM game_item_location_exclusive(OLD.item_instance_id);ELSE PERFORM game_item_location_exclusive(NEW.item_instance_id);END IF;RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_bazaar_location_guard AFTER INSERT OR DELETE ON game_bazaar_item_locations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bazaar_location_guard();
INSERT INTO game_character_inbox_cause_kinds VALUES('BAZAAR_DELIVERY'),('BAZAAR_RETURN');

-- Every ordinary bank writer already updates the one balance row. This guard
-- protects held funds and reserved seller capacity, including fee/death paths.
CREATE FUNCTION game_bazaar_bank_fence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE held NUMERIC; incoming NUMERIC;
BEGIN
 -- Native owners initialize zero with INSERT ON CONFLICT DO NOTHING before
 -- locking the existing balance. A non-inserted candidate is not a debit.
 IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_account_bank_balances WHERE account_id=NEW.account_id AND world_id=NEW.world_id) THEN RETURN NEW;END IF;
 SELECT coalesce(sum(amount),0) INTO held FROM game_bazaar_gold_holds WHERE buyer_account_id=NEW.account_id AND world_id=NEW.world_id;
 SELECT coalesce(sum(incoming_net),0) INTO incoming FROM game_bazaar_gold_holds WHERE seller_account_id=NEW.account_id AND world_id=NEW.world_id;
 IF NEW.balance<held OR ((held>0 OR incoming>0) AND NEW.balance+incoming>999999999999) THEN RAISE EXCEPTION 'Bazaar bank hold/headroom fence' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_bazaar_bank_fence BEFORE INSERT OR UPDATE ON game_account_bank_balances FOR EACH ROW EXECUTE FUNCTION game_bazaar_bank_fence();

CREATE FUNCTION game_bazaar_decision_outbox() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 INSERT INTO game_bazaar_outbox(issuer,operation_id) VALUES(NEW.issuer,NEW.operation_id); RETURN NULL;END $$;
CREATE TRIGGER game_bazaar_decision_outbox AFTER INSERT ON game_bazaar_decisions FOR EACH ROW EXECUTE FUNCTION game_bazaar_decision_outbox();
CREATE TRIGGER game_bazaar_decision_immutable BEFORE UPDATE OR DELETE ON game_bazaar_decisions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_bazaar_transition_immutable BEFORE UPDATE OR DELETE ON game_bazaar_character_transitions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['configuration','decisions','outbox','character_transitions','locks','escrows','item_locations','gold_holds','publication_outbox'] LOOP
 EXECUTE format('REVOKE ALL ON game_bazaar_%I FROM PUBLIC',n);
 EXECUTE format('GRANT SELECT ON game_bazaar_%I TO oteryn_game_control,oteryn_game_runtime',n);
 EXECUTE format('CREATE TRIGGER game_bazaar_no_truncate BEFORE TRUNCATE ON game_bazaar_%I EXECUTE FUNCTION game_character_reject_truncate()',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['decisions','character_transitions'] LOOP EXECUTE format('GRANT INSERT ON game_bazaar_%I TO oteryn_game_runtime',n);END LOOP;
 FOREACH n IN ARRAY ARRAY['outbox','locks','escrows','gold_holds','publication_outbox'] LOOP EXECUTE format('GRANT INSERT,UPDATE ON game_bazaar_%I TO oteryn_game_runtime',n);END LOOP;
 GRANT INSERT,DELETE ON game_bazaar_item_locations TO oteryn_game_runtime;
END $$;
-- Policy is deployment configuration. Only the migration/operator identity may
-- change it; no Game runtime or Platform SQL writer receives that permission.
DO $$ DECLARE n TEXT;BEGIN
 FOR n IN SELECT oid::regprocedure::text FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname LIKE 'game_bazaar_%' LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());END LOOP;
END $$;

GRANT UPDATE(account_id,lifecycle) ON game_character_roots TO oteryn_game_runtime;
