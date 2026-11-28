-- Game reserves a bounded effect before the Platform debit. Only an accepted
-- delivery receipt can consume that reservation and apply its effect.
CREATE TABLE game_store_hunting_pending (
    purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    offer_key TEXT NOT NULL,
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
CREATE UNIQUE INDEX game_store_one_pending_boost ON game_store_hunting_pending(character_id)
    WHERE reset_epoch IS NOT NULL;
CREATE UNIQUE INDEX game_store_one_pending_prey_slot ON game_store_hunting_pending(character_id)
    WHERE prey_slot IS NOT NULL;
CREATE UNIQUE INDEX game_store_one_pending_wildcard_pack ON game_store_hunting_pending(account_id, world_id)
    WHERE wildcard_count IS NOT NULL;
CREATE INDEX game_store_hunting_pending_character ON game_store_hunting_pending(character_id);
CREATE INDEX game_store_hunting_pending_account ON game_store_hunting_pending(account_id);
CREATE INDEX game_store_hunting_pending_reset ON game_store_hunting_pending(world_id, reset_epoch);

CREATE TABLE game_character_store_prey_slots (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    target SMALLINT NOT NULL CHECK (target IN (2,3)),
    operation_id UUID NOT NULL UNIQUE REFERENCES store_delivery_receipts(operation_id),
    PRIMARY KEY (character_id, target)
);

-- Preserve legacy timestamp grants. The Store writer exclusively creates
-- purchases in the hunting-time journal below.
CREATE TABLE game_character_store_hunting_boosts (
    operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    ordinal SMALLINT NOT NULL CHECK (ordinal BETWEEN 1 AND 5),
    started_at_ms BIGINT NOT NULL,
    remaining_ms BIGINT NOT NULL CHECK (remaining_ms BETWEEN 0 AND 3600000),
    last_hunt_ms BIGINT,
    last_hunt_session UUID,
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_history(world_id, reset_epoch),
    UNIQUE (character_id, world_id, reset_epoch, ordinal),
    CHECK (last_hunt_session IS NULL OR last_hunt_ms IS NOT NULL),
    CHECK (last_hunt_ms >= started_at_ms)
);
CREATE UNIQUE INDEX game_character_one_store_hunting_boost
    ON game_character_store_hunting_boosts(character_id, world_id) WHERE remaining_ms > 0;
CREATE INDEX game_character_store_hunting_boost_reset
    ON game_character_store_hunting_boosts(world_id, reset_epoch);

CREATE VIEW game_character_permanent_prey_slots AS
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

CREATE FUNCTION game_store_xp_boost_tier(p_character UUID, p_world UUID)
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

CREATE FUNCTION game_store_reserve_hunting_purchase(
    p_purchase UUID, p_account UUID, p_character UUID, p_world UUID,
    p_binding BYTEA, p_offer TEXT, p_premium BOOLEAN, p_allow_new BOOLEAN
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
    IF EXISTS (SELECT 1 FROM game_store_hunting_pending WHERE purchase_id = p_purchase
        AND account_id = p_account AND character_id = p_character AND world_id = p_world AND offer_key = p_offer)
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
        (purchase_id, account_id, character_id, world_id, offer_key, reset_epoch, prey_slot, wildcard_count)
    VALUES (p_purchase, p_account, p_character, p_world, p_offer, boost_epoch, slot, pack)
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION game_store_reserve_hunting_purchase(UUID, UUID, UUID, UUID, BYTEA, TEXT, BOOLEAN, BOOLEAN) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_reserve_hunting_purchase(UUID, UUID, UUID, UUID, BYTEA, TEXT, BOOLEAN, BOOLEAN)
    TO oteryn_game_runtime;

CREATE FUNCTION game_store_release_hunting_purchase(p_purchase UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    PERFORM 1 FROM game_character_account_guards g
    JOIN game_store_hunting_pending r USING (account_id)
    WHERE r.purchase_id = p_purchase FOR UPDATE OF g;
    DELETE FROM game_store_hunting_pending r USING store_purchase_requests p
    WHERE r.purchase_id = p_purchase AND p.purchase_request_id = r.purchase_id
      AND p.state = 2
      AND convert_from(p.answer, 'UTF8')::jsonb ->> 'result' IN
          ('STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED')
      AND convert_from(p.answer, 'UTF8')::jsonb -> 'order_id' = 'null'::jsonb
      AND convert_from(p.answer, 'UTF8')::jsonb -> 'operation_ids' = '[]'::jsonb;
END $$;
REVOKE ALL ON FUNCTION game_store_release_hunting_purchase(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_release_hunting_purchase(UUID) TO oteryn_game_runtime;

ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT prey_wildcard_entry_values;
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
    )
);

-- Preserve the existing ledger chain and take held Store units into account
-- when calculating daily headroom.
CREATE FUNCTION game_prey_wildcard_daily_credit(p_account UUID, p_world UUID, p_before SMALLINT, p_requested SMALLINT)
RETURNS SMALLINT LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT least(p_requested, greatest(0, 52 - p_before - coalesce((
        SELECT wildcard_count FROM game_store_hunting_pending
        WHERE account_id = p_account AND world_id = p_world AND wildcard_count IS NOT NULL
    ), 0)))::smallint;
$$;
REVOKE ALL ON FUNCTION game_prey_wildcard_daily_credit(UUID, UUID, SMALLINT, SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_prey_wildcard_daily_credit(UUID, UUID, SMALLINT, SMALLINT) TO oteryn_game_runtime;

CREATE FUNCTION game_prey_wildcard_store_cause() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.kind = 'DAILY_REWARD' AND NEW.credited <>
        game_prey_wildcard_daily_credit(NEW.account_id, NEW.world_id, NEW.balance_before, NEW.requested)
    THEN
        RAISE EXCEPTION 'invalid daily wildcard credit' USING ERRCODE = '23514';
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
CREATE TRIGGER game_prey_wildcard_store_cause BEFORE INSERT ON game_account_prey_wildcard_entries
    FOR EACH ROW EXECUTE FUNCTION game_prey_wildcard_store_cause();

-- Only this closed set of new receipts bypasses the Hireling proof.
ALTER TABLE store_delivery_receipts DROP CONSTRAINT store_delivery_payload_kind;
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
DROP TRIGGER store_hireling_delivery_proven ON store_delivery_receipts;
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
CREATE UNIQUE INDEX game_store_hunting_delivery_purchase ON store_delivery_receipts(purchase_request_id)
    WHERE entitlement_key ~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$';

CREATE FUNCTION game_store_hunting_delivery_proven() RETURNS trigger
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
CREATE CONSTRAINT TRIGGER game_store_hunting_delivery_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.entitlement_key ~ '^store:(xp-boost:[1-5]|permanent-prey-slot|prey-wildcards:[15])$')
    EXECUTE FUNCTION game_store_hunting_delivery_proven();

-- The immutable kill award is the exactly-once owner of this activity.
ALTER TABLE game_party_kill_xp_members
    ADD COLUMN store_hunt_at_ms BIGINT,
    ADD COLUMN store_hunt_session UUID,
    ADD CHECK ((store_hunt_at_ms IS NULL) = (store_hunt_session IS NULL)),
    ADD CHECK (store_hunt_at_ms >= 0);

CREATE FUNCTION game_store_hunting_tick() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    UPDATE game_character_store_hunting_boosts b
    SET remaining_ms = greatest(0, b.remaining_ms - CASE
            WHEN NEW.stamina_minutes >= 840 AND b.last_hunt_session = NEW.store_hunt_session
                AND NEW.store_hunt_at_ms - b.last_hunt_ms <= 60000
            THEN NEW.store_hunt_at_ms - b.last_hunt_ms ELSE 0 END),
        last_hunt_ms = NEW.store_hunt_at_ms,
        last_hunt_session = CASE WHEN NEW.stamina_minutes >= 840
            THEN NEW.store_hunt_session END
    FROM game_character_roots c
    WHERE b.character_id = NEW.character_id AND c.character_id = b.character_id
      AND b.world_id = c.world_id AND b.remaining_ms > 0 AND NEW.store_hunt_at_ms >= b.started_at_ms
      AND (b.last_hunt_ms IS NULL OR NEW.store_hunt_at_ms > b.last_hunt_ms);
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_hunting_tick() FROM PUBLIC;
CREATE TRIGGER game_store_hunting_tick AFTER INSERT ON game_party_kill_xp_members
    FOR EACH ROW WHEN (NEW.store_hunt_at_ms IS NOT NULL AND NEW.raw_share > 0)
    EXECUTE FUNCTION game_store_hunting_tick();
