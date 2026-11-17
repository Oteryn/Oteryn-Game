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
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision >= 1),
    committed_character_revision NUMERIC(20,0) NOT NULL CHECK (committed_character_revision=original_character_revision+1),
    level BIGINT NOT NULL,
    experience BIGINT NOT NULL,
    profile_revision TEXT NOT NULL,
    ruleset_revision TEXT NOT NULL,
    content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    reward_revision TEXT NOT NULL,
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
          AND f.committed_character_revision=NEW.committed_character_revision
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
CREATE CONSTRAINT TRIGGER game_horse_rental_revision_chain
    AFTER INSERT ON game_horse_rental_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();

DO $migration$
DECLARE rule RECORD; body TEXT; target TEXT;
BEGIN
    SELECT regexp_replace(pg_get_viewdef('game_character_premium_revision_chain'::regclass,true),
                          ';[[:space:]]*$','') INTO STRICT body;
    EXECUTE 'CREATE OR REPLACE VIEW game_character_premium_revision_chain WITH(security_invoker=true) AS '
        ||body||' UNION ALL SELECT character_id,original_character_revision,committed_character_revision,level,level,experience,experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_horse_rental_receipts';
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
    FOREACH target IN ARRAY ARRAY['game_atelier_coin_admit()', 'game_atelier_coin_complete(uuid)'] LOOP
        SELECT pg_get_functiondef(target::regprocedure) INTO STRICT body;
        IF position('IN (9,10,11,18)' IN body)=0 THEN
            RAISE EXCEPTION 'Horse Station requires the carried coin guard anchor in %',target;
        END IF;
        body := replace(body,'IN (9,10,11,18)','IN (9,10,11,18,19)');
        IF target='game_atelier_coin_complete(uuid)' THEN
            IF position('OR (f.cause_kind=18 AND EXISTS' IN body)=0 THEN
                RAISE EXCEPTION 'Horse Station requires the carried coin receipt anchor';
            END IF;
            body := replace(body,'OR (f.cause_kind=18 AND EXISTS',$proof$
                OR (f.cause_kind=19 AND EXISTS (
                    SELECT 1 FROM game_horse_rental_receipts h
                    WHERE h.fee_transaction_id=f.transaction_id AND h.character_id=f.character_id
                      AND h.world_id=f.world_id AND h.occurrence_id=f.cause_occurrence_id
                      AND h.created_xact_id=pg_current_xact_id() AND f.fee_gold_units=500
                )) OR (f.cause_kind=18 AND EXISTS$proof$);
        END IF;
        EXECUTE body;
    END LOOP;
    FOR rule IN SELECT conname,pg_get_constraintdef(oid) AS definition
        FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND contype='c'
    LOOP
        IF position('ARRAY[9, 10, 11, 18]' IN rule.definition)>0 THEN
            EXECUTE format('ALTER TABLE game_item_fee_burns DROP CONSTRAINT %I',rule.conname);
            EXECUTE format('ALTER TABLE game_item_fee_burns ADD CONSTRAINT %I %s',
                rule.conname,replace(rule.definition,'ARRAY[9, 10, 11, 18]','ARRAY[9, 10, 11, 18, 19]'));
        END IF;
    END LOOP;
    SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('IF NEW.cause_kind=18 THEN' IN body)=0 THEN
        RAISE EXCEPTION 'Horse Station requires the fee source guard anchor';
    END IF;
    EXECUTE replace(body,'IF NEW.cause_kind=18 THEN',$guard$
        IF NEW.cause_kind=19 THEN
            PERFORM game_atelier_coin_complete(NEW.transaction_id);
            RETURN NULL;
        END IF;
        IF NEW.cause_kind=18 THEN$guard$);
    SELECT pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure) INTO STRICT body;
    IF position('f.cause_kind IN (9,10,11)' IN body)=0 THEN
        RAISE EXCEPTION 'Horse Station requires the fee audit envelope guard anchor';
    END IF;
    EXECUTE replace(body,'f.cause_kind IN (9,10,11)','f.cause_kind IN (9,10,11,19)');
    EXECUTE format('ALTER FUNCTION game_horse_rental_receipt_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
