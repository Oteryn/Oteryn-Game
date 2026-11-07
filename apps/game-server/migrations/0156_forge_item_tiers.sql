-- ITEM-FORGE-1 / IMBUE-FORGE0 §8 storage preparation.
-- Extend the existing tier reader; tier zero is absence, never another owner.
-- Runtime remains read-only until Content recipes, ForgeCause/native receipt,
-- fee/item composition and session fencing land together. This storage audit
-- is not a forge occurrence receipt or permission for an economic mutation.

CREATE TABLE game_item_tiers (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
    tier SMALLINT NOT NULL,
    CONSTRAINT game_item_tier_range CHECK (tier BETWEEN 1 AND 10)
);

-- Immutable raw state transitions, written automatically in the same physical
-- transaction. No second balance, inventory, depot or inbox table is introduced.
-- Auditing raw values also retains evidence if a migration owner deliberately
-- bypasses the tier CHECK for corruption/recovery qualification.
CREATE TABLE game_item_tier_audit (
    change_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7()
        CHECK (game_character_is_uuid_v7(change_id)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    tier_before SMALLINT,
    tier_after SMALLINT,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_item_tier_audit_item ON game_item_tier_audit(item_instance_id, change_id);

CREATE FUNCTION game_item_tier_identity_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.item_instance_id <> OLD.item_instance_id THEN
        RAISE EXCEPTION 'item tier cannot change its ItemInstance identity' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_item_tier_audit_change() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_item_tier_audit(item_instance_id, tier_before, tier_after)
    VALUES (COALESCE(NEW.item_instance_id, OLD.item_instance_id),
            CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.tier END,
            CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.tier END);
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_item_tier_identity_immutable
    BEFORE UPDATE OF item_instance_id ON game_item_tiers
    FOR EACH ROW EXECUTE FUNCTION game_item_tier_identity_guard();
CREATE TRIGGER game_item_tier_audited
    AFTER INSERT OR UPDATE OR DELETE ON game_item_tiers
    FOR EACH ROW EXECUTE FUNCTION game_item_tier_audit_change();
CREATE TRIGGER game_item_tier_no_truncate BEFORE TRUNCATE
    ON game_item_tiers EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_tier_audit_immutable BEFORE UPDATE OR DELETE
    ON game_item_tier_audit FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_tier_audit_stamp BEFORE INSERT
    ON game_item_tier_audit FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_item_tier_audit_no_truncate BEFORE TRUNCATE
    ON game_item_tier_audit EXECUTE FUNCTION game_item_reject_truncate();

DO $$
DECLARE n TEXT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_tier_identity_guard', 'game_item_tier_audit_change'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
END;
$$;
REVOKE ALL ON game_item_tiers, game_item_tier_audit
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
-- No runtime-written columns in this foundation; do not grant bare tier writes.
GRANT SELECT ON game_item_tiers, game_item_tier_audit
    TO oteryn_game_runtime, oteryn_game_control;
