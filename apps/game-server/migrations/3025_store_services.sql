-- Reserve the complete pack before Platform can take coins. A settled reservation
-- remains linked to the receipt so an operation cannot serve a second purchase.
CREATE TABLE game_store_service_purchases (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    kind TEXT NOT NULL,
    amount SMALLINT NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    reset_epoch NUMERIC(20,0),
    blessing_keys TEXT[] NOT NULL,
    CHECK ((kind='PREY_WILDCARDS' AND amount IN (5,20) AND reset_epoch IS NULL AND cardinality(blessing_keys)=0)
        OR (kind='XP_BOOST' AND amount BETWEEN 1 AND 5 AND reset_epoch IS NOT NULL AND cardinality(blessing_keys)=0)
        OR (kind='BLESSINGS' AND amount IN (1,5) AND reset_epoch IS NULL
            AND cardinality(blessing_keys) IN (1,8) AND array_position(blessing_keys,NULL) IS NULL)
        OR (kind IN ('WEEKLY_TASK_EXPANSION','INSTANT_REWARD_ACCESS') AND amount=1 AND reset_epoch IS NULL AND cardinality(blessing_keys)=0)
        OR (kind='PREMIUM_TIME' AND amount IN (30,90,180,360) AND reset_epoch IS NULL AND cardinality(blessing_keys)=0)),
    operation_id UUID UNIQUE REFERENCES store_delivery_receipts(operation_id)
);
CREATE INDEX game_store_service_purchases_account_world
    ON game_store_service_purchases(account_id,world_id);
CREATE INDEX game_store_service_purchase_character ON game_store_service_purchases(character_id);
CREATE UNIQUE INDEX game_store_blessing_purchase_pending
    ON game_store_service_purchases(character_id) WHERE operation_id IS NULL AND kind='BLESSINGS';
CREATE UNIQUE INDEX game_store_prey_purchase_pending
    ON game_store_service_purchases(account_id,world_id) WHERE operation_id IS NULL AND kind='PREY_WILDCARDS';
CREATE UNIQUE INDEX game_store_reward_access_pending
    ON game_store_service_purchases(character_id) WHERE operation_id IS NULL AND kind='INSTANT_REWARD_ACCESS';
CREATE INDEX game_store_reward_access_character
    ON game_store_service_purchases(character_id,world_id,purchase_id)
    WHERE operation_id IS NOT NULL AND kind='INSTANT_REWARD_ACCESS';
CREATE UNIQUE INDEX game_store_task_expansion_once
    ON game_store_service_purchases(character_id) WHERE kind='WEEKLY_TASK_EXPANSION';
CREATE INDEX game_task_board_expansion_deliveries_character
    ON game_task_board_expansion_deliveries(character_id);
CREATE UNIQUE INDEX game_store_xp_purchase_pending
    ON game_store_service_purchases(character_id) WHERE operation_id IS NULL AND kind='XP_BOOST';
CREATE UNIQUE INDEX game_store_xp_purchase_tier
    ON game_store_service_purchases(character_id,world_id,reset_epoch,amount) WHERE kind='XP_BOOST';
REVOKE ALL ON game_store_service_purchases FROM PUBLIC;
GRANT SELECT, INSERT, DELETE, UPDATE(operation_id) ON game_store_service_purchases TO oteryn_game_runtime;
GRANT SELECT ON game_store_service_purchases TO oteryn_game_control;

ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT prey_wildcard_entry_values;
ALTER TABLE game_account_prey_wildcard_entries ADD CONSTRAINT prey_wildcard_entry_values CHECK (
    balance_before BETWEEN 0 AND 52 AND balance_after BETWEEN 0 AND 52
    AND (previous_claim_id IS NOT NULL OR balance_before=0)
    AND (
        (kind='DAILY_REWARD' AND requested IN (1,2,3) AND credited BETWEEN 0 AND requested
         AND balance_after=balance_before+credited)
        OR (kind='SPEND' AND requested BETWEEN 1 AND 15 AND credited=0 AND balance_after=balance_before-requested)
        OR (kind='STORE' AND requested IN (5,20) AND credited=requested AND balance_after=balance_before+credited
            AND balance_after<=50)
    )
);

