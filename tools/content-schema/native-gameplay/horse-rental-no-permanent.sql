-- A temporary personal rental can never become a permanent Account unlock,
-- regardless of a forged source-kind/provenance or a historical receipt.
CREATE FUNCTION game_horse_rental_no_permanent_grant()RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.unlock_key IN('oteryn:rental.horse.22','oteryn:rental.horse.25','oteryn:rental.horse.26')THEN
  RAISE EXCEPTION 'temporary horse rentals are Character contracts, never Account appearance grants' USING ERRCODE='23514';
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER game_horse_rental_no_permanent_receipt BEFORE INSERT ON game_account_appearance_unlock_receipts
 FOR EACH ROW EXECUTE FUNCTION game_horse_rental_no_permanent_grant();
CREATE TRIGGER game_horse_rental_no_permanent_fact BEFORE INSERT ON game_account_appearance_unlocks
 FOR EACH ROW EXECUTE FUNCTION game_horse_rental_no_permanent_grant();
