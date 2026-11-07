-- Pemaret's one fish, once-only trophy and source quest receipt commit together.
-- Existing craft and claim proof chains retain their complete validation.
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.marlin'' AND outputs=''[]''::jsonb AND before_item->>''definition_production_key''=''oteryn:item.tibia.i901''))',previous);
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_event_source_location_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.marlin'' AND before_item->>''definition_production_key''=''oteryn:item.tibia.i901'' AND jsonb_typeof(event_source_location)=''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',previous);
END $migration$;

CREATE FUNCTION game_marlin_values_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE q game_character_quest_receipts%ROWTYPE; fish game_item_craft_records%ROWTYPE;
        trophy game_reward_claim_mint_receipts%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_character_quest_receipts' THEN q:=NEW;
 ELSIF TG_TABLE_NAME='game_item_craft_records' THEN
  IF NEW.recipe<>'oteryn:npc-handoff.marlin' THEN RETURN NULL; END IF;
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_kind='command' AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id
   AND transition_key='oteryn:quest-npc/marlin_trophy_quest/pemaret/deliver';
 ELSE
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_kind='command' AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id
   AND transition_key='oteryn:quest-npc/marlin_trophy_quest/pemaret/deliver';
 END IF;
 IF q.quest_key IS DISTINCT FROM 'oteryn:quest.marlin_trophy_quest'
  OR q.cause_kind IS DISTINCT FROM 'command'
  OR q.transition_key IS DISTINCT FROM 'oteryn:quest-npc/marlin_trophy_quest/pemaret/deliver'
  OR q.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR q.track_keys IS DISTINCT FROM ARRAY['oteryn:quest-progress/quest/u7_8/marlin_trophy']::text[]
  OR (q.values_before IS DISTINCT FROM ARRAY[-1]::bigint[] AND q.values_before IS DISTINCT FROM ARRAY[0]::bigint[])
  OR q.values_after IS DISTINCT FROM ARRAY[1]::bigint[]
 THEN RAISE EXCEPTION 'Marlin exchange requires its exact atomic NPC receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO fish FROM game_item_craft_records WHERE character_id=q.character_id
  AND game_session_id=q.cause_id AND command_id=q.cause_ordinal;
 IF fish.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR fish.recipe IS DISTINCT FROM 'oteryn:npc-handoff.marlin' OR NOT fish.consume_source
  OR fish.before_item->>'definition_family' IS DISTINCT FROM 'Item'
  OR fish.before_item->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i901'
  OR fish.outputs IS DISTINCT FROM '[]'::jsonb
 THEN RAISE EXCEPTION 'Marlin exchange requires one carried fish debit' USING ERRCODE='23514'; END IF;
 PERFORM game_item_craft_complete(fish.transaction_id);
 SELECT * INTO trophy FROM game_reward_claim_mint_receipts WHERE character_id=q.character_id
  AND game_session_id=q.cause_id AND command_id=q.cause_ordinal;
 IF trophy.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR trophy.claim_family IS DISTINCT FROM 'RewardClaim'
  OR trophy.claim_production_key IS DISTINCT FROM 'oteryn:reward-claim.npc.pemaret.marlin_trophy'
  OR trophy.quantity IS DISTINCT FROM 1
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=trophy.item_instance_id
   AND i.world_id=fish.world_id AND i.definition_family='Item'
   AND i.definition_production_key='oteryn:item.tibia.i902' AND i.quantity=1 AND i.lifecycle=1
   AND i.minted_transaction_id=trophy.transaction_id)
 THEN RAISE EXCEPTION 'Marlin exchange requires its fresh once-only trophy' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_marlin_npc_values AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.quest_key='oteryn:quest.marlin_trophy_quest')
 EXECUTE FUNCTION game_marlin_values_proven();
CREATE CONSTRAINT TRIGGER game_marlin_fish_values AFTER INSERT ON game_item_craft_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.recipe='oteryn:npc-handoff.marlin')
 EXECUTE FUNCTION game_marlin_values_proven();
CREATE CONSTRAINT TRIGGER game_marlin_trophy_values AFTER INSERT ON game_reward_claim_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.claim_family='RewardClaim'
 AND NEW.claim_production_key='oteryn:reward-claim.npc.pemaret.marlin_trophy')
 EXECUTE FUNCTION game_marlin_values_proven();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_marlin_values_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_marlin_values_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_marlin_values_proven() TO oteryn_game_runtime;