-- Preserve the DAILY and SPEND receipt checks while accounting for reserved Store capacity.
DO $$
DECLARE
    body TEXT;
    anchor TEXT := '    SELECT l.* INTO STRICT e';
BEGIN
    SELECT pg_get_functiondef('game_daily_wildcard_guard()'::regprocedure) INTO STRICT body;
    IF strpos(body,anchor)=0 THEN
        RAISE EXCEPTION 'missing Prey wildcard projection guard';
    END IF;
    EXECUTE replace(body,anchor,$guard$
    IF TG_TABLE_NAME='game_account_prey_wildcard_entries' THEN
        IF NEW.kind='STORE' AND NOT EXISTS (
            SELECT 1 FROM public.game_store_service_purchases p
            JOIN public.store_delivery_receipts d USING (operation_id)
            JOIN public.store_purchase_requests r ON r.purchase_request_id=p.purchase_id
            WHERE p.operation_id=NEW.claim_id AND p.account_id=NEW.account_id
              AND p.world_id=NEW.world_id AND p.kind='PREY_WILDCARDS' AND p.amount=NEW.requested
              AND d.account_id=p.account_id AND d.world_id=p.world_id
              AND d.character_id=r.character_id AND r.account_id=p.account_id AND r.state=2
              AND d.xmin=pg_current_xact_id()::xid
        ) THEN
            RAISE EXCEPTION 'Prey Store credit requires its delivery receipt' USING ERRCODE='23514';
        END IF;
        IF NEW.kind='DAILY_REWARD' AND NEW.credited<>least(NEW.requested,
            coalesce((SELECT 50-p.amount FROM public.game_store_service_purchases p
                WHERE p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.kind='PREY_WILDCARDS' AND p.operation_id IS NULL),52)
            -NEW.balance_before) THEN
            RAISE EXCEPTION 'Prey daily credit must account for reserved Store capacity' USING ERRCODE='23514';
        END IF;
    END IF;
    SELECT l.* INTO STRICT e$guard$);
END $$;

-- The older timed grant table remains readable for its existing grants. Paid
-- boosts count down only on creature XP settlements, using the Game clock.
CREATE TABLE game_store_hunting_xp_boosts (
    operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    remaining_minutes SMALLINT NOT NULL CHECK (remaining_minutes BETWEEN 0 AND 60),
    next_minute_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX game_store_hunting_xp_history
    ON game_store_hunting_xp_boosts(character_id,world_id);
CREATE UNIQUE INDEX game_store_hunting_xp_active
    ON game_store_hunting_xp_boosts(character_id) WHERE remaining_minutes>0;
REVOKE ALL ON game_store_hunting_xp_boosts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE(remaining_minutes,next_minute_at) ON game_store_hunting_xp_boosts TO oteryn_game_runtime;
GRANT SELECT ON game_store_hunting_xp_boosts TO oteryn_game_control;

-- Keep the death/NPC blessing set as the first charge. Only paid extras need
-- their own counter, so old rows and all set-based readers retain their meaning.
CREATE TABLE game_character_blessing_extra_charges (
    character_id UUID NOT NULL,
    blessing_key TEXT NOT NULL,
    remaining SMALLINT NOT NULL CHECK (remaining BETWEEN 1 AND 4),
    grant_operation_id UUID NOT NULL REFERENCES store_delivery_receipts(operation_id),
    PRIMARY KEY(character_id,blessing_key),
    FOREIGN KEY(character_id,blessing_key)
        REFERENCES game_character_blessings(character_id,blessing_key)
);
CREATE INDEX game_blessing_extra_grant_operation
    ON game_character_blessing_extra_charges(grant_operation_id);
REVOKE ALL ON game_character_blessing_extra_charges FROM PUBLIC;
GRANT SELECT, INSERT, DELETE, UPDATE(remaining,grant_operation_id)
    ON game_character_blessing_extra_charges TO oteryn_game_runtime;
GRANT SELECT ON game_character_blessing_extra_charges TO oteryn_game_control;

CREATE FUNCTION game_store_blessing_base_reservation_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.game_store_service_purchases p
        WHERE p.character_id=NEW.character_id AND p.kind='BLESSINGS' AND p.amount=5
          AND p.operation_id IS NULL AND NEW.blessing_key=ANY(p.blessing_keys)) THEN
        RAISE EXCEPTION 'Store reserved the remaining blessing capacity' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_blessing_base_reservation_guard() FROM PUBLIC;
