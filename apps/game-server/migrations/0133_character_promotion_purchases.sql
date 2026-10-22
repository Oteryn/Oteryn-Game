-- PREM-2b+PREM-5: one Character receipt, fee ledger and permanent service effect.
-- Rollback after receipts exist retains evidence and revokes new writes only.
ALTER TABLE game_item_fee_burns
    DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
    ALTER COLUMN charm_key DROP NOT NULL,
    ADD COLUMN purchase_service_key TEXT,
    ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (cause_kind IN (1,2,3)),
    ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (
      (cause_kind=1 AND charm_key IS NOT NULL AND purchase_service_key IS NULL)
      OR (cause_kind=2 AND charm_key IS NULL AND purchase_service_key IS NOT NULL
          AND purchase_service_key ~ '^oteryn:service[.]promotion[.][a-z0-9_]{1,80}$'
          AND bank_debit_gold_units=0 AND fee_gold_units=20000)
      OR (cause_kind=3 AND charm_key IS NULL AND purchase_service_key IS NOT NULL AND purchase_service_key IN (
          'oteryn:service.blessing.the_wisdom_of_solitude',
          'oteryn:service.blessing.blood_of_the_mountain',
          'oteryn:service.blessing.heart_of_the_mountain')));

CREATE TABLE game_character_npc_purchase_receipts (
    purchase_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(purchase_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    command_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(command_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding)=32),
    cause_kind SMALLINT NOT NULL CHECK (cause_kind IN (2,3)),
    npc_key TEXT NOT NULL CHECK (npc_key ~ '^oteryn:npc[.][a-z0-9_]{1,80}$'),
    npc_identity BYTEA NOT NULL CHECK (octet_length(npc_identity)=16),
    npc_generation NUMERIC(20,0) NOT NULL CHECK (npc_generation BETWEEN 1 AND 18446744073709551615),
    service_key TEXT NOT NULL CHECK (octet_length(service_key) BETWEEN 1 AND 128),
    blessing_key TEXT,
    fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns(transaction_id)
        DEFERRABLE INITIALLY DEFERRED,
    fee_gold_units BIGINT NOT NULL CHECK (fee_gold_units>0),
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision>=1),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision=original_character_revision+1
               AND committed_character_revision<=18446744073709551615),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after=level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before>=0),
    experience_after BIGINT NOT NULL CHECK (experience_after=experience_before),
    profile_revision TEXT NOT NULL,
    ruleset_revision TEXT NOT NULL,
    content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    reward_revision TEXT NOT NULL,
    committed_at BIGINT NOT NULL CHECK (committed_at>=0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, command_session_id, command_id),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (character_id, purchase_occurrence_id),
    CHECK ((cause_kind=2 AND blessing_key IS NULL
            AND service_key='oteryn:service.promotion.' || substr(npc_key,12)
            AND level_before>=20 AND fee_gold_units=20000)
        OR (cause_kind=3 AND blessing_key IS NOT NULL AND blessing_key='oteryn:blessing.' || substr(service_key,25)
            AND service_key IN ('oteryn:service.blessing.the_wisdom_of_solitude',
                                'oteryn:service.blessing.blood_of_the_mountain',
                                'oteryn:service.blessing.heart_of_the_mountain')))
);

CREATE TABLE game_character_promotions (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    purchase_occurrence_id UUID NOT NULL UNIQUE,
    FOREIGN KEY (character_id,purchase_occurrence_id)
        REFERENCES game_character_npc_purchase_receipts(character_id,purchase_occurrence_id)
        DEFERRABLE INITIALLY DEFERRED
);

-- Reuse the existing complete cross-kind guard, rather than copying its implementation.
-- That guard consumes only the common Character receipt columns of appearance receipts.
CREATE VIEW game_character_premium_revision_chain WITH (security_invoker=true) AS
    SELECT character_id,original_character_revision,committed_character_revision,
           level_before,level_after,experience_before,experience_after,profile_revision,
           ruleset_revision,content_revision,simulation_revision,evidence_revision,
           declaration_revision,policy_revision,reward_revision
      FROM game_character_appearance_receipts
    UNION ALL
    SELECT character_id,original_character_revision,committed_character_revision,
           level_before,level_after,experience_before,experience_after,profile_revision,
           ruleset_revision,content_revision,simulation_revision,evidence_revision,
           declaration_revision,policy_revision,reward_revision
      FROM game_character_npc_purchase_receipts;
