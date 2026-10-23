-- MARKET-1 bounded storage checkpoint, extending the existing MARKET0 readers.
-- MARKET-CONTENT-1 and DEPOT-WIRE-1 are absent: runtime receives SELECT only.
-- No offer/value writer, ledger or event is enabled. Atomic settlement must
-- later use BANK-1, FeeBurnCause::MarketFee and the existing 0076 Inbox owner;
-- 0151 remains the sole CharacterDepot owner. No alternative depot is created.

CREATE TABLE game_market_books (
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    definition_key TEXT NOT NULL CHECK (octet_length(definition_key) BETWEEN 1 AND 512),
    PRIMARY KEY (world_id, definition_key)
);
CREATE TABLE game_market_offers (
    offer_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(offer_id)),
    world_id UUID NOT NULL,
    definition_key TEXT NOT NULL,
    definition_revision TEXT NOT NULL CHECK (octet_length(definition_revision) BETWEEN 1 AND 512),
    side SMALLINT NOT NULL CHECK (side IN (1,2)),
    account_id UUID NOT NULL REFERENCES game_character_account_guards ON DELETE RESTRICT,
    character_id UUID NOT NULL REFERENCES game_character_roots ON DELETE RESTRICT,
    piece_price NUMERIC(20,0) NOT NULL CHECK (piece_price BETWEEN 1 AND 999999999999),
    amount INT NOT NULL CHECK (amount BETWEEN 1 AND 64000),
    remaining INT NOT NULL CHECK (remaining BETWEEN 0 AND amount),
    anonymous BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_us BIGINT NOT NULL CHECK (created_at_us BETWEEN 0 AND 9223369444854775807),
    expires_at_us BIGINT NOT NULL CHECK (expires_at_us = created_at_us + 2592000000000),
    state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5),
    ended_as SMALLINT CHECK (ended_as IN (2,3,4)),
    escrow_gold NUMERIC(20,0) NOT NULL CHECK (escrow_gold BETWEEN 0 AND 999999999999),
    held_credit_gold NUMERIC(20,0) NOT NULL CHECK (held_credit_gold BETWEEN 0 AND 18446744073709551615),
    matching_pending BOOLEAN NOT NULL DEFAULT FALSE,
    ended_at_us BIGINT,
    end_sequence BIGINT,
    UNIQUE (offer_id, world_id),
    CONSTRAINT game_market_end_sequence_unique UNIQUE (end_sequence),
    FOREIGN KEY (world_id, definition_key) REFERENCES game_market_books ON DELETE RESTRICT,
    CHECK (piece_price * amount <= 999999999999),
    CHECK ((state = 5) = (ended_as IS NOT NULL)),
    CHECK (escrow_gold = CASE WHEN side = 1 AND state = 1 THEN piece_price * remaining ELSE 0 END),
    CHECK ((state = 5 AND held_credit_gold > 0) OR (state <> 5 AND held_credit_gold = 0)),
    CHECK (state <> 1 OR remaining > 0),
    CHECK (state <> 2 OR remaining = 0),
    CHECK (ended_as <> 2 OR remaining = 0),
    CHECK (state = 1 OR NOT matching_pending),
    CONSTRAINT game_market_ending_metadata CHECK (
        (state = 1 AND ended_at_us IS NULL AND end_sequence IS NULL)
        OR (state <> 1 AND ended_at_us IS NOT NULL AND ended_at_us >= created_at_us
            AND end_sequence IS NOT NULL AND end_sequence > 0)
    )
);
CREATE SEQUENCE game_market_end_sequence AS BIGINT MINVALUE 1 NO CYCLE;

-- Match every paged reader, including owner reads and ending history.
CREATE INDEX market_own_open ON game_market_offers (world_id,character_id,created_at_us DESC,offer_id DESC) WHERE state = 1;
CREATE INDEX market_own_ended ON game_market_offers (world_id,character_id,ended_at_us DESC,end_sequence DESC) WHERE state IN (2,3,4,5);
CREATE INDEX market_pending ON game_market_offers (world_id,definition_key,created_at_us,offer_id) WHERE state = 1 AND matching_pending;
CREATE INDEX market_expiring ON game_market_offers (world_id,expires_at_us,offer_id) WHERE state = 1;
CREATE INDEX market_sell_price ON game_market_offers (world_id,definition_key,piece_price ASC,created_at_us,offer_id) WHERE state = 1 AND side = 2;
CREATE INDEX market_buy_price ON game_market_offers (world_id,definition_key,piece_price DESC,created_at_us,offer_id) WHERE state = 1 AND side = 1;

