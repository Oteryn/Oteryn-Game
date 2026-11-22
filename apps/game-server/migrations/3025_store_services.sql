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
    CHECK ((kind='PREY_WILDCARDS' AND amount IN (5,20) AND reset_epoch IS NULL)
        OR (kind='XP_BOOST' AND amount BETWEEN 1 AND 5 AND reset_epoch IS NOT NULL)),
    operation_id UUID UNIQUE REFERENCES store_delivery_receipts(operation_id)
);
CREATE INDEX game_store_service_purchases_account_world
    ON game_store_service_purchases(account_id,world_id);
CREATE INDEX game_store_service_purchase_character ON game_store_service_purchases(character_id);
CREATE UNIQUE INDEX game_store_prey_purchase_pending
    ON game_store_service_purchases(account_id,world_id) WHERE operation_id IS NULL AND kind='PREY_WILDCARDS';
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

CREATE FUNCTION game_store_service_settlement_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.store_delivery_receipts d
        JOIN public.store_receipt_outbox o USING(operation_id)
        JOIN public.store_purchase_requests p ON p.purchase_request_id=NEW.purchase_id
        WHERE d.operation_id=NEW.operation_id AND d.xmin=pg_current_xact_id()::xid
          AND d.item_instance_id IS NULL AND d.account_id=NEW.account_id
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
    )) THEN
        RAISE EXCEPTION 'Store service receipt requires its atomic benefit and outbox' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_service_settlement_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_store_service_settlement_guard
    AFTER INSERT OR UPDATE ON game_store_service_purchases DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN(NEW.operation_id IS NOT NULL) EXECUTE FUNCTION game_store_service_settlement_guard();
