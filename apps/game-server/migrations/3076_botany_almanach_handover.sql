-- Rabaz accepts the untouched Almanach minted by his native chest claim.
CREATE FUNCTION game_quest_botany_almanach_handover(
    p_character UUID, p_item UUID, p_transition TEXT
) RETURNS BOOLEAN LANGUAGE sql STABLE SET search_path=pg_catalog,public AS $$
    SELECT p_transition='oteryn:quest-reward/an-interest-in-botany/s4'
       AND EXISTS(
           SELECT 1 FROM game_item_instances i
           JOIN game_item_texts t USING (item_instance_id)
           JOIN game_reward_claim_mint_receipts r USING (item_instance_id)
           WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
             AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i11699'
             AND i.definition_revision_ref='definition-r1'
             AND t.writer_character_id IS NULL AND t.text_revision=1
             AND r.character_id=p_character AND r.transaction_id=i.minted_transaction_id
             AND r.claim_family='RewardClaim'
             AND r.claim_production_key='oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach'
             AND r.claim_revision_ref='reward-claim-r1'
       )
$$;
REVOKE ALL ON FUNCTION game_quest_botany_almanach_handover(UUID,UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_botany_almanach_handover(UUID,UUID,TEXT) TO oteryn_game_runtime;

DO $migration$
DECLARE
    body TEXT;
    needle TEXT := E'\n        OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)';
BEGIN
    body := pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'unexpected carried quest text guard';
    END IF;
    EXECUTE replace(body,needle,needle || $proof$
        AND NOT (NEW.quantity=1 AND NEW.replacement IS NULL
                 AND game_quest_botany_almanach_handover(
                     NEW.character_id,i.item_instance_id,NEW.transition_key))$proof$);
END $migration$;
