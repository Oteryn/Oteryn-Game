-- FORGE-LIVE-1 / D1230: install the native DustLimit reservation, ledger and receipt owner.
-- Promoted from the private continuation packet; existing Gain/Spend accounting is preserved.

-- A legitimate pre-D1230 owner has no operation which changes 100. Verify the whole legacy
-- history while writes are excluded; reject unexplained rows rather than inventing unlocks.
LOCK TABLE game_character_forge_dust, game_character_forge_dust_entries IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM game_character_forge_dust WHERE dust_limit <> 100)
       OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries
                   WHERE dust_limit_before <> 100 OR dust_limit_after <> 100
                      OR NOT ((kind='GAIN' AND cause='creature_kill') OR (kind='SPEND' AND cause='proficiency'))
                      OR NOT (CASE kind
                           WHEN 'GAIN' THEN balance_after=balance_before+amount-lost_amount
                               AND (lost_amount=0 OR balance_after=100)
                           WHEN 'SPEND' THEN lost_amount=0 AND balance_after=balance_before-amount
                           ELSE FALSE END)) THEN
        RAISE EXCEPTION 'D1230 requires the proven legacy limit-100 history; no inferred unlock conversion'
            USING ERRCODE='23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_character_forge_dust_entries e
                LEFT JOIN game_character_forge_dust_entries p ON p.entry_id=e.previous_entry_id
                WHERE (e.previous_entry_id IS NULL AND e.balance_before<>0)
                   OR (e.previous_entry_id IS NOT NULL AND
                       (p.entry_id IS NULL OR p.character_id<>e.character_id OR p.balance_after<>e.balance_before)))
       OR EXISTS (SELECT 1 FROM game_character_forge_dust d
                   LEFT JOIN game_character_forge_dust_entries e ON e.entry_id=d.last_entry_id
                   WHERE (d.last_entry_id IS NULL AND (d.balance<>0 OR EXISTS
                       (SELECT 1 FROM game_character_forge_dust_entries z WHERE z.character_id=d.character_id)))
                      OR (d.last_entry_id IS NOT NULL AND (e.entry_id IS NULL OR e.character_id<>d.character_id
                          OR e.balance_after<>d.balance OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries s WHERE s.previous_entry_id=e.entry_id))))
       OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries e WHERE NOT EXISTS
                   (SELECT 1 FROM game_character_forge_dust d WHERE d.character_id=e.character_id)) THEN
        RAISE EXCEPTION 'D1230 legacy dust chain/balance proof failed' USING ERRCODE='23514';
    END IF;
    -- Unique predecessor and first-entry indexes make each reachable path linear. A detached
    -- cycle cannot be reached from a first entry; the count comparison detects it as well.
    IF (WITH RECURSIVE reachable(entry_id) AS (
            SELECT entry_id FROM game_character_forge_dust_entries WHERE previous_entry_id IS NULL
            UNION ALL
            SELECT e.entry_id FROM game_character_forge_dust_entries e JOIN reachable p ON e.previous_entry_id=p.entry_id
        ) SELECT count(*) FROM reachable)
        <> (SELECT count(*) FROM game_character_forge_dust_entries) THEN
        RAISE EXCEPTION 'D1230 legacy dust history has an unreachable component' USING ERRCODE='23514';
    END IF;
END $$;