CREATE TRIGGER game_store_blessing_base_reservation_guard
    BEFORE INSERT ON game_character_blessings FOR EACH ROW
    EXECUTE FUNCTION game_store_blessing_base_reservation_guard();

-- Preserve the original NPC evidence requirement. A Store grant can introduce
-- a base charge only with its own settled reservation in this transaction.
DO $$ DECLARE body TEXT; BEGIN
    SELECT pg_get_functiondef('game_character_blessing_purchase_guard()'::regprocedure) INTO STRICT body;
    IF strpos(body,'    IF NOT EXISTS')=0 THEN RAISE EXCEPTION 'missing NPC blessing grant guard'; END IF;
    EXECUTE replace(body,'    IF NOT EXISTS',$guard$
    IF EXISTS (SELECT 1 FROM public.game_store_service_purchases p
        WHERE p.kind='BLESSINGS' AND p.character_id=NEW.character_id
          AND NEW.blessing_key=ANY(p.blessing_keys)
          AND p.operation_id::text=NEW.provenance AND p.xmin=pg_current_xact_id()::xid) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS$guard$);
END $$;

ALTER TABLE game_character_death_receipts
    ADD COLUMN stacked_blessings_consumed TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[]
    CHECK (game_character_is_blessing_set(stacked_blessings_consumed)
        AND stacked_blessings_consumed <@ blessings_before
        AND stacked_blessings_consumed <@ blessings_after);

CREATE FUNCTION game_store_blessing_extra_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE old_remaining SMALLINT := 0;
BEGIN
    IF TG_OP<>'INSERT' THEN old_remaining := OLD.remaining; END IF;
    IF TG_OP<>'DELETE' AND NEW.remaining>old_remaining AND EXISTS (
        SELECT 1 FROM public.game_store_service_purchases p
        JOIN public.game_character_blessings b USING(character_id)
        WHERE p.xmin=pg_current_xact_id()::xid AND p.kind='BLESSINGS'
          AND p.operation_id=NEW.grant_operation_id AND p.character_id=NEW.character_id
          AND b.blessing_key=NEW.blessing_key AND b.blessing_key=ANY(p.blessing_keys)
          AND NEW.remaining-old_remaining=p.amount-(b.provenance=p.operation_id::text)::integer
    ) THEN
        RETURN NULL;
    END IF;
    IF TG_OP<>'INSERT' AND (
        (TG_OP='DELETE' AND OLD.remaining=1) OR
        (TG_OP='UPDATE' AND NEW.remaining=OLD.remaining-1 AND NEW.grant_operation_id=OLD.grant_operation_id)
    ) AND EXISTS (
        SELECT 1 FROM public.game_character_death_receipts d
        WHERE d.character_id=OLD.character_id AND d.created_xact_id=pg_current_xact_id()
          AND OLD.blessing_key=ANY(d.stacked_blessings_consumed)
    ) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'extra blessing requires its Store grant or death consumption' USING ERRCODE='23514';
END $$;
REVOKE ALL ON FUNCTION game_store_blessing_extra_guard() FROM PUBLIC;
CREATE TRIGGER game_store_blessing_extra_guard
    AFTER INSERT OR UPDATE OR DELETE ON game_character_blessing_extra_charges
    FOR EACH ROW EXECUTE FUNCTION game_store_blessing_extra_guard();

