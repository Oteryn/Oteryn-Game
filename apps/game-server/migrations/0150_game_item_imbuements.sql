-- ITEM-IMBUE-1, IMBUE-FORGE0 §4.1: the single durable imbuement state owner.
-- This is storage preparation only. Apply/Clear require the still-unmerged
-- item-only fee owner; checkpoint writes require admitted elapsed provenance.
-- No runtime mutation grant or new audit cause is activated by this migration.
-- A follow-up must install the native audited writers and their receipt guards
-- before granting writes. In particular, neither a zero row nor an old audit
-- event is sufficient authority to delete a row in this preparation.
-- Rollback is additive: revoke access and retain rows, never discard item state.

CREATE TABLE game_item_imbuements (
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 0 AND 2),
    -- Category, not family: elemental damage families share one category.
    category SMALLINT NOT NULL CHECK (category BETWEEN 0 AND 19),
    imbuement_key TEXT NOT NULL CHECK (octet_length(imbuement_key) BETWEEN 1 AND 512),
    imbuement_revision TEXT NOT NULL
        CHECK (octet_length(imbuement_revision) BETWEEN 1 AND 512),
    tier SMALLINT NOT NULL CHECK (tier BETWEEN 1 AND 3),
    remaining_ms BIGINT NOT NULL CHECK (remaining_ms BETWEEN 0 AND 72000000),
    applying_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(applying_occurrence_id)),
    applying_transaction_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(applying_transaction_id)),
    PRIMARY KEY (item_instance_id, slot),
    -- Zero-time retained rows still occupy a slot/category until audited expiry.
    UNIQUE (item_instance_id, category),
    UNIQUE (applying_occurrence_id),
    UNIQUE (applying_transaction_id)
);

CREATE FUNCTION game_item_imbuement_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'imbuement deletion requires an admitted audited clear or expiry writer'
            USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND (
        (NEW.item_instance_id, NEW.slot, NEW.category, NEW.imbuement_key,
         NEW.imbuement_revision, NEW.tier, NEW.applying_occurrence_id,
         NEW.applying_transaction_id)
        IS DISTINCT FROM
        (OLD.item_instance_id, OLD.slot, OLD.category, OLD.imbuement_key,
         OLD.imbuement_revision, OLD.tier, OLD.applying_occurrence_id,
         OLD.applying_transaction_id)
        OR NEW.remaining_ms > OLD.remaining_ms
    ) THEN
        RAISE EXCEPTION 'imbuement binding is immutable and remaining time only decreases'
            USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' AND NEW.remaining_ms <> 72000000 THEN
        RAISE EXCEPTION 'an imbuement starts with twenty hours of eligible time'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_item_imbuement_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_item_imbuements FOR EACH ROW
    EXECUTE FUNCTION game_item_imbuement_row_guard();
CREATE TRIGGER game_item_imbuements_no_truncate BEFORE TRUNCATE
    ON game_item_imbuements EXECUTE FUNCTION game_character_reject_truncate();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_item_imbuement_row_guard() SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON game_item_imbuements FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_imbuement_row_guard() FROM PUBLIC;
-- Read-only until the inseparable native writer / fee / audit bundle lands.
-- There are deliberately no partial INSERT/UPDATE column grants to activate.
GRANT SELECT ON game_item_imbuements TO oteryn_game_runtime, oteryn_game_control;
