-- PRIVATE REVIEW PACKET, NOT AN INSTALLED MIGRATION OR A RESERVED NUMBER.
-- Append after the current final migration bodies at the root coordinator's fresh slot.
-- References: main cecbfda32b8d4cbb830566cf1b7d1173bef2a6cd; 0059/0060 remain immutable.
-- D1230: n=0..225, limit=100+20*n, next cost=25+n, no material/gold/RNG line.
-- Only ForgeCause::DustLimit is added. Existing Gain/Spend semantics and every earlier
-- ledger, bank-fee, custody and type-2 activation guard are preserved.
-- Rollback: forward-only successor that stops new reservations. Never erase a paid unlock,
-- immutable dust entry or receipt, and never reinterpret the former 225 bound as 225 unlocks.

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
-- Complete reviewed bundle includes its least grants. This file is not applied by merely
-- existing in proposal/; root selects the migration number only after fresh allocation.
GRANT SELECT,INSERT ON game_forge_dust_limit_reservations,game_forge_dust_limit_receipts,game_forge_dust_limit_history TO oteryn_game_runtime;
GRANT UPDATE(work_units_used) ON game_forge_dust_limit_reservations TO oteryn_game_runtime;
GRANT DELETE ON game_forge_dust_limit_history TO oteryn_game_runtime;
GRANT SELECT ON game_forge_dust_limit_reservations,game_forge_dust_limit_receipts,game_forge_dust_limit_history TO oteryn_game_control;