ALTER TABLE store_delivery_receipts DROP CONSTRAINT store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK(
 (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
 OR (item_instance_id IS NULL AND entitlement_key='prey-list-expansion' AND purchase_request_id IS NULL)
 OR (item_instance_id IS NULL AND entitlement_key='charm-expansion' AND purchase_request_id IS NOT NULL)
 OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
 OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-cosmetic:%' AND purchase_request_id IS NOT NULL)
 OR (item_instance_id IS NULL AND entitlement_key='store-service' AND purchase_request_id IS NULL));

CREATE FUNCTION game_store_service_receipt_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.game_store_service_purchases p
        WHERE p.operation_id=NEW.operation_id AND p.xmin=pg_current_xact_id()::xid
          AND p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id
    ) THEN
        RAISE EXCEPTION 'Store service receipt missing its settled reservation' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_service_receipt_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_store_service_receipt_guard
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN(NEW.entitlement_key='store-service') EXECUTE FUNCTION game_store_service_receipt_guard();

CREATE FUNCTION game_store_service_settlement_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.store_delivery_receipts d
        JOIN public.store_receipt_outbox o USING(operation_id)
        JOIN public.store_purchase_requests p ON p.purchase_request_id=NEW.purchase_id
        WHERE d.operation_id=NEW.operation_id AND d.xmin=pg_current_xact_id()::xid
          AND d.item_instance_id IS NULL AND d.entitlement_key='store-service' AND d.account_id=NEW.account_id
          AND d.world_id=NEW.world_id AND d.character_id=NEW.character_id
          AND p.account_id=d.account_id AND p.character_id=d.character_id AND p.state=2
          AND convert_from(p.answer,'UTF8')::jsonb->>'result'='ACCEPTED'
          AND convert_from(p.answer,'UTF8')::jsonb->'operation_ids'=jsonb_build_array(d.operation_id::text)
    ) OR (NEW.kind='XP_BOOST' AND NOT EXISTS (
        SELECT 1 FROM public.game_store_hunting_xp_boosts b
        WHERE b.operation_id=NEW.operation_id AND b.character_id=NEW.character_id
          AND b.world_id=NEW.world_id AND b.remaining_minutes=60 AND b.xmin=pg_current_xact_id()::xid
    )) OR (NEW.kind='PREY_WILDCARDS' AND NOT EXISTS (
        SELECT 1 FROM public.game_account_prey_wildcard_entries e
        WHERE e.claim_id=NEW.operation_id AND e.account_id=NEW.account_id AND e.world_id=NEW.world_id
          AND e.kind='STORE' AND e.credited=NEW.amount AND e.xmin=pg_current_xact_id()::xid
    )) OR (NEW.kind='WEEKLY_TASK_EXPANSION' AND NOT EXISTS (
        SELECT 1 FROM public.game_task_board_expansion_deliveries d
        WHERE d.delivery_reference='store-service:'||NEW.operation_id::text
          AND d.character_id=NEW.character_id AND d.world_id=NEW.world_id
          AND d.created_xact_id=pg_current_xact_id()
    )) OR (NEW.kind='BLESSINGS' AND EXISTS (
        SELECT 1 FROM unnest(NEW.blessing_keys) k(key) WHERE NOT EXISTS (
            SELECT 1 FROM public.game_character_blessings b
            LEFT JOIN public.game_character_blessing_extra_charges e USING(character_id,blessing_key)
            WHERE b.character_id=NEW.character_id AND b.blessing_key=k.key
              AND ((NEW.amount=1 AND b.provenance=NEW.operation_id::text AND b.xmin=pg_current_xact_id()::xid)
                 OR (e.grant_operation_id=NEW.operation_id AND e.xmin=pg_current_xact_id()::xid))
        )
    )) THEN
        RAISE EXCEPTION 'Store service receipt requires its atomic benefit and outbox' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_service_settlement_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_store_service_settlement_guard
    AFTER INSERT OR UPDATE ON game_store_service_purchases DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN(NEW.operation_id IS NOT NULL) EXECUTE FUNCTION game_store_service_settlement_guard();

