-- PREM-1b (PREMIUM-DELIVERY-0 §3.1, §10.2 and §11 scope item 7): the durable
-- Premium semantic conflict and its security audit, written by
-- `durability::premium_fence`.
--
--   * `game_premium_account_conflict`: one row per account, keyed by the
--     account alone so a first-ever unsupported response needs no evidence
--     row. It keeps the first detected semantic failure (a same-revision
--     contradiction or lifecycle regression, or an unsupported response) and
--     denies Premium for good: no path clears it (§3.1 declared deferral);
--   * `game_premium_security_audit`: one append-only row per account, kind and
--     `authority_revision` (consumer contract §15). Bounded columns only: never
--     the payload or any credential.
-- Neither table is ever updated, deleted or truncated. No existing table
-- changes.

CREATE TABLE game_premium_account_conflict (
    account_id UUID PRIMARY KEY,
    -- 1 CONTRADICTION, 2 UNSUPPORTED
    kind SMALLINT NOT NULL CHECK (kind IN (1, 2)),
    authority_revision NUMERIC(20, 0) NOT NULL
        CHECK (authority_revision BETWEEN 0 AND 18446744073709551615),
    snapshot_schema TEXT NOT NULL CHECK (snapshot_schema ~ '^[A-Za-z0-9._:-]{1,128}$'),
    producer_profile TEXT NOT NULL CHECK (producer_profile ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_id TEXT NOT NULL CHECK (product_id ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_version BIGINT NOT NULL CHECK (product_version BETWEEN 0 AND 4294967295),
    recorded_at_ms BIGINT NOT NULL
        DEFAULT floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint
);

CREATE TABLE game_premium_security_audit (
    account_id UUID NOT NULL,
    -- 1 CONTRADICTION, 2 UNSUPPORTED
    kind SMALLINT NOT NULL CHECK (kind IN (1, 2)),
    authority_revision NUMERIC(20, 0) NOT NULL
        CHECK (authority_revision BETWEEN 0 AND 18446744073709551615),
    snapshot_schema TEXT NOT NULL CHECK (snapshot_schema ~ '^[A-Za-z0-9._:-]{1,128}$'),
    producer_profile TEXT NOT NULL CHECK (producer_profile ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_id TEXT NOT NULL CHECK (product_id ~ '^[A-Za-z0-9._:-]{1,128}$'),
    product_version BIGINT NOT NULL CHECK (product_version BETWEEN 0 AND 4294967295),
    recorded_at_ms BIGINT NOT NULL
        DEFAULT floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint,
    PRIMARY KEY (account_id, kind, authority_revision)
);

CREATE TRIGGER game_premium_account_conflict_immutable BEFORE UPDATE OR DELETE
    ON game_premium_account_conflict FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_security_audit_immutable BEFORE UPDATE OR DELETE
    ON game_premium_security_audit FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_account_conflict_no_truncate BEFORE TRUNCATE
    ON game_premium_account_conflict EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_premium_security_audit_no_truncate BEFORE TRUNCATE
    ON game_premium_security_audit EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_premium_account_conflict, game_premium_security_audit FROM PUBLIC;
-- Runtime: a semantic failure inserts the conflict (first detection kept) and
-- its audit row; it never updates or deletes either.
GRANT SELECT, INSERT ON game_premium_account_conflict, game_premium_security_audit
    TO oteryn_game_runtime;
GRANT SELECT ON game_premium_account_conflict, game_premium_security_audit
    TO oteryn_game_control;
