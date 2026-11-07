-- Tereban's eight material debits and final cape are bound to the existing
-- source NPC quest receipt and recoverable quest-XP obligation in one transaction.
-- Rollback disables these NPC branches; immutable earned receipts remain.
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.father-material'' AND jsonb_typeof(outputs)=''array'' AND jsonb_array_length(outputs)=0 AND before_item->>''definition_production_key'' IN (''oteryn:item.tibia.i11545'',''oteryn:item.tibia.i11546'',''oteryn:item.tibia.i11547'',''oteryn:item.tibia.i11548'',''oteryn:item.tibia.i11549'',''oteryn:item.tibia.i11550'',''oteryn:item.tibia.i11551'',''oteryn:item.tibia.i11552'')))',previous);
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_event_source_location_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.father-material'' AND before_item->>''definition_production_key'' IN (''oteryn:item.tibia.i11545'',''oteryn:item.tibia.i11546'',''oteryn:item.tibia.i11547'',''oteryn:item.tibia.i11548'',''oteryn:item.tibia.i11549'',''oteryn:item.tibia.i11550'',''oteryn:item.tibia.i11551'',''oteryn:item.tibia.i11552'') AND jsonb_typeof(event_source_location)=''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',previous);
END $migration$;

CREATE FUNCTION game_father_values_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE q game_character_quest_receipts%ROWTYPE; craft game_item_craft_records%ROWTYPE;
        cape game_reward_claim_mint_receipts%ROWTYPE; material TEXT; expected_track TEXT; expected_rule TEXT; required_xp BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_character_quest_receipts' THEN q:=NEW;
 ELSIF TG_TABLE_NAME='game_item_craft_records' THEN
  IF NEW.recipe<>'oteryn:npc-handoff.father-material' THEN RETURN NULL; END IF;
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id AND quest_key='oteryn:quest.a_father_s_burden_quest';
 ELSE
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id AND transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete';
 END IF;
 IF q.quest_key IS DISTINCT FROM 'oteryn:quest.a_father_s_burden_quest'
  OR q.cause_kind IS DISTINCT FROM 'command' OR q.created_xact_id IS DISTINCT FROM pg_current_xact_id()
 THEN RAISE EXCEPTION 'Father values require their atomic NPC receipt' USING ERRCODE='23514'; END IF;
 IF q.transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete' THEN
  SELECT * INTO cape FROM game_reward_claim_mint_receipts WHERE character_id=q.character_id
   AND claim_family='RewardClaim' AND claim_production_key='oteryn:reward-claim.npc.tereban.old_cape'
   AND game_session_id=q.cause_id AND command_id=q.cause_ordinal;
  IF cape.created_xact_id IS DISTINCT FROM pg_current_xact_id()
   OR q.track_keys IS DISTINCT FROM ARRAY['oteryn:quest-progress/quest/u8_6/afathers_burden/status']::text[]
   OR q.values_before IS DISTINCT FROM ARRAY[1]::bigint[] OR q.values_after IS DISTINCT FROM ARRAY[2]::bigint[]
   OR NOT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=q.character_id
    AND track_key='oteryn:quest-progress/quest/u8_6/afathers_burden/progress' AND value=8)
  THEN RAISE EXCEPTION 'Father completion requires eight hand-ins and one cape' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=cape.item_instance_id AND definition_family='Item' AND definition_production_key='oteryn:item.tibia.i11701' AND quantity=1 AND lifecycle=1 AND minted_transaction_id=cape.transaction_id) THEN RAISE EXCEPTION 'Father requires the exact fresh cape' USING ERRCODE='23514'; END IF;
  required_xp:=8000;
 ELSE
  SELECT * INTO craft FROM game_item_craft_records WHERE character_id=q.character_id AND game_session_id=q.cause_id
   AND command_id=q.cause_ordinal AND recipe='oteryn:npc-handoff.father-material';
  material:=craft.before_item->>'definition_production_key';
  SELECT track,rule INTO expected_track,expected_rule FROM (VALUES
   ('oteryn:item.tibia.i11548','sinew','strong-sinew-deliver'),('oteryn:item.tibia.i11547','wood','exquisite-wood-deliver'),
   ('oteryn:item.tibia.i11546','cloth','spectral-cloth-deliver'),('oteryn:item.tibia.i11545','silk','exquisite-silk-deliver'),
   ('oteryn:item.tibia.i11552','crystal','magic-crystal-deliver'),('oteryn:item.tibia.i11551','root','mystic-root-deliver'),
   ('oteryn:item.tibia.i11549','iron','old-iron-deliver'),('oteryn:item.tibia.i11550','scale','flexible-dragon-scale-deliver')) v(item,track,rule)
   WHERE item=material;
  IF craft.created_xact_id IS DISTINCT FROM pg_current_xact_id() OR expected_track IS NULL
   OR q.transition_key IS DISTINCT FROM 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/'||expected_rule
   OR cardinality(q.track_keys)<>2 OR NOT craft.consume_source OR craft.outputs IS DISTINCT FROM '[]'::jsonb
   OR array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track) IS NULL
   OR array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress') IS NULL
   OR q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track)] IS DISTINCT FROM 1
   OR q.values_after[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track)] IS DISTINCT FROM 2
   OR q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')] NOT BETWEEN 0 AND 7
   OR q.values_after[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')]
    IS DISTINCT FROM q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')]+1
  THEN RAISE EXCEPTION 'Father hand-in requires its exact material and progress' USING ERRCODE='23514'; END IF;
  PERFORM game_item_craft_complete(craft.transaction_id);
  required_xp:=2500;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_quest_xp_obligations x WHERE x.character_id=q.character_id
  AND x.cause_id=q.cause_id AND x.cause_ordinal=q.cause_ordinal AND x.transition_key=q.transition_key
  AND x.amount=required_xp AND x.xmin=pg_current_xact_id()::xid)
 THEN RAISE EXCEPTION 'Father values require their atomic quest XP obligation' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_father_npc_values AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.quest_key='oteryn:quest.a_father_s_burden_quest'
 AND (NEW.transition_key LIKE '%-deliver' OR NEW.transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete'))
 EXECUTE FUNCTION game_father_values_proven();
CREATE CONSTRAINT TRIGGER game_father_material_values AFTER INSERT ON game_item_craft_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_father_values_proven();
CREATE CONSTRAINT TRIGGER game_father_cape_values AFTER INSERT ON game_reward_claim_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.claim_family='RewardClaim' AND NEW.claim_production_key='oteryn:reward-claim.npc.tereban.old_cape')
 EXECUTE FUNCTION game_father_values_proven();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_father_values_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_father_values_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_father_values_proven() TO oteryn_game_runtime;
