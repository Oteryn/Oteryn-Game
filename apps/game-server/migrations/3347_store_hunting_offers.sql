-- Game reserves a bounded effect before the Platform debit. Only an accepted
-- delivery receipt can consume that reservation and apply its effect.
CREATE TABLE IF NOT EXISTS game_store_hunting_pending (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    offer_key TEXT NOT NULL,
    catalog_revision TEXT NOT NULL,
    price BIGINT NOT NULL CHECK (price > 0),
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    sent BOOLEAN NOT NULL DEFAULT FALSE,
    recovery_generation NUMERIC(20,0) NOT NULL,
    operation_id UUID UNIQUE,
    daily_deficit SMALLINT NOT NULL DEFAULT 0 CHECK (daily_deficit BETWEEN 0 AND 5),
    reset_epoch NUMERIC(20,0),
    prey_slot SMALLINT,
    wildcard_count SMALLINT,
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_history(world_id, reset_epoch),
    CHECK (
        (offer_key ~ '^store:xp-boost:[1-5]$' AND reset_epoch IS NOT NULL AND prey_slot IS NULL AND wildcard_count IS NULL)
        OR (offer_key = 'store:permanent-prey-slot' AND reset_epoch IS NULL AND prey_slot IS NOT NULL
            AND prey_slot IN (2,3) AND wildcard_count IS NULL)
        OR (reset_epoch IS NULL AND prey_slot IS NULL AND wildcard_count IS NOT NULL
            AND wildcard_count IN (1,5) AND offer_key = 'store:prey-wildcards:' || wildcard_count)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS game_store_one_pending_boost ON game_store_hunting_pending(character_id)
    WHERE reset_epoch IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS game_store_one_pending_prey_slot ON game_store_hunting_pending(character_id)
    WHERE prey_slot IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS game_store_one_pending_wildcard_pack ON game_store_hunting_pending(account_id, world_id)
    WHERE wildcard_count IS NOT NULL;
CREATE INDEX IF NOT EXISTS game_store_hunting_pending_character ON game_store_hunting_pending(character_id);
CREATE INDEX IF NOT EXISTS game_store_hunting_pending_account ON game_store_hunting_pending(account_id);
CREATE INDEX IF NOT EXISTS game_store_hunting_pending_reset ON game_store_hunting_pending(world_id, reset_epoch);
CREATE INDEX IF NOT EXISTS game_store_hunting_sent_queue ON game_store_hunting_pending(recovery_generation, purchase_id)
    WHERE sent AND operation_id IS NULL;

CREATE TABLE IF NOT EXISTS game_character_store_prey_slots (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    target SMALLINT NOT NULL CHECK (target IN (2,3)),
    operation_id UUID NOT NULL UNIQUE REFERENCES store_delivery_receipts(operation_id),
    PRIMARY KEY (character_id, target)
);

-- Preserve legacy timestamp grants. The Store writer exclusively creates
-- purchases in the hunting-time journal below.
CREATE TABLE IF NOT EXISTS game_character_store_hunting_boosts (
    operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    ordinal SMALLINT NOT NULL CHECK (ordinal BETWEEN 1 AND 5),
    started_at_ms BIGINT NOT NULL,
    remaining_ms BIGINT NOT NULL CHECK (remaining_ms BETWEEN 0 AND 3600000),
    last_hunt_ms BIGINT,
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_history(world_id, reset_epoch),
    UNIQUE (character_id, world_id, reset_epoch, ordinal),
    CHECK (last_hunt_ms >= started_at_ms)
);
CREATE UNIQUE INDEX IF NOT EXISTS game_character_one_store_hunting_boost
    ON game_character_store_hunting_boosts(character_id, world_id) WHERE remaining_ms > 0;
CREATE INDEX IF NOT EXISTS game_character_store_hunting_boost_reset
    ON game_character_store_hunting_boosts(world_id, reset_epoch);

CREATE OR REPLACE VIEW game_character_permanent_prey_slots AS
    SELECT character_id, target FROM game_character_store_prey_slots
    UNION
    SELECT c.character_id, u.target
    FROM game_character_roots c
    JOIN game_account_prey_unlocks u USING (account_id, world_id)
    JOIN game_character_roots claim ON claim.character_id = u.claiming_character_id
        AND claim.account_id = u.account_id AND claim.world_id = u.world_id;

REVOKE ALL ON game_store_hunting_pending, game_character_store_prey_slots,
    game_character_store_hunting_boosts, game_character_permanent_prey_slots FROM PUBLIC;
GRANT SELECT ON game_store_hunting_pending TO oteryn_game_runtime;
GRANT SELECT ON game_character_store_prey_slots, game_character_store_hunting_boosts,
    game_character_permanent_prey_slots TO oteryn_game_runtime;
GRANT SELECT ON game_store_hunting_pending, game_character_store_prey_slots,
    game_character_store_hunting_boosts, game_character_permanent_prey_slots TO oteryn_game_control;

CREATE OR REPLACE FUNCTION game_store_xp_boost_tier(p_character UUID, p_world UUID)
RETURNS TABLE (epoch NUMERIC, tier SMALLINT)
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    WITH instant AS MATERIALIZED (SELECT game_daily_effect_now() AS at)
    SELECT reset.reset_epoch, (1 + coalesce((
        SELECT max(ordinal) FROM game_character_store_hunting_boosts
        WHERE character_id = p_character AND world_id = p_world AND reset_epoch = reset.reset_epoch
    ), 0))::smallint
    FROM (SELECT reset_epoch FROM game_world_reset_history
        WHERE world_id = p_world ORDER BY reset_epoch DESC LIMIT 1) reset
    WHERE NOT EXISTS (SELECT 1 FROM game_character_store_hunting_boosts
        WHERE character_id = p_character AND world_id = p_world AND remaining_ms > 0)
      AND NOT EXISTS (SELECT 1 FROM game_character_store_xp_boosts, instant
        WHERE character_id = p_character AND world_id = p_world
          AND starts_at <= instant.at AND expires_at > instant.at);
$$;
REVOKE ALL ON FUNCTION game_store_xp_boost_tier(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_xp_boost_tier(UUID, UUID) TO oteryn_game_runtime;

-- Close an unsent checkout before reading or using the capacity it reserved.
CREATE OR REPLACE FUNCTION game_store_expire_hunting_purchases(p_account UUID, p_world UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    PERFORM 1 FROM game_character_account_guards WHERE account_id = p_account FOR UPDATE;
    UPDATE store_purchase_requests p
    SET state = 2,
        answer = convert_to('{"result":"OFFER_UNAVAILABLE","order_id":null,"operation_ids":[]}', 'UTF8')
    WHERE p.state = 1 AND p.purchase_request_id IN (
        SELECT r.purchase_id FROM game_store_hunting_pending r
        WHERE r.account_id = p_account AND r.world_id = p_world AND NOT r.sent
          AND r.reserved_at <= clock_timestamp() - interval '30 seconds'
        FOR UPDATE
    );
END $$;
REVOKE ALL ON FUNCTION game_store_expire_hunting_purchases(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_expire_hunting_purchases(UUID, UUID) TO oteryn_game_runtime;

CREATE TABLE IF NOT EXISTS game_store_hunting_retired (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id)
);
REVOKE ALL ON game_store_hunting_retired FROM PUBLIC;
GRANT SELECT ON game_store_hunting_retired TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_store_reserve_hunting_purchase(
    p_purchase UUID, p_account UUID, p_character UUID, p_world UUID,
    p_binding BYTEA, p_offer TEXT, p_premium BOOLEAN, p_allow_new BOOLEAN,
    p_revision TEXT, p_price BIGINT, p_generation NUMERIC
) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    boost_epoch NUMERIC;
    boost_tier SMALLINT;
    pack SMALLINT;
    slot SMALLINT;
BEGIN
    PERFORM 1 FROM game_character_account_guards WHERE account_id = p_account FOR UPDATE;
    IF NOT EXISTS (
        SELECT 1 FROM store_purchase_requests p JOIN game_character_roots c USING (character_id)
        WHERE p.purchase_request_id = p_purchase AND p.binding_digest = p_binding
          AND p.account_id = p_account AND p.character_id = p_character
          AND c.account_id = p_account AND c.world_id = p_world
    ) THEN
        RETURN FALSE;
    END IF;
    IF EXISTS (SELECT 1 FROM game_store_hunting_retired WHERE purchase_id = p_purchase) THEN
        RETURN NULL;
    END IF;
    PERFORM game_store_expire_hunting_purchases(p_account, p_world);

    IF EXISTS (SELECT 1 FROM game_store_hunting_pending WHERE purchase_id = p_purchase
        AND account_id = p_account AND character_id = p_character AND world_id = p_world AND offer_key = p_offer
        AND catalog_revision = p_revision AND price = p_price)
    THEN
        RETURN TRUE;
    END IF;
    IF NOT p_allow_new OR NOT EXISTS (SELECT 1 FROM store_purchase_requests
        WHERE purchase_request_id = p_purchase AND state = 1) THEN RETURN FALSE; END IF;

    IF p_offer ~ '^store:xp-boost:[1-5]$' THEN
        SELECT epoch, tier INTO boost_epoch, boost_tier FROM game_store_xp_boost_tier(p_character, p_world);
        IF boost_tier IS DISTINCT FROM right(p_offer, 1)::smallint THEN RETURN FALSE; END IF;
    ELSIF p_offer = 'store:permanent-prey-slot' THEN
        SELECT candidate INTO slot FROM (VALUES (2::smallint), (3::smallint)) AS slots(candidate)
        WHERE (candidate = 3 OR NOT p_premium) AND NOT EXISTS (
            SELECT 1 FROM game_character_permanent_prey_slots
            WHERE character_id = p_character AND target = candidate
        ) ORDER BY candidate LIMIT 1;
        IF slot IS NULL THEN RETURN FALSE; END IF;
    ELSIF p_offer IN ('store:prey-wildcards:1', 'store:prey-wildcards:5') THEN
        pack := right(p_offer, 1)::smallint;
        IF pack + coalesce((SELECT balance FROM game_account_prey_wildcards
            WHERE account_id = p_account AND world_id = p_world), 0) > 50 THEN RETURN FALSE; END IF;
    ELSE
        RETURN FALSE;
    END IF;
    INSERT INTO game_store_hunting_pending
        (purchase_id, account_id, character_id, world_id, offer_key, reset_epoch, prey_slot, wildcard_count, catalog_revision, price, recovery_generation)
    VALUES (p_purchase, p_account, p_character, p_world, p_offer, boost_epoch, slot, pack, p_revision, p_price, p_generation)
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION game_store_reserve_hunting_purchase(UUID, UUID, UUID, UUID, BYTEA, TEXT, BOOLEAN, BOOLEAN, TEXT, BIGINT, NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_reserve_hunting_purchase(UUID, UUID, UUID, UUID, BYTEA, TEXT, BOOLEAN, BOOLEAN, TEXT, BIGINT, NUMERIC)
    TO oteryn_game_runtime;

-- An unsent reservation may expire. Once marked sent, only authoritative
-- Platform evidence can release it or bind the resulting operation.
CREATE OR REPLACE FUNCTION game_store_mark_hunting_sent(p_purchase UUID, p_generation NUMERIC) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE buyer UUID;
BEGIN
    SELECT account_id INTO buyer FROM game_store_hunting_pending WHERE purchase_id = p_purchase;
    PERFORM 1 FROM game_character_account_guards WHERE account_id = buyer FOR UPDATE;
    UPDATE game_store_hunting_pending r SET sent = TRUE
    FROM store_purchase_requests p
    WHERE r.purchase_id = p_purchase AND p.purchase_request_id = r.purchase_id
      AND r.recovery_generation = p_generation AND p.state = 1
      AND (r.sent OR r.reserved_at > clock_timestamp() - interval '30 seconds');
    RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION game_store_mark_hunting_sent(UUID, NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_mark_hunting_sent(UUID, NUMERIC) TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_store_hunting_purchase_decided() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE operation UUID;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_store_hunting_pending WHERE purchase_id = NEW.purchase_request_id) THEN
        RETURN NEW;
    END IF;
    PERFORM 1 FROM game_character_account_guards WHERE account_id = NEW.account_id FOR UPDATE;
    operation := game_store_accepted_operation(NEW.answer);
    IF operation IS NOT NULL THEN
        -- Serialize acceptance against insertion of the restore conflict.
        PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:store-restore-operation:' || operation::text,0));
        IF EXISTS (SELECT 1 FROM game_store_restore_conflicts
            WHERE operation_id = operation AND delivery_operation_id IS NULL)
        THEN
            DELETE FROM game_store_hunting_pending WHERE purchase_id = NEW.purchase_request_id;
        ELSE
            UPDATE game_store_hunting_pending SET operation_id = operation
            WHERE purchase_id = NEW.purchase_request_id AND sent;
        END IF;
    ELSIF convert_from(NEW.answer, 'UTF8')::jsonb ->> 'result' IN
            ('STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED')
      AND convert_from(NEW.answer, 'UTF8')::jsonb -> 'order_id' = 'null'::jsonb
      AND convert_from(NEW.answer, 'UTF8')::jsonb -> 'operation_ids' = '[]'::jsonb
    THEN
        DELETE FROM game_store_hunting_pending WHERE purchase_id = NEW.purchase_request_id;
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_hunting_purchase_decided() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_store_hunting_purchase_decided ON store_purchase_requests;
CREATE TRIGGER game_store_hunting_purchase_decided AFTER UPDATE OF state ON store_purchase_requests
    FOR EACH ROW WHEN (OLD.state=1 AND NEW.state=2)
    EXECUTE FUNCTION game_store_hunting_purchase_decided();

CREATE OR REPLACE FUNCTION game_store_hunting_restore_conflict() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE buyer UUID;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:store-restore-operation:' || NEW.operation_id::text,0));
    SELECT account_id INTO buyer FROM game_store_hunting_pending WHERE operation_id = NEW.operation_id;
    IF buyer IS NOT NULL THEN
        PERFORM 1 FROM game_character_account_guards WHERE account_id = buyer FOR UPDATE;
        IF NOT EXISTS (SELECT 1 FROM store_delivery_receipts WHERE operation_id = NEW.operation_id) THEN
            DELETE FROM game_store_hunting_pending WHERE operation_id = NEW.operation_id;
        END IF;
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_hunting_restore_conflict() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_store_hunting_restore_conflict ON game_store_restore_conflicts;
CREATE TRIGGER game_store_hunting_restore_conflict AFTER INSERT ON game_store_restore_conflicts
    FOR EACH ROW WHEN (NEW.delivery_operation_id IS NULL)
    EXECUTE FUNCTION game_store_hunting_restore_conflict();

-- Recovery has fenced the previous generation before admitting its successor.
-- Preserve sent request identities so replay cannot initiate a new Platform debit.
CREATE OR REPLACE FUNCTION game_store_retire_hunting_purchases_on_recovery() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    WITH released AS (
        DELETE FROM game_store_hunting_pending
        WHERE recovery_generation < NEW.recovery_generation AND operation_id IS NULL
        RETURNING purchase_id, sent
    ), unsent AS (
        UPDATE store_purchase_requests p SET state = 2,
            answer = convert_to('{"result":"OFFER_UNAVAILABLE","order_id":null,"operation_ids":[]}', 'UTF8')
        FROM released WHERE p.purchase_request_id = released.purchase_id AND NOT released.sent AND p.state = 1
    )
    INSERT INTO game_store_hunting_retired (purchase_id)
    SELECT purchase_id FROM released WHERE sent;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_retire_hunting_purchases_on_recovery() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_store_retire_hunting_purchases_on_recovery ON game_character_recovery_admissions;
CREATE TRIGGER game_store_retire_hunting_purchases_on_recovery AFTER INSERT ON game_character_recovery_admissions
    FOR EACH ROW WHEN (NEW.predecessor_generation > 0)
    EXECUTE FUNCTION game_store_retire_hunting_purchases_on_recovery();

ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT IF EXISTS prey_wildcard_entry_values;
ALTER TABLE game_account_prey_wildcard_entries ADD CONSTRAINT prey_wildcard_entry_values CHECK (
    balance_before BETWEEN 0 AND 52 AND balance_after BETWEEN 0 AND 52
    AND (previous_claim_id IS NOT NULL OR balance_before = 0)
    AND (
        (kind = 'DAILY_REWARD' AND requested IN (1,2,3)
            AND credited BETWEEN 0 AND least(requested, 52 - balance_before)
            AND balance_after = balance_before + credited)
        OR (kind = 'SPEND' AND requested BETWEEN 1 AND 15 AND credited = 0
            AND balance_after = balance_before - requested)
        OR (kind = 'STORE' AND requested IN (1,5) AND credited = requested
            AND balance_after = balance_before + credited)
        OR (kind = 'STORE_HOLD_RELEASE' AND requested BETWEEN 1 AND 5
            AND credited = requested AND balance_after = balance_before + credited)
    )
);

-- Preserve the existing ledger chain and take held Store units into account
-- when calculating daily headroom.
CREATE OR REPLACE FUNCTION game_prey_wildcard_daily_credit(p_account UUID, p_world UUID, p_before SMALLINT, p_requested SMALLINT)
RETURNS SMALLINT LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT least(p_requested, greatest(0, 52 - p_before - coalesce((
        SELECT wildcard_count FROM game_store_hunting_pending
        WHERE account_id = p_account AND world_id = p_world AND wildcard_count IS NOT NULL
    ), 0)))::smallint;
$$;
REVOKE ALL ON FUNCTION game_prey_wildcard_daily_credit(UUID, UUID, SMALLINT, SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_prey_wildcard_daily_credit(UUID, UUID, SMALLINT, SMALLINT) TO oteryn_game_runtime;

CREATE TABLE IF NOT EXISTS game_store_hunting_wildcard_makeups (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    claim_id UUID NOT NULL UNIQUE DEFAULT game_character_uuid_v7(),
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    credit SMALLINT NOT NULL CHECK (credit BETWEEN 1 AND 5),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_store_hunting_wildcard_makeups FROM PUBLIC;

CREATE OR REPLACE FUNCTION game_store_restore_wildcard_credit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    previous game_account_prey_wildcards%ROWTYPE;
    makeup UUID;
BEGIN
    IF EXISTS (SELECT 1 FROM store_delivery_receipts WHERE purchase_request_id = OLD.purchase_id) THEN
        RETURN OLD;
    END IF;
    SELECT * INTO STRICT previous FROM game_account_prey_wildcards
    WHERE account_id = OLD.account_id AND world_id = OLD.world_id FOR UPDATE;
    INSERT INTO game_store_hunting_wildcard_makeups(purchase_id, account_id, world_id, credit)
    VALUES (OLD.purchase_id, OLD.account_id, OLD.world_id, OLD.daily_deficit) RETURNING claim_id INTO makeup;
    INSERT INTO game_account_prey_wildcard_entries
        (claim_id, account_id, world_id, previous_claim_id, kind, requested, credited, balance_before, balance_after)
    VALUES (makeup, OLD.account_id, OLD.world_id, previous.last_claim_id, 'STORE_HOLD_RELEASE',
        OLD.daily_deficit, OLD.daily_deficit, previous.balance, previous.balance + OLD.daily_deficit);
    UPDATE game_account_prey_wildcards
    SET balance = previous.balance + OLD.daily_deficit, last_claim_id = makeup
    WHERE account_id = OLD.account_id AND world_id = OLD.world_id;
    RETURN OLD;
END $$;
REVOKE ALL ON FUNCTION game_store_restore_wildcard_credit() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_store_restore_wildcard_credit ON game_store_hunting_pending;
CREATE TRIGGER game_store_restore_wildcard_credit AFTER DELETE ON game_store_hunting_pending
    FOR EACH ROW WHEN (OLD.daily_deficit > 0)
    EXECUTE FUNCTION game_store_restore_wildcard_credit();

CREATE OR REPLACE FUNCTION game_prey_wildcard_store_cause() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.kind = 'DAILY_REWARD' AND NEW.credited <>
        game_prey_wildcard_daily_credit(NEW.account_id, NEW.world_id, NEW.balance_before, NEW.requested)
    THEN
        RAISE EXCEPTION 'invalid daily wildcard credit' USING ERRCODE = '23514';
    END IF;
    IF NEW.kind = 'DAILY_REWARD' THEN
        -- Compare the actual credit with a shadow balance that never had a
        -- Store hold. A release makes up the difference. Spends reduce both
        -- balances equally, so the deficit is at most the reserved pack.
        UPDATE game_store_hunting_pending
        SET daily_deficit = least(52, NEW.balance_before + daily_deficit + NEW.requested)
            - NEW.balance_after
        WHERE account_id = NEW.account_id AND world_id = NEW.world_id AND wildcard_count IS NOT NULL;
    END IF;
    IF NEW.kind = 'STORE_HOLD_RELEASE' AND NOT EXISTS (
        SELECT 1 FROM game_store_hunting_wildcard_makeups m
        WHERE (m.claim_id, m.account_id, m.world_id, m.credit) =
            (NEW.claim_id, NEW.account_id, NEW.world_id, NEW.credited)
            AND m.created_xact_id = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'wildcard make-up credit requires a reservation release' USING ERRCODE = '23514';
    END IF;
    IF NEW.kind = 'STORE' AND NOT EXISTS (
        SELECT 1 FROM store_delivery_receipts r
        JOIN game_store_hunting_pending p ON p.purchase_id = r.purchase_request_id
        WHERE r.operation_id = NEW.claim_id AND r.account_id = NEW.account_id
          AND r.world_id = NEW.world_id AND p.wildcard_count = NEW.requested
          AND r.entitlement_key = p.offer_key AND r.xmin = pg_current_xact_id()::xid
    ) THEN
        RAISE EXCEPTION 'Store wildcards require a same-transaction delivery' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_prey_wildcard_store_cause() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_wildcard_store_cause ON game_account_prey_wildcard_entries;
CREATE TRIGGER game_prey_wildcard_store_cause BEFORE INSERT ON game_account_prey_wildcard_entries
    FOR EACH ROW EXECUTE FUNCTION game_prey_wildcard_store_cause();

-- Only this closed set of new receipts bypasses the Hireling proof.
ALTER TABLE store_delivery_receipts DROP CONSTRAINT IF EXISTS store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK (
    (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'prey-list-expansion' AND purchase_request_id IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'charm-expansion' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-cosmetic:%' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-blessing:%' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND purchase_request_id IS NOT NULL AND
        entitlement_key ~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$')
);
DROP TRIGGER IF EXISTS store_hireling_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_hireling_delivery_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (
        NEW.purchase_request_id IS NOT NULL
        AND NEW.entitlement_key IS DISTINCT FROM 'charm-expansion'
        AND (NEW.entitlement_key IS NULL OR (
            NEW.entitlement_key NOT LIKE 'store-cosmetic:%'
            AND NEW.entitlement_key NOT LIKE 'store-blessing:%'
            AND NEW.entitlement_key !~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$'
        ))
    ) EXECUTE FUNCTION game_store_hireling_delivery_proven();
CREATE UNIQUE INDEX IF NOT EXISTS game_store_hunting_delivery_purchase ON store_delivery_receipts(purchase_request_id)
    WHERE entitlement_key ~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$';

CREATE OR REPLACE FUNCTION game_store_hunting_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    pending game_store_hunting_pending%ROWTYPE;
    previous game_account_prey_wildcards%ROWTYPE;
    before_balance SMALLINT;
BEGIN
    PERFORM 1 FROM game_character_account_guards WHERE account_id = NEW.account_id FOR UPDATE;
    SELECT r.* INTO STRICT pending
    FROM game_store_hunting_pending r
    JOIN store_purchase_requests p ON r.purchase_id = p.purchase_request_id
    JOIN game_store_purchase_history h USING (purchase_request_id)
    JOIN game_character_roots c ON c.character_id = p.character_id
    JOIN store_receipt_outbox o ON o.operation_id = NEW.operation_id
    CROSS JOIN LATERAL (SELECT convert_from(p.answer, 'UTF8')::jsonb AS value) a
    WHERE r.purchase_id = NEW.purchase_request_id
      AND (r.account_id, r.character_id, r.world_id) = (NEW.account_id, NEW.character_id, NEW.world_id)
      AND p.state = 2 AND p.account_id = r.account_id AND p.character_id = r.character_id
      AND h.account_id = p.account_id AND h.character_id = p.character_id
      AND c.account_id = r.account_id AND c.world_id = r.world_id
      AND r.offer_key = h.offer_key AND h.offer_key = NEW.entitlement_key
      AND r.price = h.price AND r.catalog_revision = h.catalog_revision
      AND r.operation_id = NEW.operation_id
      AND NOT EXISTS (SELECT 1 FROM game_store_restore_conflicts WHERE operation_id = NEW.operation_id)
      AND a.value->>'result' = 'ACCEPTED' AND a.value->>'order_id' = h.order_id::text
      AND a.value->'operation_ids' = jsonb_build_array(NEW.operation_id::text)
    FOR UPDATE OF r;

    IF pending.reset_epoch IS NOT NULL THEN
        INSERT INTO game_character_store_hunting_boosts
            (operation_id, character_id, world_id, reset_epoch, ordinal, started_at_ms, remaining_ms)
        VALUES (NEW.operation_id, NEW.character_id, NEW.world_id, pending.reset_epoch,
            right(pending.offer_key, 1)::smallint,
            floor(extract(epoch FROM game_daily_effect_now()) * 1000)::bigint, 3600000);
    ELSIF pending.prey_slot IS NOT NULL THEN
        INSERT INTO game_character_store_prey_slots(character_id, target, operation_id)
        VALUES (NEW.character_id, pending.prey_slot, NEW.operation_id);
    ELSE
        SELECT * INTO previous FROM game_account_prey_wildcards
        WHERE account_id = NEW.account_id AND world_id = NEW.world_id FOR UPDATE;
        before_balance := coalesce(previous.balance, 0);
        INSERT INTO game_account_prey_wildcard_entries
            (claim_id, account_id, world_id, previous_claim_id, kind, requested, credited, balance_before, balance_after)
        VALUES (NEW.operation_id, NEW.account_id, NEW.world_id, previous.last_claim_id,
            'STORE', pending.wildcard_count, pending.wildcard_count,
            before_balance, before_balance + pending.wildcard_count);
        INSERT INTO game_account_prey_wildcards(account_id, world_id, balance, last_claim_id)
        VALUES (NEW.account_id, NEW.world_id, before_balance + pending.wildcard_count, NEW.operation_id)
        ON CONFLICT (account_id, world_id) DO UPDATE
        SET balance = excluded.balance, last_claim_id = excluded.last_claim_id;
    END IF;
    DELETE FROM game_store_hunting_pending WHERE purchase_id = pending.purchase_id;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_hunting_delivery_proven() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_store_hunting_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER game_store_hunting_delivery_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.entitlement_key ~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$')
    EXECUTE FUNCTION game_store_hunting_delivery_proven();

-- Debit every observed span, clipped to purchase time and the last settled kill.
-- Spans were collected at the stamina in force during combat, before kill XP.
CREATE OR REPLACE FUNCTION game_consume_store_hunting_time(
    p_death BYTEA, p_character UUID, p_at_ms BIGINT, p_starts BIGINT[], p_ends BIGINT[]
) RETURNS VOID
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
    UPDATE game_character_store_hunting_boosts b
    SET remaining_ms = greatest(0, b.remaining_ms - coalesce((
            SELECT sum(upper(span) - lower(span))::bigint
            FROM unnest((SELECT range_agg(int8range(start_ms, end_ms))
                FROM (SELECT greatest(start_ms, b.started_at_ms, coalesce(b.last_hunt_ms, b.started_at_ms)) AS start_ms,
                             least(end_ms, p_at_ms) AS end_ms
                      FROM unnest(p_starts, p_ends) AS seen(start_ms, end_ms)) clipped
                WHERE start_ms < end_ms)) AS span
        ), 0)),
        last_hunt_ms = p_at_ms
    FROM game_character_roots c
    WHERE b.character_id = p_character AND c.character_id = b.character_id
      AND b.world_id = c.world_id AND b.remaining_ms > 0 AND p_at_ms >= b.started_at_ms
      AND (b.last_hunt_ms IS NULL OR p_at_ms > b.last_hunt_ms)
      AND EXISTS (SELECT 1 FROM game_party_kill_xp_receipts
          WHERE death_key = p_death AND xmin = pg_current_xact_id()::xid);
$$;
REVOKE ALL ON FUNCTION game_consume_store_hunting_time(BYTEA, UUID, BIGINT, BIGINT[], BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_consume_store_hunting_time(BYTEA, UUID, BIGINT, BIGINT[], BIGINT[])
    TO oteryn_game_runtime;
