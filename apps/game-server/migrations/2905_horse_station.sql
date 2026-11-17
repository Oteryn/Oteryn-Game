CREATE TABLE game_horse_stations (
    world_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(world_id)),
    loose_until_ms BIGINT CHECK (loose_until_ms >= 0),
    confined_at_ms BIGINT CHECK (confined_at_ms >= 0)
);

REVOKE ALL ON game_horse_stations FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_horse_stations TO oteryn_game_runtime;
GRANT SELECT ON game_horse_stations TO oteryn_game_control;

CREATE TABLE game_horse_rental_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    world_id UUID NOT NULL,
    occurrence_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(occurrence_id)),
    fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns(transaction_id),
    mount_id SMALLINT NOT NULL CHECK (mount_id IN (22,25,26)),
    rented_at_ms BIGINT NOT NULL CHECK (rented_at_ms > 0),
    expires_at_ms BIGINT NOT NULL CHECK (expires_at_ms-rented_at_ms=86400000),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_horse_rental_character_expiry
    ON game_horse_rental_receipts(character_id,expires_at_ms DESC);
REVOKE ALL ON game_horse_rental_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_horse_rental_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_horse_rental_receipts TO oteryn_game_control;

CREATE FUNCTION game_horse_rental_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_item_fee_burns f
        WHERE f.transaction_id=NEW.fee_transaction_id AND f.cause_kind=19
          AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id
          AND f.cause_occurrence_id=NEW.occurrence_id AND f.fee_gold_units=500
          AND f.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'Horse Station rental requires its atomic fee' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_horse_rental_receipt_consistent
    AFTER INSERT ON game_horse_rental_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_horse_rental_receipt_guard();
REVOKE ALL ON FUNCTION game_horse_rental_receipt_guard() FROM PUBLIC;

DO $migration$
DECLARE rule RECORD; body TEXT;
BEGIN
    FOR rule IN
        SELECT conname, regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') AS definition
        FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass
          AND conname IN ('game_item_fee_burns_cause_kind_check', 'game_item_fee_burns_purchase_cause',
                          'game_item_fee_burns_committed_character_revision_check')
    LOOP
        EXECUTE format('ALTER TABLE game_item_fee_burns DROP CONSTRAINT %I',rule.conname);
        body := CASE rule.conname
            WHEN 'game_item_fee_burns_cause_kind_check' THEN 'cause_kind=19'
            WHEN 'game_item_fee_burns_purchase_cause' THEN
                'cause_kind=19 AND charm_key IS NULL AND purchase_service_key=''oteryn:service.horse_station.rent'' AND fee_gold_units=500'
            ELSE 'cause_kind=19 AND committed_character_revision BETWEEN 1 AND 18446744073709551615'
        END;
        EXECUTE format('ALTER TABLE game_item_fee_burns ADD CONSTRAINT %I CHECK ((%s) OR (%s)) NOT VALID',
                       rule.conname,substr(rule.definition,7),body);
    END LOOP;
    SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN
        RAISE EXCEPTION 'Horse Station requires the item-only fee guard anchor';
    END IF;
    EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
        '(NEW.cause_kind=19 AND EXISTS(SELECT 1 FROM game_horse_rental_receipts r WHERE r.fee_transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
    EXECUTE format('ALTER FUNCTION game_horse_rental_receipt_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
