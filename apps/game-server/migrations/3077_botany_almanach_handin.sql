-- Rabaz accepts the intact book minted by his storage chest. Its original text
-- stays with the retired instance for the regular item-text cleanup owner.
CREATE FUNCTION game_quest_callback_accepts_authored_text(p_transition TEXT, p_item UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT $1 = 'oteryn:quest-reward/an-interest-in-botany/s4' AND EXISTS (
   SELECT 1 FROM game_item_instances i
   JOIN game_item_texts t USING (item_instance_id)
   JOIN game_reward_claim_mint_receipts r USING (item_instance_id)
   WHERE i.item_instance_id = $2 AND i.lifecycle = 1 AND i.quantity = 1
     AND i.definition_family = 'Item' AND i.definition_production_key = 'oteryn:item.tibia.i11699'
     AND r.claim_family = 'RewardClaim'
     AND r.claim_production_key = 'oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach'
     AND t.writer_character_id IS NULL AND t.text_revision = 1 AND t.text = r.root_text
 )
$$;

DO $migration$
DECLARE
 body TEXT := pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
 original CONSTANT TEXT := $guard$        OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)$guard$;
 replacement CONSTANT TEXT := $guard$        OR (EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
            AND NOT (NEW.quantity=1 AND NEW.replacement IS NULL
                     AND game_quest_callback_accepts_authored_text(NEW.transition_key,i.item_instance_id)))
        OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)$guard$;
BEGIN
 IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
  RAISE EXCEPTION 'unexpected carried quest text guard';
 END IF;
 EXECUTE replace(body,original,replacement);
 EXECUTE format('ALTER FUNCTION game_quest_callback_accepts_authored_text(TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_quest_callback_accepts_authored_text(TEXT,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_callback_accepts_authored_text(TEXT,UUID) TO oteryn_game_runtime;
