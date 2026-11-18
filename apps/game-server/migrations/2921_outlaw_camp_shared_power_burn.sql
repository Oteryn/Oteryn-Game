-- Outlaw Camp's power burn consumes a shared ring without changing personal progress.
DO $migration$
DECLARE
 predicate TEXT;
 body TEXT;
 needle CONSTANT TEXT := 'IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
BEGIN
 SELECT pg_get_expr(conbin, conrelid) INTO STRICT predicate
 FROM pg_constraint
 WHERE conrelid = 'game_character_quest_receipts'::regclass
   AND conname = 'game_character_quest_receipt_effects_shape';

 ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
 EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (('
  || predicate || $shape$) OR (
   transition_key = 'oteryn:quest-callback/authored-outlaw-camp-power-burn'
   AND quest_key = 'oteryn:quest.the_outlaw_camp_quest'
   AND cause_kind = 'use' AND NOT completes
   AND cardinality(track_keys) = 0 AND cardinality(values_before) = 0 AND cardinality(values_after) = 0
   AND level_after = level_before AND experience_after = experience_before))$shape$;

 body := pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
 IF (length(body) - length(replace(body, needle, ''))) / length(needle) <> 1 THEN
  RAISE EXCEPTION 'missing or ambiguous canonical value-only quest guard';
 END IF;
 EXECUTE replace(body, needle, needle || $arm$
 IF NEW.transition_key = 'oteryn:quest-callback/authored-outlaw-camp-power-burn' THEN
  IF NEW.quest_key = 'oteryn:quest.the_outlaw_camp_quest' AND NEW.cause_kind = 'use'
   AND NEW.level_after = NEW.level_before AND NEW.experience_after = NEW.experience_before
   AND NEW.created_xact_id = pg_current_xact_id()
   AND EXISTS (
    SELECT 1 FROM game_quest_callback_consumptions c
    WHERE c.character_id = NEW.character_id AND c.cause_id = NEW.cause_id
      AND c.cause_ordinal = NEW.cause_ordinal AND c.transition_key = NEW.transition_key
      AND c.created_xact_id = pg_current_xact_id() AND c.location_kind = 'ground'
      AND c.quantity = 1 AND c.replacement IS NULL
      AND c.before_item->>'definition_family' = 'Item'
      AND c.before_item->>'definition_production_key' = 'oteryn:item.tibia.i3050'
      AND c.before_item->>'definition_revision_ref' = 'definition-r1'
      AND c.before_ground->'spatial_position' = to_jsonb(int4send(32613) || int4send(32220) || int2send(-10::smallint))
   ) THEN RETURN NULL; END IF;
  RAISE EXCEPTION 'Outlaw Camp power burn lacks same-transaction pedestal ring' USING ERRCODE = '23514';
 END IF;
$arm$);
END $migration$;

REVOKE ALL ON FUNCTION game_quest_reward_value_only_complete() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_value_only_complete() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_quest_reward_value_only_complete() TO oteryn_game_control;
