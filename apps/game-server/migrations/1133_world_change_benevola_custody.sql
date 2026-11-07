-- WORLDCHG-Q: a closed native Benevola command uses the existing Quest/custody receipt.
-- No new item, quest, contribution or scheduling owner. Existing USE proof arms stay exact.
DO $migration$
DECLARE body TEXT; old_arm TEXT := 'r.cause_kind=''use'' AND r.created_xact_id=pg_current_xact_id()';
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 IF position(old_arm IN body)=0 THEN RAISE EXCEPTION 'missing carried quest USE proof arm'; END IF;
 body:=replace(body,old_arm,$arm$
 (r.cause_kind='use' OR (
   r.cause_kind='command'
   AND NEW.transition_key='oteryn:quest-transition/worldchange/benevola/handover'
   AND NEW.quantity=1 AND NEW.replacement IS NULL
   AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i12369'
   AND convert_from(NEW.proof,'UTF8')::jsonb->'native_npc'->>'native_npc'='oteryn:npc.benevola'
   AND convert_from(NEW.proof,'UTF8')::jsonb->'native_npc'->'source'->'position'='[32596,31745,7]'::jsonb
   AND EXISTS(SELECT 1 FROM game_world_change_credit_receipts c
      WHERE c.world_id=(NEW.before_item->>'world_id')::uuid AND c.source_kind=3
      AND c.occurrence=decode(convert_from(NEW.proof,'UTF8')::jsonb->'native_npc'->>'counter_occurrence','hex'))
 )) AND r.created_xact_id=pg_current_xact_id()
$arm$);
 EXECUTE body;
END $migration$;

-- A receipt-only native handover has no personal tracks, completion or reward.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
 cardinality(track_keys) BETWEEN 0 AND 8
 AND (cardinality(track_keys)>0 OR completes OR (
   transition_key='oteryn:quest-transition/worldchange/benevola/handover'
   AND quest_key='oteryn:quest/worldchange/benevola' AND cause_kind='command' AND NOT completes))
 AND (cardinality(track_keys)=0 OR (array_ndims(track_keys)=1 AND array_lower(track_keys,1)=1
   AND array_ndims(values_before)=1 AND array_lower(values_before,1)=1
   AND array_ndims(values_after)=1 AND array_lower(values_after,1)=1))
 AND cardinality(values_before)=cardinality(track_keys)
 AND cardinality(values_after)=cardinality(track_keys)
 AND array_position(track_keys,NULL) IS NULL
 AND array_position(values_before,NULL) IS NULL
 AND array_position(values_after,NULL) IS NULL);
