-- Local candidate: join D151 build receipts to the actual spell COMMIT.
-- Historical rows deliberately retain UNKNOWN physical provenance.
ALTER TABLE game_character_build_receipts ADD COLUMN created_xact_id xid8;
ALTER TABLE game_character_build_receipts ALTER COLUMN created_xact_id
    SET DEFAULT pg_current_xact_id();
CREATE FUNCTION game_spell_training_actual_transaction_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
    IF NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id() THEN
        RAISE EXCEPTION 'build receipt must originate in its actual transaction';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_spell_training_actual_transaction_guard
BEFORE INSERT ON game_character_build_receipts FOR EACH ROW
EXECUTE FUNCTION game_spell_training_actual_transaction_guard();