CREATE FUNCTION game_store_service_purchase_transition() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF OLD.operation_id IS NOT NULL THEN
        RAISE EXCEPTION 'committed Store service is immutable' USING ERRCODE='23514';
    END IF;
    IF TG_OP='DELETE' THEN
        IF NOT EXISTS (SELECT 1 FROM public.store_purchase_requests r
            WHERE r.purchase_request_id=OLD.purchase_id AND r.state=2
              AND convert_from(r.answer,'UTF8')::jsonb->>'result' IN (
                  'STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED')
              AND convert_from(r.answer,'UTF8')::jsonb->'operation_ids'='[]'::jsonb
              AND convert_from(r.answer,'UTF8')::jsonb->'order_id'='null'::jsonb) THEN
            RAISE EXCEPTION 'Store service release requires terminal refusal' USING ERRCODE='23514';
        END IF;
        RETURN OLD;
    END IF;
    IF NEW.operation_id IS NULL OR (to_jsonb(NEW)-'operation_id') IS DISTINCT FROM (to_jsonb(OLD)-'operation_id') THEN
        RAISE EXCEPTION 'Store service reservation facts are immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_service_purchase_transition() FROM PUBLIC;
CREATE TRIGGER game_store_service_purchase_transition
    BEFORE UPDATE OR DELETE ON game_store_service_purchases FOR EACH ROW
    EXECUTE FUNCTION game_store_service_purchase_transition();
CREATE TRIGGER game_store_service_purchase_no_truncate
    BEFORE TRUNCATE ON game_store_service_purchases FOR EACH STATEMENT
    EXECUTE FUNCTION game_daily_reward_immutable();

CREATE FUNCTION game_store_weekly_reservation_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.game_store_service_purchases p
        WHERE p.character_id=NEW.character_id AND p.kind='WEEKLY_TASK_EXPANSION'
          AND p.operation_id IS NULL) THEN
        RAISE EXCEPTION 'Task Board expansion reserved by a pending Store purchase' USING ERRCODE='23514';
    END IF;
    IF NEW.delivery_reference LIKE 'store-service:%' AND NOT EXISTS (
        SELECT 1 FROM public.game_store_service_purchases p
        WHERE p.character_id=NEW.character_id AND p.world_id=NEW.world_id
          AND p.kind='WEEKLY_TASK_EXPANSION' AND p.xmin=pg_current_xact_id()::xid
          AND NEW.delivery_reference='store-service:'||p.operation_id::text
    ) THEN
        RAISE EXCEPTION 'Task Board expansion needs its Store settlement' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_weekly_reservation_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_store_weekly_reservation_guard
    AFTER INSERT ON game_task_board_expansion_deliveries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_store_weekly_reservation_guard();

CREATE TABLE game_store_reward_access_uses (
    purchase_id UUID PRIMARY KEY REFERENCES game_store_service_purchases(purchase_id),
    claim_id UUID NOT NULL UNIQUE REFERENCES game_character_daily_reward_claims(transaction_id)
);
REVOKE ALL ON game_store_reward_access_uses FROM PUBLIC;
GRANT SELECT, INSERT ON game_store_reward_access_uses TO oteryn_game_runtime;
GRANT SELECT ON game_store_reward_access_uses TO oteryn_game_control;

CREATE FUNCTION game_store_reward_access_use_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.game_store_service_purchases p
        JOIN public.game_character_daily_reward_claims c ON c.transaction_id=NEW.claim_id
        WHERE p.purchase_id=NEW.purchase_id AND p.operation_id IS NOT NULL AND p.kind='INSTANT_REWARD_ACCESS'
          AND p.character_id=c.character_id AND p.world_id=c.world_id AND c.xmin=pg_current_xact_id()::xid
    ) THEN
        RAISE EXCEPTION 'Instant Reward Access needs its same-transaction daily claim' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_reward_access_use_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_store_reward_access_use_guard
    AFTER INSERT ON game_store_reward_access_uses DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_store_reward_access_use_guard();
CREATE TRIGGER game_store_reward_access_use_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON game_store_reward_access_uses FOR EACH STATEMENT
    EXECUTE FUNCTION game_daily_reward_immutable();