ALTER TABLE game_character_forge_dust_entries
    DROP CONSTRAINT game_character_forge_dust_entries_dust_limit_before_check,
    DROP CONSTRAINT game_character_forge_dust_entries_dust_limit_after_check,
    DROP CONSTRAINT game_character_forge_dust_entry_cause_admitted,
    DROP CONSTRAINT game_character_forge_dust_entry_arithmetic,
    ADD CONSTRAINT game_character_forge_dust_entries_dust_limit_before_check
        CHECK (dust_limit_before BETWEEN 100 AND 4600 AND (dust_limit_before-100)%20=0),
    ADD CONSTRAINT game_character_forge_dust_entries_dust_limit_after_check
        CHECK (dust_limit_after BETWEEN 100 AND 4600 AND (dust_limit_after-100)%20=0),
    ADD CONSTRAINT game_character_forge_dust_entry_cause_admitted CHECK (
        (kind='GAIN' AND cause='creature_kill') OR (kind='SPEND' AND cause='proficiency')
        OR (kind='LIMIT_RAISE' AND cause='forge_dust_limit')),
    ADD CONSTRAINT game_character_forge_dust_entry_arithmetic CHECK (CASE kind
        WHEN 'GAIN' THEN dust_limit_after=dust_limit_before
            AND balance_after=balance_before+amount-lost_amount
            AND (lost_amount=0 OR balance_after=dust_limit_after)
        WHEN 'SPEND' THEN dust_limit_after=dust_limit_before AND lost_amount=0
            AND balance_after=balance_before-amount
        WHEN 'LIMIT_RAISE' THEN dust_limit_before<4600 AND dust_limit_after=dust_limit_before+20
            AND amount=25+(dust_limit_before-100)/20 AND lost_amount=0
            AND balance_after=balance_before-amount
        ELSE FALSE END);
ALTER TABLE game_character_forge_dust
    DROP CONSTRAINT game_character_forge_dust_limit_range,
    ADD CONSTRAINT game_character_forge_dust_limit_range
        CHECK (dust_limit BETWEEN 100 AND 4600 AND (dust_limit-100)%20=0);

-- Only IDs and the stable intent are frozen. Like TRANSFER, there is no frozen envelope;
-- commit selects the actual Type2 tuple under begin_type2_transaction's first-statement fence.
-- Thus the 0079 grandfather marker for persisted pre-activation event bytes does not apply.
CREATE TABLE game_forge_dust_limit_reservations (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(occurrence_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id) AND channel_id<>world_id),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id) AND event_id<>transaction_id),
    entry_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(entry_id) AND entry_id<>transaction_id AND entry_id<>event_id),
    occurred_at BIGINT NOT NULL CHECK (occurred_at BETWEEN 1 AND 9223372029078775807),
    expected_upgrade_count SMALLINT NOT NULL CHECK (expected_upgrade_count BETWEEN 0 AND 224),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest)=32 AND content_digest<>decode(repeat('00',32),'hex')),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision='D1230'),
    simulation_profile_revision BIGINT NOT NULL CHECK (simulation_profile_revision BETWEEN 1 AND 4294967295),
    work_units_used SMALLINT NOT NULL DEFAULT 0 CHECK (work_units_used BETWEEN 0 AND 3),
    PRIMARY KEY (character_id,occurrence_id),
    UNIQUE (character_id,occurrence_id,transaction_id,event_id,entry_id)
);

CREATE TABLE game_forge_dust_limit_receipts (
    character_id UUID NOT NULL,
    occurrence_id UUID NOT NULL,
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    entry_id UUID NOT NULL UNIQUE,
    outcome SMALLINT NOT NULL CHECK (outcome IN (1,2)), -- 1 Raised, 2 terminal REVISION_CHANGED
    previous_entry_id UUID,
    amount BIGINT,
    balance_before BIGINT,
    balance_after BIGINT,
    dust_limit_before SMALLINT,
    dust_limit_after SMALLINT,
    envelope_sha256 BYTEA,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,occurrence_id),
    FOREIGN KEY (character_id,occurrence_id,transaction_id,event_id,entry_id)
        REFERENCES game_forge_dust_limit_reservations(character_id,occurrence_id,transaction_id,event_id,entry_id),
    CONSTRAINT game_forge_dust_limit_receipt_closed CHECK (
        (outcome=1 AND previous_entry_id IS NOT NULL AND game_character_is_uuid_v7(previous_entry_id)
            AND amount IS NOT NULL AND balance_before IS NOT NULL AND balance_after IS NOT NULL
            AND dust_limit_before IS NOT NULL AND dust_limit_after IS NOT NULL AND envelope_sha256 IS NOT NULL
            AND octet_length(envelope_sha256)=32
            AND dust_limit_before BETWEEN 100 AND 4580 AND (dust_limit_before-100)%20=0
            AND dust_limit_after=dust_limit_before+20
            AND amount=25+(dust_limit_before-100)/20
            AND balance_before BETWEEN amount AND dust_limit_before AND balance_after=balance_before-amount)
        OR (outcome=2 AND previous_entry_id IS NULL AND amount IS NULL
            AND balance_before IS NULL AND balance_after IS NULL
            AND dust_limit_before IS NULL AND dust_limit_after IS NULL AND envelope_sha256 IS NULL))
);

