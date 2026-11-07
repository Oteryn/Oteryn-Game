-- PREM-1a (PREMIUM-DELIVERY-0 §6; PROD-ENTITLEMENTS-01 consumer contract §6):
-- the durable Premium consumer fence, written by `durability::premium_fence`.
--
--   * `game_premium_evidence`: one immutable row per accepted snapshot,
--     (account, authority_revision), with a SHA-256 fingerprint of its
--     semantic fields. It is the equivocation horizon (§6.2): a later snapshot
--     that repeats an accepted revision with other content is detected even
--     below the high water. Never deleted;
--   * `game_premium_account_fence`: one row per account, the
--     `authority_revision` high water (its latest accepted evidence) and a
--     sticky conflict marker;
--   * `game_premium_entitlement_fence`: one row per (account, entitlement),
--     the `lifecycle_revision` high water, the fingerprint of its lifecycle
--     facts and its latest accepted evidence.
-- Guards reject any update that lowers a high water, clears a conflict or
-- changes a key, and every delete or truncate. No existing table changes.

CREATE TABLE game_premium_evidence (
    account_id UUID NOT NULL,
    authority_revision NUMERIC(20, 0) NOT NULL
        CHECK (authority_revision BETWEEN 0 AND 18446744073709551615),
    fingerprint BYTEA NOT NULL CHECK (octet_length(fingerprint) = 32),
    producer_revision TEXT NOT NULL CHECK (producer_revision ~ '^[A-Za-z0-9._:-]{1,128}$'),
    producer_profile TEXT NOT NULL CHECK (producer_profile ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_id TEXT NOT NULL CHECK (product_id ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_version BIGINT NOT NULL CHECK (product_version BETWEEN 0 AND 4294967295),
    entitlement_id TEXT CHECK (entitlement_id ~ '^[A-Za-z0-9._:-]{1,128}$'),
    -- 1 ACTIVE, 2 NOT_YET_EFFECTIVE, 3 EXPIRED, 4 REVOKED, 5 NONE
    entitlement_state SMALLINT NOT NULL CHECK (entitlement_state BETWEEN 1 AND 5),
    lifecycle_revision NUMERIC(20, 0) NOT NULL
        CHECK (lifecycle_revision BETWEEN 0 AND 18446744073709551615),
    effective_from_us BIGINT NOT NULL CHECK (effective_from_us >= 0),
    effective_until_us BIGINT NOT NULL CHECK (effective_until_us >= 0),
    authority_issued_at_us BIGINT NOT NULL CHECK (authority_issued_at_us >= 0),
    authority_valid_until_us BIGINT NOT NULL CHECK (authority_valid_until_us > authority_issued_at_us),
    refresh_after_us BIGINT NOT NULL CHECK (refresh_after_us >= 0),
    accepted_at_ms BIGINT NOT NULL
        DEFAULT floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint,
    PRIMARY KEY (account_id, authority_revision),
    CHECK ((entitlement_state = 5) = (entitlement_id IS NULL))
);

CREATE TABLE game_premium_account_fence (
    account_id UUID PRIMARY KEY,
    authority_revision NUMERIC(20, 0) NOT NULL,
    conflict_authority_revision NUMERIC(20, 0),
    FOREIGN KEY (account_id, authority_revision)
        REFERENCES game_premium_evidence (account_id, authority_revision)
);

CREATE TABLE game_premium_entitlement_fence (
    account_id UUID NOT NULL,
    entitlement_id TEXT NOT NULL,
    lifecycle_revision NUMERIC(20, 0) NOT NULL,
    lifecycle_fingerprint BYTEA NOT NULL CHECK (octet_length(lifecycle_fingerprint) = 32),
    authority_revision NUMERIC(20, 0) NOT NULL,
    PRIMARY KEY (account_id, entitlement_id),
    FOREIGN KEY (account_id, authority_revision)
        REFERENCES game_premium_evidence (account_id, authority_revision)
);

CREATE FUNCTION game_premium_account_fence_advance() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.account_id <> OLD.account_id
       OR NEW.authority_revision < OLD.authority_revision
       OR (OLD.conflict_authority_revision IS NOT NULL
           AND NEW.conflict_authority_revision IS DISTINCT FROM OLD.conflict_authority_revision) THEN
        RAISE EXCEPTION 'Premium account fence only advances' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_premium_entitlement_fence_advance() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.account_id <> OLD.account_id
       OR NEW.entitlement_id <> OLD.entitlement_id
       OR NEW.lifecycle_revision < OLD.lifecycle_revision
       OR (NEW.lifecycle_revision = OLD.lifecycle_revision
           AND NEW.lifecycle_fingerprint <> OLD.lifecycle_fingerprint)
       OR NEW.authority_revision <= OLD.authority_revision THEN
        RAISE EXCEPTION 'Premium entitlement fence only advances' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_premium_account_fence_advances BEFORE UPDATE
    ON game_premium_account_fence FOR EACH ROW
    EXECUTE FUNCTION game_premium_account_fence_advance();
CREATE TRIGGER game_premium_entitlement_fence_advances BEFORE UPDATE
    ON game_premium_entitlement_fence FOR EACH ROW
    EXECUTE FUNCTION game_premium_entitlement_fence_advance();
CREATE TRIGGER game_premium_evidence_immutable BEFORE UPDATE OR DELETE
    ON game_premium_evidence FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_account_fence_no_delete BEFORE DELETE
    ON game_premium_account_fence FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_entitlement_fence_no_delete BEFORE DELETE
    ON game_premium_entitlement_fence FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_evidence_no_truncate BEFORE TRUNCATE
    ON game_premium_evidence EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_premium_account_fence_no_truncate BEFORE TRUNCATE
    ON game_premium_account_fence EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_premium_entitlement_fence_no_truncate BEFORE TRUNCATE
    ON game_premium_entitlement_fence EXECUTE FUNCTION game_character_reject_truncate();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_premium_account_fence_advance() '
                   'SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_premium_entitlement_fence_advance() '
                   'SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_premium_evidence, game_premium_account_fence, game_premium_entitlement_fence
    FROM PUBLIC;
REVOKE ALL ON FUNCTION game_premium_account_fence_advance(),
    game_premium_entitlement_fence_advance() FROM PUBLIC;
-- Runtime: accepting a snapshot inserts evidence and inserts or advances the
-- fences; it never deletes.
GRANT SELECT, INSERT ON game_premium_evidence TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_premium_account_fence, game_premium_entitlement_fence
    TO oteryn_game_runtime;
GRANT SELECT ON game_premium_evidence, game_premium_account_fence, game_premium_entitlement_fence
    TO oteryn_game_control;
