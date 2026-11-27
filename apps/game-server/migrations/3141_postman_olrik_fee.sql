-- Olrik's fixed wager uses the existing carried-coin and bank fee proof.
DO $migration$
DECLARE definition TEXT;
BEGIN
    SELECT regexp_replace(pg_get_constraintdef(oid), ' NOT VALID$', '') INTO STRICT definition
      FROM pg_constraint
     WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
    IF position('oteryn:quest-npc/postman/olrik/roll' IN definition)=0 THEN
        ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
        EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
            || substr(definition,7) || $arm$) OR (
                cause_kind=18 AND charm_key IS NULL AND fee_gold_units=5
                AND purchase_service_key='oteryn:quest-npc/postman/olrik/roll'
            )) NOT VALID$arm$;
    END IF;
END
$migration$;

CREATE OR REPLACE FUNCTION game_postman_olrik_roll_paid() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_quest_npc_program_receipts n
         JOIN game_quest_reward_fees f ON f.character_id=n.character_id
              AND f.cause_id=n.game_session_id AND f.cause_ordinal=n.command_id
              AND f.transition_key=n.transition_key
         WHERE n.character_id=NEW.character_id AND n.game_session_id=NEW.cause_id
           AND n.command_id=NEW.cause_ordinal AND n.transition_key=NEW.transition_key
           AND n.inventory_changed AND f.fee_gold_units=5 AND f.include_bank
           AND f.committed_character_revision=NEW.committed_character_revision
           AND f.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'Olrik roll requires its exact same-transaction NPC response and fee'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_postman_olrik_roll_paid() SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_postman_olrik_roll_paid() FROM PUBLIC;

DROP TRIGGER IF EXISTS game_postman_olrik_roll_paid ON game_character_quest_receipts;
CREATE CONSTRAINT TRIGGER game_postman_olrik_roll_paid
AFTER INSERT ON game_character_quest_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW WHEN (NEW.transition_key='oteryn:quest-npc/postman/olrik/roll')
EXECUTE FUNCTION game_postman_olrik_roll_paid();