-- One family's bounded history. There can be at most 225 successful DustLimit operations per
-- Character; the future combined Forge history must compose all admitted shapes into the
-- same 1000-row budget before it is exposed by FORGE_V1. This table grants no such wire API.
CREATE TABLE game_forge_dust_limit_history (
    character_id UUID NOT NULL,
    occurrence_id UUID NOT NULL,
    history_ordinal BIGINT NOT NULL CHECK (history_ordinal>0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,occurrence_id),
    UNIQUE (character_id,history_ordinal),
    FOREIGN KEY (character_id,occurrence_id) REFERENCES game_forge_dust_limit_receipts(character_id,occurrence_id)
);

CREATE FUNCTION game_forge_dust_limit_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='UPDATE' AND NEW.work_units_used=OLD.work_units_used+1
       AND (to_jsonb(NEW)-'work_units_used')=(to_jsonb(OLD)-'work_units_used') THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'Forge DustLimit reservation identity and revision binding are immutable' USING ERRCODE='23514';
END $$;

-- Check the receipt and its exact counterpart rows at the end of this physical transaction.
-- Invoked from both the receipt and the ledger: neither half can be committed alone or later.
CREATE FUNCTION game_forge_dust_limit_proof_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_receipt game_forge_dust_limit_receipts%ROWTYPE;
    v_reserved game_forge_dust_limit_reservations%ROWTYPE;
    v_character UUID;
    v_occurrence UUID;
    v_current_content BYTEA;
