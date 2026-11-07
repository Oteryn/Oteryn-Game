-- DRAFT finite source observations for AvarTar; no quest/chest reward is minted here.
-- The actual generic chest claim/MINT owner remains the only producer of value1.
CREATE FUNCTION game_npc_final_claim_projection(p_character UUID,p_track TEXT)
 RETURNS TABLE(source_value BIGINT,claim_transaction_id UUID) LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE claim_key TEXT; stored_ack BIGINT; actual_claim UUID;
BEGIN
 claim_key:=CASE p_track
  WHEN 'oteryn:quest-progress/quest/u7_24/the_annihilator/reward'
   THEN 'oteryn:reward-claim.quest.u7_24.the_annihilator.reward'
  WHEN 'oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet'
   THEN 'oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_helmet'
  ELSE NULL END;
 IF claim_key IS NULL THEN RETURN;END IF;
 SELECT claim.claimed_transaction_id INTO actual_claim FROM game_reward_claims claim
 JOIN game_reward_claim_mint_receipts minted ON minted.transaction_id=claim.claimed_transaction_id
  AND minted.character_id=claim.character_id AND minted.claim_family=claim.claim_family
  AND minted.claim_production_key=claim.claim_production_key AND minted.claim_revision_ref=claim.claim_revision_ref
 WHERE claim.character_id=p_character AND claim.claim_family='RewardClaim' AND claim.claim_production_key=claim_key
 ORDER BY claim.claimed_transaction_id LIMIT 1;
 SELECT value INTO stored_ack FROM game_character_quest_tracks WHERE character_id=p_character AND track_key=p_track;
 IF actual_claim IS NULL THEN
  IF stored_ack IS NOT NULL THEN
   RAISE EXCEPTION 'Final cosmetic acknowledgement lacks actual chest claim' USING ERRCODE='23514';END IF;
  source_value:=-1;claim_transaction_id:=NULL;RETURN NEXT;RETURN;
 END IF;
 IF stored_ack IS NOT NULL AND stored_ack<>2 THEN
 RAISE EXCEPTION 'Invalid final cosmetic acknowledgement value' USING ERRCODE='23514';END IF;
 source_value:=CASE WHEN stored_ack=2 THEN 2 ELSE 1 END;
 claim_transaction_id:=actual_claim;
 RETURN NEXT;
END $$;
REVOKE ALL ON FUNCTION game_npc_final_claim_projection(UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_final_claim_projection(UUID,TEXT) TO oteryn_game_runtime;

-- Before integration, source Snapshot and private arm must BOTH use this exact
-- projection for the two known bindings. A first acknowledgement stores native
-- component initial(-1)->2 while its source observation is actual claim1->2;
-- the typed Quest child must seal the actual claim_transaction_id and reprove
-- this projection. It is not an arbitrary source_before/owner_before exception.

DO $claim_projection_path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_final_claim_projection(uuid,text) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $claim_projection_path$;

-- The child admission arm calls this only for an expected, Node-sealed native
-- operation. It does not admit a source delta or child by itself.
CREATE FUNCTION game_npc_final_claim_projection_current(p_character UUID,p_source TEXT,
 p_track TEXT,p_claim UUID,p_source_before BIGINT,p_owner_before BIGINT)
 RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT p_claim IS NOT NULL AND p_source_before=1 AND p_owner_before=-1
 AND ((p_source='oteryn:npc-cosmetic/avar_tar/demon-base'
   AND p_track='oteryn:quest-progress/quest/u7_24/the_annihilator/reward')
  OR(p_source='oteryn:npc-cosmetic/avar_tar/demon-shield'
   AND p_track='oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet'))
 AND NOT EXISTS(SELECT 1 FROM game_character_quest_tracks actual_track
   WHERE actual_track.character_id=p_character AND actual_track.track_key=p_track)
 AND EXISTS(SELECT 1 FROM game_npc_final_claim_projection(p_character,p_track) actual_claim
   WHERE actual_claim.source_value=1 AND actual_claim.claim_transaction_id=p_claim)
$$;
REVOKE ALL ON FUNCTION game_npc_final_claim_projection_current(UUID,TEXT,TEXT,UUID,BIGINT,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_final_claim_projection_current(UUID,TEXT,TEXT,UUID,BIGINT,BIGINT) TO oteryn_game_runtime;
DO $claim_projection_current_path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_final_claim_projection_current(uuid,text,text,uuid,bigint,bigint) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $claim_projection_current_path$;