DO $$
DECLARE body TEXT;
BEGIN
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO body;
    IF position('game_character_appearance_receipts' IN body)=0 THEN
        RAISE EXCEPTION 'expected appearance receipt arm in Character chain';
    END IF;
    EXECUTE replace(body,'game_character_appearance_receipts','game_character_premium_revision_chain');
END;
$$;

CREATE FUNCTION game_character_npc_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
          JOIN game_character_roots c ON c.character_id=f.character_id
         WHERE f.transaction_id=NEW.fee_transaction_id AND f.character_id=NEW.character_id
           AND f.cause_kind=NEW.cause_kind AND f.cause_occurrence_id=NEW.purchase_occurrence_id
           AND f.purchase_service_key=NEW.service_key AND f.fee_gold_units=NEW.fee_gold_units
           AND f.committed_character_revision=NEW.committed_character_revision
           AND f.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
           AND c.account_id=NEW.account_id AND c.character_revision>=NEW.committed_character_revision)
       OR (NEW.cause_kind=2 AND NOT EXISTS (
           SELECT 1 FROM game_character_promotions p WHERE p.character_id=NEW.character_id
             AND p.purchase_occurrence_id=NEW.purchase_occurrence_id))
       OR (NEW.cause_kind=3 AND NOT EXISTS (
           SELECT 1 FROM game_character_blessings b WHERE b.character_id=NEW.character_id
             AND b.blessing_key=NEW.blessing_key AND b.provenance=NEW.purchase_occurrence_id::text)) THEN
        RAISE EXCEPTION 'NPC purchase must commit with fee, receipt and effect' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_promotion_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_npc_purchase_receipts r
        WHERE r.character_id=NEW.character_id AND r.purchase_occurrence_id=NEW.purchase_occurrence_id
          AND r.cause_kind=2 AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'promotion requires this transaction purchase' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

-- The reverse proof prevents a new purchase cause from borrowing another receipt kind.
CREATE FUNCTION game_npc_purchase_fee_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind IN (2,3) AND NOT EXISTS (
        SELECT 1 FROM game_character_npc_purchase_receipts r
         WHERE r.fee_transaction_id=NEW.transaction_id AND r.cause_kind=NEW.cause_kind
           AND r.purchase_occurrence_id=NEW.cause_occurrence_id
           AND r.character_id=NEW.character_id AND r.service_key=NEW.purchase_service_key
           AND r.committed_character_revision=NEW.committed_character_revision
           AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'purchase fee requires its own Character purchase receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_blessing_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_npc_purchase_receipts r
        WHERE r.character_id=NEW.character_id AND r.blessing_key=NEW.blessing_key
          AND r.purchase_occurrence_id::text=NEW.provenance AND r.cause_kind=3
          AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'blessing requires this transaction purchase' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_npc_purchase_consistent AFTER INSERT ON game_character_npc_purchase_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_npc_purchase_guard();
CREATE CONSTRAINT TRIGGER game_npc_purchase_progression_consistent AFTER INSERT ON game_character_npc_purchase_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_promotion_purchase_consistent AFTER INSERT ON game_character_promotions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_promotion_guard();
CREATE CONSTRAINT TRIGGER game_blessing_purchase_consistent AFTER INSERT ON game_character_blessings
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_blessing_purchase_guard();
CREATE CONSTRAINT TRIGGER game_npc_purchase_fee_source AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_purchase_fee_source_guard();
CREATE TRIGGER game_npc_purchase_stamp BEFORE INSERT ON game_character_npc_purchase_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_npc_purchase_immutable BEFORE UPDATE OR DELETE ON game_character_npc_purchase_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_promotion_immutable BEFORE UPDATE OR DELETE ON game_character_promotions
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_purchase_no_truncate BEFORE TRUNCATE ON game_character_npc_purchase_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_promotion_no_truncate BEFORE TRUNCATE ON game_character_promotions
 EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE name TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_character_npc_purchase_guard()',
        'game_character_promotion_guard()', 'game_character_blessing_purchase_guard()',
        'game_npc_purchase_fee_source_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
    END LOOP;
END;
$$;
REVOKE ALL ON game_character_npc_purchase_receipts,game_character_promotions,
 game_character_premium_revision_chain FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_npc_purchase_receipts,game_character_promotions,
 game_character_blessings TO oteryn_game_runtime;
GRANT SELECT ON game_character_premium_revision_chain TO oteryn_game_runtime;
GRANT SELECT ON game_character_npc_purchase_receipts,game_character_promotions,
 game_character_premium_revision_chain TO oteryn_game_control;