BEGIN
    IF TG_TABLE_NAME='game_character_forge_dust_entries' THEN
        IF NEW.kind<>'LIMIT_RAISE' THEN RETURN NULL; END IF;
        v_character:=NEW.character_id; v_occurrence:=NEW.cause_occurrence_id;
    ELSE
        v_character:=NEW.character_id; v_occurrence:=NEW.occurrence_id;
    END IF;
    SELECT * INTO v_receipt FROM game_forge_dust_limit_receipts WHERE character_id=v_character AND occurrence_id=v_occurrence;
    IF NOT FOUND OR v_receipt.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'Forge DustLimit requires its exact same-transaction terminal receipt' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT v_reserved FROM game_forge_dust_limit_reservations WHERE character_id=v_character AND occurrence_id=v_occurrence;
    IF v_reserved.work_units_used=0 THEN
        RAISE EXCEPTION 'Forge DustLimit attempt was not charged to its reservation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || v_reserved.world_id::text || ':' || v_reserved.channel_id::text,0));
    SELECT server_artifact_digest INTO v_current_content FROM game_content_activations
      WHERE world_id=v_reserved.world_id AND channel_id=v_reserved.channel_id ORDER BY activation_sequence DESC LIMIT 1;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Forge DustLimit has no independently active Content binding' USING ERRCODE='23514';
    END IF;
    IF v_receipt.outcome=2 THEN
        -- This initial material-free policy has the genuine profile revision 1. A successor
        -- which changes that revision must amend this policy proof with its writer; no runtime
        -- request can choose an arbitrary active revision to force this terminal state.
        IF (v_current_content=v_reserved.content_digest AND v_reserved.ruleset_revision='D1230'
             AND v_reserved.simulation_profile_revision=1)
           OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries e WHERE e.created_xact_id=pg_current_xact_id() OR e.transaction_id=v_receipt.transaction_id OR e.entry_id=v_receipt.entry_id
                        OR (e.character_id=v_character AND e.cause='forge_dust_limit' AND e.cause_occurrence_id=v_occurrence))
           OR EXISTS (SELECT 1 FROM game_item_audit_outbox a WHERE a.created_xact_id=pg_current_xact_id() OR a.transaction_id=v_receipt.transaction_id OR a.event_id=v_receipt.event_id)
           OR EXISTS (SELECT 1 FROM game_item_fee_burns f WHERE f.created_xact_id=pg_current_xact_id())
           OR EXISTS (SELECT 1 FROM game_forge_dust_limit_history h WHERE h.character_id=v_character AND h.occurrence_id=v_occurrence) THEN
            RAISE EXCEPTION 'Forge REVISION_CHANGED must prove changed active revisions and contain no effect/event/history' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
    IF v_current_content<>v_reserved.content_digest OR v_reserved.ruleset_revision<>'D1230'
       OR v_reserved.simulation_profile_revision<>1
       OR v_receipt.dust_limit_before<>100+20*v_reserved.expected_upgrade_count
       OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries extra
            WHERE extra.transaction_id=v_receipt.transaction_id AND extra.entry_id<>v_receipt.entry_id)
       OR NOT EXISTS (SELECT 1 FROM game_character_forge_dust_entries e
            WHERE e.entry_id=v_receipt.entry_id AND e.character_id=v_character
              AND e.previous_entry_id=v_receipt.previous_entry_id
              AND e.kind='LIMIT_RAISE' AND e.cause='forge_dust_limit' AND e.cause_occurrence_id=v_occurrence
              AND e.transaction_id=v_receipt.transaction_id AND e.amount=v_receipt.amount AND e.lost_amount=0
              AND (e.balance_before,e.balance_after,e.dust_limit_before,e.dust_limit_after)
                  =(v_receipt.balance_before,v_receipt.balance_after,v_receipt.dust_limit_before,v_receipt.dust_limit_after)
              AND e.created_xact_id=pg_current_xact_id())
       OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id=v_receipt.event_id AND a.transaction_id=v_receipt.transaction_id
              AND a.item_instance_id IS NULL AND a.event_type_id=2 AND a.transaction_ordinal=1 AND a.transaction_count=1
              AND a.occurred_at=v_reserved.occurred_at AND a.envelope_sha256=v_receipt.envelope_sha256
              AND sha256(a.envelope)=v_receipt.envelope_sha256 AND a.created_xact_id=pg_current_xact_id())
       OR NOT EXISTS (SELECT 1 FROM game_forge_dust_limit_history h
            WHERE h.character_id=v_character AND h.occurrence_id=v_occurrence AND h.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Forge DustLimit ledger/audit/history must equal its exact same-transaction receipt' USING ERRCODE='23514';
    END IF;
    -- For the entry-trigger invocation, its own new row must be the admitted one. A second
    -- different-ID LIMIT_RAISE in this transaction cannot borrow another row's proof.
    IF TG_TABLE_NAME='game_character_forge_dust_entries' AND NEW.entry_id<>v_receipt.entry_id THEN
        RAISE EXCEPTION 'Forge DustLimit extra ledger entry is not covered by the receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_forge_dust_limit_history_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN
        IF (SELECT count(*) FROM (SELECT 1 FROM game_forge_dust_limit_history h
             WHERE h.character_id=OLD.character_id AND h.history_ordinal>OLD.history_ordinal
             ORDER BY history_ordinal DESC LIMIT 1000) newer)=1000
           AND EXISTS (SELECT 1 FROM game_forge_dust_limit_history h WHERE h.character_id=OLD.character_id
                        AND h.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
        RAISE EXCEPTION 'Forge history deletes only the oldest overflow in its append transaction' USING ERRCODE='23514';
    ELSIF TG_OP='UPDATE' THEN
        RAISE EXCEPTION 'Forge history rows are immutable' USING ERRCODE='23514';
    ELSIF TG_WHEN='BEFORE' THEN
        IF NEW.history_ordinal<>COALESCE((SELECT history_ordinal FROM game_forge_dust_limit_history
             WHERE character_id=NEW.character_id ORDER BY history_ordinal DESC LIMIT 1),0)+1 THEN
            RAISE EXCEPTION 'Forge history ordinal must be the exact next ordinal' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM game_forge_dust_limit_receipts r WHERE r.character_id=NEW.character_id
                    AND r.occurrence_id=NEW.occurrence_id AND r.outcome=1 AND r.created_xact_id=pg_current_xact_id())
       OR (SELECT count(*) FROM (SELECT 1 FROM game_forge_dust_limit_history h
             WHERE h.character_id=NEW.character_id LIMIT 1001) bounded)>1000 THEN
        RAISE EXCEPTION 'Forge history must prove a same-transaction successful receipt and its 1000-row ceiling' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

-- Exact successor of the last body on main (0072). Preserve its bank-only fee proof verbatim;
-- extend the null-item case solely for this receipt's closed successful DustLimit shape.
CREATE OR REPLACE FUNCTION game_item_audit_item_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.item_instance_id IS NULL AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
         WHERE f.event_id = NEW.event_id
           AND f.transaction_id = NEW.transaction_id
           AND f.line_count = 0
           AND f.bank_debit_gold_units = f.fee_gold_units
           AND f.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (SELECT 1 FROM game_forge_dust_limit_receipts r
            WHERE r.event_id=NEW.event_id AND r.transaction_id=NEW.transaction_id AND r.outcome=1
              AND r.envelope_sha256=NEW.envelope_sha256 AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'null-item DUR-03 audit needs the exact bank-only fee or Forge DustLimit proof'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER game_forge_dust_limit_reservation_immutable BEFORE UPDATE OR DELETE
    ON game_forge_dust_limit_reservations FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_reservation_guard();
CREATE TRIGGER game_forge_dust_limit_reservation_no_truncate BEFORE TRUNCATE
    ON game_forge_dust_limit_reservations EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_forge_dust_limit_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_forge_dust_limit_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_forge_dust_limit_receipt_no_truncate BEFORE TRUNCATE
    ON game_forge_dust_limit_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_forge_dust_limit_receipt_stamp BEFORE INSERT
    ON game_forge_dust_limit_receipts FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE CONSTRAINT TRIGGER game_forge_dust_limit_receipt_proven AFTER INSERT
    ON game_forge_dust_limit_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_proof_guard();
CREATE CONSTRAINT TRIGGER game_forge_dust_limit_entry_proven AFTER INSERT
    ON game_character_forge_dust_entries DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_proof_guard();
CREATE TRIGGER game_forge_dust_limit_history_order BEFORE INSERT
    ON game_forge_dust_limit_history FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_history_guard();
CREATE TRIGGER game_forge_dust_limit_history_immutable BEFORE UPDATE OR DELETE
    ON game_forge_dust_limit_history FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_history_guard();
CREATE TRIGGER game_forge_dust_limit_history_stamp BEFORE INSERT
    ON game_forge_dust_limit_history FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_forge_dust_limit_history_no_truncate BEFORE TRUNCATE
    ON game_forge_dust_limit_history EXECUTE FUNCTION game_character_reject_truncate();
CREATE CONSTRAINT TRIGGER game_forge_dust_limit_history_proven AFTER INSERT
    ON game_forge_dust_limit_history DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_dust_limit_history_guard();

DO $$
DECLARE v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_forge_dust_limit_reservation_guard()', 'game_forge_dust_limit_proof_guard()',
        'game_forge_dust_limit_history_guard()', 'game_item_audit_item_proven()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',v_function,current_schema());
    END LOOP;
END $$;
REVOKE ALL ON game_forge_dust_limit_reservations,game_forge_dust_limit_receipts,game_forge_dust_limit_history FROM PUBLIC;
REVOKE ALL ON FUNCTION game_forge_dust_limit_reservation_guard(),game_forge_dust_limit_proof_guard(),
    game_forge_dust_limit_history_guard(),game_item_audit_item_proven() FROM PUBLIC;
-- The installed native DustLimit owner receives only its reviewed grants.
GRANT SELECT,INSERT ON game_forge_dust_limit_reservations,game_forge_dust_limit_receipts,game_forge_dust_limit_history TO oteryn_game_runtime;
GRANT UPDATE(work_units_used) ON game_forge_dust_limit_reservations TO oteryn_game_runtime;
GRANT DELETE ON game_forge_dust_limit_history TO oteryn_game_runtime;
GRANT SELECT ON game_forge_dust_limit_reservations,game_forge_dust_limit_receipts,game_forge_dust_limit_history TO oteryn_game_control;

-- FORGE-LIVE-1: staged SQL prototype, NOT a native gameplay value owner.
-- Runtime is denied access: native dust, items, tiers and fees remain authoritative.
-- This prototype must not be enabled before native ForgeCause composition replaces it.
CREATE TABLE game_character_forge_wallets (
 character_id UUID PRIMARY KEY, gold BIGINT NOT NULL CHECK(gold>=0), dust INTEGER NOT NULL CHECK(dust>=0),
 dust_limit INTEGER NOT NULL DEFAULT 100 CHECK(dust_limit BETWEEN 100 AND 4600),
 slivers INTEGER NOT NULL DEFAULT 0 CHECK(slivers>=0), cores INTEGER NOT NULL DEFAULT 0 CHECK(cores>=0), revision BIGINT NOT NULL DEFAULT 0 CHECK(revision>=0)
);
CREATE TABLE game_character_forge_items (
 item_instance_id UUID PRIMARY KEY, character_id UUID NOT NULL, classification SMALLINT NOT NULL CHECK(classification BETWEEN 1 AND 4),
 tier SMALLINT NOT NULL CHECK(tier BETWEEN 0 AND 10), active BOOLEAN NOT NULL DEFAULT true,
 UNIQUE(character_id,item_instance_id)
);
CREATE TABLE game_character_forge_receipts (
 character_id UUID NOT NULL, occurrence UUID NOT NULL, request JSONB NOT NULL, result JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(), PRIMARY KEY(character_id,occurrence)
);
CREATE TABLE game_character_forge_history (
 character_id UUID NOT NULL, ordinal BIGINT GENERATED ALWAYS AS IDENTITY, occurrence UUID NOT NULL,
 operation TEXT NOT NULL, succeeded BOOLEAN NOT NULL, result JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 PRIMARY KEY(character_id,ordinal), UNIQUE(character_id,occurrence)
);
CREATE INDEX game_character_forge_history_recent ON game_character_forge_history(character_id,ordinal DESC);

-- The single mutation entry point locks wallet then items in UUID order. A repeated occurrence
-- returns its frozen result; a different request for that occurrence is rejected.
CREATE FUNCTION game_commit_character_forge(p_character UUID,p_occurrence UUID,p_request JSONB) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT AS $$
DECLARE w game_character_forge_wallets%ROWTYPE; prior game_character_forge_receipts%ROWTYPE;
 op TEXT:=p_request->>'operation'; a UUID; b UUID; ia game_character_forge_items%ROWTYPE; ib game_character_forge_items%ROWTYPE;
 gold_cost BIGINT:=0; dust_cost INT:=0; core_cost INT:=0; ok BOOLEAN; roll INT; r JSONB; new_tier INT;
BEGIN
 IF p_request IS NULL OR jsonb_typeof(p_request)<>'object' THEN RAISE EXCEPTION 'invalid forge request'; END IF;
 SELECT * INTO w FROM game_character_forge_wallets WHERE character_id=p_character FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'forge wallet unavailable'; END IF;
 SELECT * INTO prior FROM game_character_forge_receipts WHERE character_id=p_character AND occurrence=p_occurrence;
 IF FOUND THEN IF prior.request<>p_request THEN RAISE EXCEPTION 'conflicting forge occurrence'; END IF; RETURN prior.result; END IF;
 IF op='dust_to_slivers' THEN
  IF w.dust<60 THEN RAISE EXCEPTION 'insufficient forge resources'; END IF;
  UPDATE game_character_forge_wallets SET dust=dust-60,slivers=slivers+3,revision=revision+1 WHERE character_id=p_character; ok:=true;
 ELSIF op='slivers_to_cores' THEN
  IF w.slivers<50 THEN RAISE EXCEPTION 'insufficient forge resources'; END IF;
  UPDATE game_character_forge_wallets SET slivers=slivers-50,cores=cores+1,revision=revision+1 WHERE character_id=p_character; ok:=true;
 ELSIF op='dust_limit' THEN
  IF w.dust_limit>=4600 OR w.dust<25+((w.dust_limit-100)/20) THEN RAISE EXCEPTION 'insufficient forge resources'; END IF;
  UPDATE game_character_forge_wallets SET dust=dust-(25+((dust_limit-100)/20)),dust_limit=dust_limit+20,revision=revision+1 WHERE character_id=p_character; ok:=true;
 ELSIF op IN ('fusion','convergence_fusion','transfer','convergence_transfer') THEN
  a:=(p_request->>'first_item')::uuid; b:=(p_request->>'second_item')::uuid;
  IF a=b THEN RAISE EXCEPTION 'forge items must differ'; END IF;
  PERFORM 1 FROM game_character_forge_items WHERE item_instance_id IN(a,b) ORDER BY item_instance_id FOR UPDATE;
  SELECT * INTO ia FROM game_character_forge_items WHERE item_instance_id=a AND character_id=p_character AND active;
  SELECT * INTO ib FROM game_character_forge_items WHERE item_instance_id=b AND character_id=p_character AND active;
  IF ia.item_instance_id IS NULL OR ib.item_instance_id IS NULL OR ia.classification<>ib.classification THEN RAISE EXCEPTION 'invalid forge custody'; END IF;
  -- Pinned Canary 04b83b51 catalogue; reference evidence, not Content activation.
  new_tier:=CASE WHEN op LIKE '%fusion' THEN ia.tier+1 ELSE ib.tier END;
  gold_cost:=CASE ia.classification*100+new_tier
   WHEN 101 THEN 25000 WHEN 201 THEN 750000 WHEN 202 THEN 5000000
   WHEN 301 THEN 4000000 WHEN 302 THEN 10000000 WHEN 303 THEN 20000000
   WHEN 401 THEN 8000000 WHEN 402 THEN 20000000 WHEN 403 THEN 40000000 WHEN 404 THEN 65000000
   WHEN 405 THEN 100000000 WHEN 406 THEN 250000000 WHEN 407 THEN 750000000 WHEN 408 THEN 2500000000
   WHEN 409 THEN 8000000000 WHEN 410 THEN 15000000000 ELSE -1 END;
  IF gold_cost<0 THEN RAISE EXCEPTION 'unsupported forge catalogue row'; END IF;
  IF op='fusion' THEN dust_cost:=100; ELSIF op='convergence_fusion' THEN dust_cost:=130;
  ELSIF op='transfer' THEN dust_cost:=100; core_cost:=CASE new_tier WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 5 WHEN 4 THEN 10 WHEN 5 THEN 15 WHEN 6 THEN 25 WHEN 7 THEN 35 WHEN 8 THEN 50 WHEN 9 THEN 60 WHEN 10 THEN 85 ELSE 1 END;
  ELSE dust_cost:=160; core_cost:=CASE new_tier WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 5 WHEN 4 THEN 10 WHEN 5 THEN 15 WHEN 6 THEN 25 WHEN 7 THEN 35 WHEN 8 THEN 50 WHEN 9 THEN 60 WHEN 10 THEN 85 ELSE 1 END; END IF;
  IF op LIKE 'convergence_%' THEN
   IF ia.classification<>4 THEN RAISE EXCEPTION 'convergence requires classification four'; END IF;
   gold_cost:=CASE WHEN op='convergence_fusion' THEN
    CASE new_tier WHEN 1 THEN 55000000 WHEN 2 THEN 110000000 WHEN 3 THEN 170000000
     WHEN 4 THEN 300000000 WHEN 5 THEN 875000000 WHEN 6 THEN 2350000000
     WHEN 7 THEN 6950000000 WHEN 8 THEN 21250000000 WHEN 9 THEN 50000000000
     WHEN 10 THEN 125000000000 ELSE -1 END
    ELSE CASE new_tier WHEN 1 THEN 65000000 WHEN 2 THEN 165000000 WHEN 3 THEN 375000000
     WHEN 4 THEN 800000000 WHEN 5 THEN 2000000000 WHEN 6 THEN 5250000000
     WHEN 7 THEN 14500000000 WHEN 8 THEN 42500000000 WHEN 9 THEN 100000000000
     WHEN 10 THEN 300000000000 ELSE -1 END END;
   IF gold_cost<0 THEN RAISE EXCEPTION 'unsupported convergence catalogue row'; END IF;
  END IF;
  IF w.gold<gold_cost OR w.dust<dust_cost OR w.cores<core_cost THEN RAISE EXCEPTION 'insufficient forge resources'; END IF;
  roll:=((hashtextextended(p_occurrence::text||op,0)%100)+100)%100;
  ok:=op LIKE 'convergence_%' OR roll<50;
  UPDATE game_character_forge_wallets SET gold=gold-gold_cost,dust=dust-dust_cost,cores=cores-core_cost,revision=revision+1 WHERE character_id=p_character;
  IF op LIKE '%fusion' THEN
   IF ia.tier<>ib.tier OR ia.tier>=(CASE ia.classification WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 3 ELSE 10 END) THEN RAISE EXCEPTION 'invalid fusion tiers'; END IF;
   UPDATE game_character_forge_items SET active=false WHERE item_instance_id=b;
   IF ok THEN UPDATE game_character_forge_items SET tier=tier+1 WHERE item_instance_id=a; new_tier:=ia.tier+1;
   ELSE new_tier:=ia.tier;
    IF ib.tier>0 THEN UPDATE game_character_forge_items SET active=true,tier=tier-1 WHERE item_instance_id=b; END IF;
   END IF;
  ELSE
   IF ia.tier<>0 OR ib.tier<2 THEN RAISE EXCEPTION 'invalid transfer tier'; END IF;
   new_tier:=CASE WHEN op='convergence_transfer' THEN ib.tier ELSE ib.tier-1 END;
   UPDATE game_character_forge_items SET tier=new_tier WHERE item_instance_id=a;
   UPDATE game_character_forge_items SET active=false WHERE item_instance_id=b; ok:=true;
  END IF;
 ELSE RAISE EXCEPTION 'unknown forge operation'; END IF;
 r:=jsonb_build_object('operation',op,'succeeded',ok,'tier',new_tier,'revision',w.revision+1);
 INSERT INTO game_character_forge_receipts VALUES(p_character,p_occurrence,p_request,r,statement_timestamp());
 INSERT INTO game_character_forge_history(character_id,occurrence,operation,succeeded,result) VALUES(p_character,p_occurrence,op,ok,r);
 RETURN r;
END;
$$;
REVOKE ALL ON FUNCTION game_commit_character_forge(UUID,UUID,JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_commit_character_forge(UUID,UUID,JSONB)
 FROM oteryn_game_runtime, oteryn_game_control;
REVOKE ALL ON game_character_forge_wallets, game_character_forge_items,
 game_character_forge_history, game_character_forge_receipts
 FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
