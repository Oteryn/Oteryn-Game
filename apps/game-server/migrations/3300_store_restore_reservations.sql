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
CREATE INDEX store_purchase_accepted_account
    ON store_purchase_requests(account_id,purchase_request_id)
    WHERE state=2 AND game_store_accepted_operation(answer) IS NOT NULL;

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

CREATE FUNCTION game_store_release_restored_reservation(operation UUID) RETURNS void
LANGUAGE plpgsql STRICT AS $$
DECLARE purchases UUID[];
BEGIN
    -- Serialize the two arrival orders. After waiting, the reads below see whichever
    -- side committed first, even when the outer statement began before that commit.
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:store-restore-operation:'||operation::text,0));
    IF NOT EXISTS (
        SELECT 1 FROM game_store_restore_conflicts
        WHERE operation_id=operation AND delivery_operation_id IS NULL
    ) OR EXISTS (SELECT 1 FROM store_delivery_receipts WHERE operation_id=operation) THEN
        RETURN;
    END IF;
    SELECT array_agg(purchase_request_id) INTO purchases FROM (
        SELECT purchase_request_id FROM store_purchase_requests
        WHERE state=2 AND game_store_accepted_operation(answer)=operation
        LIMIT 2
    ) matches;
    IF cardinality(purchases) IS DISTINCT FROM 1 THEN
        RETURN;
    END IF;

    DELETE FROM game_character_store_cosmetics WHERE purchase_id=purchases[1] AND operation_id IS NULL;
    DELETE FROM game_prey_expansion_purchases WHERE purchase_id=purchases[1];
    DELETE FROM game_charm_expansion_purchases WHERE purchase_id=purchases[1];
    UPDATE game_hireling_store_reservations SET state=3 WHERE purchase_request_id=purchases[1] AND state=1;
END $$;

CREATE FUNCTION game_store_release_restored_reservations() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_RELID='game_store_restore_conflicts'::regclass THEN
        PERFORM game_store_release_restored_reservation(NEW.operation_id);
    ELSE
        PERFORM game_store_release_restored_reservation(game_store_accepted_operation(NEW.answer));
    END IF;
    RETURN NEW;
END $$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_prey_purchase_release_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_store_release_restored_reservation(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_store_release_restored_reservations() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_store_release_restored_reservation(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_store_release_restored_reservations() FROM PUBLIC;
CREATE TRIGGER game_store_restore_release_reservations
    AFTER INSERT ON game_store_restore_conflicts
    FOR EACH ROW WHEN (NEW.delivery_operation_id IS NULL)
    EXECUTE FUNCTION game_store_release_restored_reservations();
CREATE TRIGGER game_store_answer_release_reservations
    AFTER UPDATE OF state ON store_purchase_requests
    FOR EACH ROW WHEN (OLD.state=1 AND NEW.state=2)
    EXECUTE FUNCTION game_store_release_restored_reservations();

DO $$ BEGIN
    PERFORM game_store_release_restored_reservation(conflict.operation_id)
    FROM game_store_restore_conflicts conflict
    WHERE conflict.delivery_operation_id IS NULL;
END $$;
