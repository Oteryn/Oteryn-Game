-- M07: a source carrot STEP consumes an offering without inventing progress.
-- Keep every existing receipt arm and require the existing carried BURN proof.
DO $migration$
DECLARE predicate TEXT; body TEXT;
 needle TEXT:='IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT predicate FROM pg_constraint
 WHERE conrelid='game_character_quest_receipts'::regclass AND conname='game_character_quest_receipt_effects_shape';
 ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
 EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (('
  ||predicate||$shape$) OR (transition_key='oteryn:quest-callback/dreamers-challenge-carrot-entry'
   AND quest_key='oteryn:quest.dreamer_s_challenge_quest' AND cause_kind='step' AND NOT completes
   AND cardinality(track_keys)=0 AND cardinality(values_before)=0 AND cardinality(values_after)=0
   AND level_after=level_before AND experience_after=experience_before))$shape$;
 body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'missing or ambiguous canonical value-only quest guard';
 END IF;
 body:=replace(body,needle,needle||$arm$
 IF NEW.transition_key='oteryn:quest-callback/dreamers-challenge-carrot-entry' THEN
  IF NEW.quest_key='oteryn:quest.dreamer_s_challenge_quest' AND NEW.cause_kind='step'
   AND NEW.level_after=NEW.level_before AND NEW.experience_after=NEW.experience_before
   AND NEW.created_xact_id=pg_current_xact_id()
   AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions p WHERE p.character_id=NEW.character_id
     AND p.cause_id=NEW.cause_id AND p.cause_ordinal=NEW.cause_ordinal AND p.transition_key=NEW.transition_key
     AND p.created_xact_id=pg_current_xact_id() AND p.quantity=1 AND p.replacement IS NULL
     AND p.location_kind IN ('container','nested','backpack','equipment')
     AND p.before_item->>'definition_family'='Item'
     AND p.before_item->>'definition_production_key'='oteryn:item.tibia.i3595'
     AND p.before_item->>'definition_revision_ref'='definition-r1'
     AND convert_from(p.proof,'UTF8')::jsonb->'callback'->'selector'->>'unique'='2241'
     AND convert_from(p.proof,'UTF8')::jsonb->'callback'->>'trigger'='step_in'
     AND convert_from(p.proof,'UTF8')::jsonb->'callback'->>'requires_premium'='true'
     AND convert_from(p.proof,'UTF8')::jsonb->'callback'->'teleport'='{"x":32861,"y":32235,"floor":-9}'::jsonb)
  THEN RETURN NULL; END IF;
  RAISE EXCEPTION 'Dreamer carrot entry lacks same-transaction carried offering' USING ERRCODE='23514';
 END IF;
$arm$);
 EXECUTE body;
END $migration$;

-- The carried input owner historically admits USE and qualified NPC commands.
-- Admit STEP only for this exact carrot receipt; retain every existing arm.
DO $migration$
DECLARE body TEXT; needle TEXT:='r.cause_kind=''use''';
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'missing or ambiguous carried quest cause guard';
 END IF;
 body:=replace(body,needle,$arm$(r.cause_kind='use' OR
  (r.cause_kind='step' AND r.transition_key='oteryn:quest-callback/dreamers-challenge-carrot-entry'
   AND r.quest_key='oteryn:quest.dreamer_s_challenge_quest'
   AND NEW.quantity=1 AND NEW.replacement IS NULL
   AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3595'))$arm$);
 EXECUTE body;
 EXECUTE format('ALTER FUNCTION game_quest_callback_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
