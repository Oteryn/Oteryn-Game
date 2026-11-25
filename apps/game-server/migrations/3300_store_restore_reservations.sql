-- Resolve the exact accepted operation while the restore conflict is recorded.
CREATE FUNCTION game_store_accepted_operation(answer BYTEA) RETURNS UUID
LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog
AS $$
DECLARE decision JSONB;
BEGIN
    decision := convert_from(answer,'UTF8')::jsonb;
    IF decision->>'result' = 'ACCEPTED'
       AND jsonb_typeof(decision->'operation_ids') = 'array'
       AND jsonb_array_length(decision->'operation_ids') = 1 THEN
        RETURN (decision->'operation_ids'->>0)::uuid;
    END IF;
    RETURN NULL;
EXCEPTION WHEN data_exception THEN
    -- Early journals validated byte length only. An unreadable answer cannot release a reservation.
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_accepted_operation(BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_accepted_operation(BYTEA) TO oteryn_game_runtime, oteryn_game_control;

CREATE INDEX store_purchase_accepted_operation
    ON store_purchase_requests(game_store_accepted_operation(answer)) WHERE state=2;

-- Extend the Prey release guard with the same accepted operation and permanent restore fence.
CREATE OR REPLACE FUNCTION game_prey_purchase_release_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    retained BYTEA;
    answer JSONB;
BEGIN
    SELECT r.answer INTO retained FROM store_purchase_requests r
    WHERE r.purchase_request_id=OLD.purchase_id AND r.character_id=OLD.character_id AND r.state=2;
    IF EXISTS (
        SELECT 1 FROM game_store_restore_conflicts conflict
        WHERE conflict.operation_id=game_store_accepted_operation(retained)
    ) THEN
        RETURN OLD;
    END IF;
    answer := convert_from(retained,'UTF8')::jsonb;
    IF answer IS NULL
       OR answer->'order_id' IS DISTINCT FROM 'null'::jsonb
       OR answer->'operation_ids' IS DISTINCT FROM '[]'::jsonb
       OR COALESCE(answer->>'result','') NOT IN (
           'STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED'
       ) THEN
        RAISE EXCEPTION 'Prey purchase reservation requires a definitive Store refusal or restore conflict'
            USING ERRCODE='23514';
    END IF;
    RETURN OLD;
END $$;

CREATE FUNCTION game_store_release_restored_reservations() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE purchases UUID[];
BEGIN
    IF NEW.delivery_operation_id IS NOT NULL
       OR EXISTS (SELECT 1 FROM store_delivery_receipts WHERE operation_id=NEW.operation_id) THEN
        RETURN NEW;
    END IF;
    SELECT array_agg(purchase_request_id) INTO purchases FROM (
        SELECT purchase_request_id FROM store_purchase_requests
        WHERE state=2 AND game_store_accepted_operation(answer)=NEW.operation_id
        LIMIT 2
    ) matches;
    IF cardinality(purchases) IS DISTINCT FROM 1 THEN
        RETURN NEW;
    END IF;

    DELETE FROM game_character_store_cosmetics WHERE purchase_id=purchases[1] AND operation_id IS NULL;
    DELETE FROM game_prey_expansion_purchases WHERE purchase_id=purchases[1];
    DELETE FROM game_charm_expansion_purchases WHERE purchase_id=purchases[1];
    UPDATE game_hireling_store_reservations SET state=3 WHERE purchase_request_id=purchases[1] AND state=1;
    RETURN NEW;
END $$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_prey_purchase_release_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_store_release_restored_reservations() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_store_release_restored_reservations() FROM PUBLIC;
CREATE TRIGGER game_store_restore_release_reservations
    AFTER INSERT ON game_store_restore_conflicts
    FOR EACH ROW EXECUTE FUNCTION game_store_release_restored_reservations();
