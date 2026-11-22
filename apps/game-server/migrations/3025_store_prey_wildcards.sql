-- Reserve the complete pack before Platform can take coins. A settled reservation
-- remains linked to the receipt so an operation cannot serve a second purchase.
CREATE TABLE game_store_prey_purchases (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    quantity SMALLINT NOT NULL CHECK (quantity IN (5,20)),
    operation_id UUID UNIQUE REFERENCES store_delivery_receipts(operation_id)
);
CREATE INDEX game_store_prey_purchases_account_world
    ON game_store_prey_purchases(account_id,world_id);
CREATE UNIQUE INDEX game_store_prey_purchase_pending
    ON game_store_prey_purchases(account_id,world_id) WHERE operation_id IS NULL;
REVOKE ALL ON game_store_prey_purchases FROM PUBLIC;
GRANT SELECT, INSERT, DELETE, UPDATE(operation_id) ON game_store_prey_purchases TO oteryn_game_runtime;
GRANT SELECT ON game_store_prey_purchases TO oteryn_game_control;

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
            SELECT 1 FROM public.game_store_prey_purchases p
            JOIN public.store_delivery_receipts d USING (operation_id)
            JOIN public.store_purchase_requests r ON r.purchase_request_id=p.purchase_id
            WHERE p.operation_id=NEW.claim_id AND p.account_id=NEW.account_id
              AND p.world_id=NEW.world_id AND p.quantity=NEW.requested
              AND d.account_id=p.account_id AND d.world_id=p.world_id
              AND d.character_id=r.character_id AND r.account_id=p.account_id AND r.state=2
              AND d.xmin=pg_current_xact_id()::xid
        ) THEN
            RAISE EXCEPTION 'Prey Store credit requires its delivery receipt' USING ERRCODE='23514';
        END IF;
        IF NEW.kind='DAILY_REWARD' AND NEW.credited<>least(NEW.requested,
            coalesce((SELECT 50-p.quantity FROM public.game_store_prey_purchases p
                WHERE p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.operation_id IS NULL),52)
            -NEW.balance_before) THEN
            RAISE EXCEPTION 'Prey daily credit must account for reserved Store capacity' USING ERRCODE='23514';
        END IF;
    END IF;
    SELECT l.* INTO STRICT e$guard$);
END $$;