-- Structural escrow proposal used by the existing reader; no runtime insertion
-- or removal grant. This is not admitted custody: the DUR-03 union, transaction
-- evidence, item conservation and Inbox reservations must land with its writer.
CREATE TABLE game_item_market_escrow_locations (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    offer_id UUID NOT NULL,
    ordinal NUMERIC(20,0) NOT NULL CHECK (ordinal BETWEEN 1 AND 18446744073709551615),
    UNIQUE (offer_id, ordinal),
    FOREIGN KEY (item_instance_id,world_id) REFERENCES game_item_instances (item_instance_id,world_id) ON DELETE RESTRICT,
    FOREIGN KEY (offer_id,world_id) REFERENCES game_market_offers (offer_id,world_id) ON DELETE RESTRICT
);
CREATE INDEX market_escrow_item_order ON game_item_market_escrow_locations (offer_id,world_id,item_instance_id);

CREATE TABLE game_market_operations (
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    occurrence BYTEA PRIMARY KEY CHECK (octet_length(occurrence) BETWEEN 26 AND 27),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 6),
    actor UUID REFERENCES game_character_roots ON DELETE RESTRICT,
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    outcome BYTEA NOT NULL CHECK (octet_length(outcome) BETWEEN 1 AND 512),
    CHECK ((kind <= 3) = (actor IS NOT NULL))
);

-- Lock the actual Character root before admission/counting, serializing the
-- 100th slot across all of its ware books. Expired OPEN offers still count.
-- SECURITY INVOKER is deliberate: this helper conveys no runtime write grant.
CREATE FUNCTION game_market_offer_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_world UUID;
    v_account UUID;
    v_count BIGINT;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF (NEW.offer_id,NEW.world_id,NEW.definition_key,NEW.definition_revision,
            NEW.side,NEW.account_id,NEW.character_id,NEW.piece_price,NEW.amount,
            NEW.anonymous,NEW.created_at_us,NEW.expires_at_us)
           IS DISTINCT FROM
           (OLD.offer_id,OLD.world_id,OLD.definition_key,OLD.definition_revision,
            OLD.side,OLD.account_id,OLD.character_id,OLD.piece_price,OLD.amount,
            OLD.anonymous,OLD.created_at_us,OLD.expires_at_us) THEN
            RAISE EXCEPTION 'Market offer identity and terms are immutable' USING ERRCODE = '23514';
        END IF;
        IF OLD.state <> 1 AND NEW.state = 1 THEN
            RAISE EXCEPTION 'Market ended offer cannot reopen' USING ERRCODE = '23514';
        END IF;
    END IF;
    SELECT world_id,account_id INTO v_world,v_account FROM game_character_roots
     WHERE character_id = NEW.character_id FOR UPDATE;
    IF NOT FOUND OR v_world <> NEW.world_id OR v_account <> NEW.account_id THEN
        RAISE EXCEPTION 'Market Character Account World mismatch' USING ERRCODE = '23514';
    END IF;
    IF NEW.state = 1 AND TG_OP = 'INSERT' THEN
        SELECT count(*) INTO v_count FROM (
            SELECT 1 FROM game_market_offers WHERE character_id = NEW.character_id
             AND state = 1 LIMIT 100
        ) AS offers;
        IF v_count >= 100 THEN
            RAISE EXCEPTION 'Market open offer capacity exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_market_offer_guard BEFORE INSERT OR UPDATE ON game_market_offers
    FOR EACH ROW EXECUTE FUNCTION game_market_offer_guard();

CREATE FUNCTION game_market_operation_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'immutable Market occurrence' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_market_operation_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_market_operations FOR EACH STATEMENT EXECUTE FUNCTION game_market_operation_immutable();

REVOKE ALL ON game_market_books,game_market_offers,game_item_market_escrow_locations,
    game_market_operations,game_market_end_sequence FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_market_offer_guard(),game_market_operation_immutable() FROM PUBLIC;
GRANT SELECT ON game_market_books,game_market_offers,game_item_market_escrow_locations,
    game_market_operations TO oteryn_game_runtime,oteryn_game_control;
