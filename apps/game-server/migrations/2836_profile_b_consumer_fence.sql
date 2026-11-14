-- Profile-B v1: one account authority sequence, independent of Premium's authority sequence.
CREATE TABLE game_profile_b_accounts (
    account_id UUID PRIMARY KEY CHECK (get_byte(uuid_send(account_id),6)>>4=7 AND get_byte(uuid_send(account_id),8)&192=128),
    authority_revision BIGINT NOT NULL DEFAULT 0 CHECK (authority_revision BETWEEN 0 AND 9007199254740991),
    answer_bytes BYTEA CHECK (octet_length(answer_bytes) BETWEEN 1 AND 8192)
);
CREATE TABLE game_profile_b_products (
    account_id UUID NOT NULL REFERENCES game_profile_b_accounts(account_id),
    product_id TEXT NOT NULL CHECK (octet_length(product_id)<=64 AND product_id ~ '^[a-z][a-z0-9_]{0,31}(\.[a-z][a-z0-9_]{0,31}){1,3}$'),
    product_version BIGINT NOT NULL CHECK (product_version BETWEEN 1 AND 9007199254740991),
    lifecycle_revision BIGINT NOT NULL DEFAULT 0 CHECK (lifecycle_revision BETWEEN 0 AND 9007199254740991),
    lifecycle_bytes BYTEA CHECK (octet_length(lifecycle_bytes) BETWEEN 1 AND 512),
    hint_revision BIGINT NOT NULL DEFAULT 0 CHECK (hint_revision BETWEEN 0 AND 9007199254740991),
    PRIMARY KEY(account_id,product_id,product_version)
);
CREATE TABLE game_profile_b_changes_cursor (
    singleton BOOLEAN PRIMARY KEY DEFAULT true CHECK (singleton),
    stream_cursor TEXT CHECK (stream_cursor IS NULL OR (stream_cursor ~ '^v1:[1-9][0-9]{0,18}$' AND substring(stream_cursor FROM 4)::numeric<=9223372036854775807)),
    stopped BOOLEAN NOT NULL DEFAULT false
);
INSERT INTO game_profile_b_changes_cursor(singleton) VALUES(true);
CREATE FUNCTION game_profile_b_monotonic() RETURNS trigger LANGUAGE plpgsql
SET search_path=pg_catalog,public AS $$ BEGIN
    IF TG_TABLE_NAME='game_profile_b_accounts' THEN
        IF NEW.authority_revision<OLD.authority_revision OR
            (NEW.authority_revision=OLD.authority_revision AND NEW.answer_bytes IS DISTINCT FROM OLD.answer_bytes) THEN
            RAISE EXCEPTION 'Profile-B authority fence regression';
        END IF;
    ELSE
        IF OLD.lifecycle_bytes IS NOT NULL AND (convert_from(OLD.lifecycle_bytes,'UTF8')::jsonb->>0) IS NOT NULL
            AND (convert_from(OLD.lifecycle_bytes,'UTF8')::jsonb->>0) IS DISTINCT FROM (convert_from(NEW.lifecycle_bytes,'UTF8')::jsonb->>0) THEN
            RAISE EXCEPTION 'Profile-B entitlement identity is immutable';
        END IF;
        IF NEW.lifecycle_revision<OLD.lifecycle_revision OR NEW.hint_revision<OLD.hint_revision OR
            (NEW.lifecycle_revision=OLD.lifecycle_revision AND OLD.lifecycle_bytes IS NOT NULL AND NEW.lifecycle_bytes IS DISTINCT FROM OLD.lifecycle_bytes) THEN
            RAISE EXCEPTION 'Profile-B lifecycle fence regression';
        END IF;
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_profile_b_monotonic() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_profile_b_monotonic() TO oteryn_game_runtime;
CREATE TRIGGER game_profile_b_accounts_monotonic BEFORE UPDATE ON game_profile_b_accounts
    FOR EACH ROW EXECUTE FUNCTION game_profile_b_monotonic();
CREATE TRIGGER game_profile_b_products_monotonic BEFORE UPDATE ON game_profile_b_products
    FOR EACH ROW EXECUTE FUNCTION game_profile_b_monotonic();
REVOKE ALL ON game_profile_b_accounts,game_profile_b_products,game_profile_b_changes_cursor FROM PUBLIC;
GRANT SELECT,INSERT ON game_profile_b_accounts,game_profile_b_products TO oteryn_game_runtime;
GRANT UPDATE(authority_revision,answer_bytes) ON game_profile_b_accounts TO oteryn_game_runtime;
GRANT UPDATE(lifecycle_revision,lifecycle_bytes,hint_revision) ON game_profile_b_products TO oteryn_game_runtime;
GRANT SELECT,UPDATE ON game_profile_b_changes_cursor TO oteryn_game_runtime;
GRANT SELECT ON game_profile_b_accounts,game_profile_b_products,game_profile_b_changes_cursor TO oteryn_game_control;
