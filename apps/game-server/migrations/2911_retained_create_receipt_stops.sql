-- Terminal publication dispositions are append-only and leave receipt bytes immutable.
CREATE TABLE game_character_retained_create_receipt_stops (
    operation_id UUID PRIMARY KEY REFERENCES game_character_command_receipts (operation_id)
);

REVOKE ALL ON game_character_retained_create_receipt_stops FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_retained_create_receipt_stops TO oteryn_game_runtime;
GRANT SELECT ON game_character_retained_create_receipt_stops TO oteryn_game_control;

-- The scan index contains only outstanding committed creates. Its insertion key
-- bounds a sweep even when Platform delivers a new operation with an older UUID.
CREATE TABLE game_character_retained_create_scan (
    scan_key BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    operation_id UUID NOT NULL UNIQUE REFERENCES game_character_command_receipts (operation_id)
);

REVOKE ALL ON game_character_retained_create_scan FROM PUBLIC;
GRANT SELECT ON game_character_retained_create_scan TO oteryn_game_runtime, oteryn_game_control;

-- Runtime can neither add nor remove scan entries directly.
CREATE FUNCTION game_character_retained_create_enqueue() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    INSERT INTO public.game_character_retained_create_scan (operation_id) VALUES (NEW.operation_id);
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_character_retained_create_dequeue() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    DELETE FROM public.game_character_retained_create_scan WHERE operation_id = NEW.operation_id;
    RETURN NEW;
END;
$$;

REVOKE ALL ON FUNCTION game_character_retained_create_enqueue() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_retained_create_dequeue() FROM PUBLIC;

CREATE TRIGGER game_character_retained_create_enqueue
    AFTER INSERT ON game_character_command_receipts FOR EACH ROW
    WHEN (NEW.command = 'CreateCharacter' AND NEW.outcome = 1)
    EXECUTE FUNCTION game_character_retained_create_enqueue();

CREATE TRIGGER game_character_retained_create_dequeue
    AFTER INSERT ON game_character_retained_create_receipt_stops FOR EACH ROW
    EXECUTE FUNCTION game_character_retained_create_dequeue();

-- Install the triggers before backfill so the migration holds the receipt
-- table lock throughout the copy and cannot miss a concurrent create.
INSERT INTO game_character_retained_create_scan (operation_id)
    SELECT operation_id FROM game_character_command_receipts
    WHERE command = 'CreateCharacter' AND outcome = 1
    ORDER BY operation_id;
