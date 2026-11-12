-- WORLDCHG-Q: a closed native Benevola command uses the existing Quest/custody receipt.
-- No new item, quest, contribution or scheduling owner. Existing USE proof arms stay exact.
-- Compose inside canonical QENGINE's arm without replacing its receipt predicate
-- or the outer current-transaction check. The older main arm is supported too.
DO $migration$
DECLARE body TEXT; old_arm TEXT := 'r.cause_kind=''use''';
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 IF (length(body)-length(replace(body,old_arm,'')))/length(old_arm)<>1 THEN
   RAISE EXCEPTION 'missing or ambiguous carried quest USE proof arm';
 END IF;
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
 ))
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


-- Consolidated package migration: 1134_yeo_gloo_barter.sql
-- Closed source-backed repeatable Yeo barter on the existing NPC Trade receipt/audit owner.
CREATE FUNCTION game_npc_yeo_barter(t game_npc_trades) RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT t.npc_key='oteryn:npc.yeo' AND t.side=1 AND t.quantity=1 AND t.item_units=1
   AND t.unit_price=0 AND t.total_gold_units=0 AND t.burned_gold_units=0 AND t.minted_gold_units=0
   AND t.item_definition_family='Item' AND t.item_definition_production_key='oteryn:item.tibia.i21803'
   AND t.item_definition_revision_ref='definition-r1'
$$;
REVOKE ALL ON FUNCTION game_npc_yeo_barter(game_npc_trades) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_yeo_barter(game_npc_trades) TO oteryn_game_runtime;
DO $migration$
DECLARE body TEXT; needle TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
 needle:=E'AND l.line_kind IN (CASE NEW.side WHEN 1 THEN 3 ELSE 1 END,\n                                         CASE NEW.side WHEN 1 THEN 3 ELSE 4 END)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing opposite-side NPC line guard'; END IF;
 body:=replace(body,needle,needle||' AND NOT (game_npc_yeo_barter(NEW) AND l.line_kind=3)');
 needle:=E'NEW.item_definition_production_key,\n                                      NEW.item_definition_revision_ref';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing NPC typed definition guard'; END IF;
 body:=replace(body,needle,E'CASE WHEN game_npc_yeo_barter(NEW) AND l.line_kind=3 THEN ''oteryn:item.tibia.i3581'' ELSE NEW.item_definition_production_key END,\n                                      NEW.item_definition_revision_ref');
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing NPC consistency body'; END IF;
 body:=overlay(body placing $arm$
  -- A zero-price Yeo request is never a free generic merchant offer. Exactly ten
  -- real carried Shrimp must be BURN lines before the single Gloo output.
  IF NEW.npc_key='oteryn:npc.yeo' AND (
      NOT game_npc_yeo_barter(NEW)
      OR (SELECT coalesce(sum(l.quantity_before-l.quantity_after),0) FROM game_npc_trade_lines l
          WHERE l.transaction_id=NEW.transaction_id AND l.line_kind=3)<>10
      OR EXISTS(SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=NEW.transaction_id AND l.line_kind NOT IN (3,4))
  ) THEN RAISE EXCEPTION 'Yeo barter requires exactly ten Shrimp for one Gloo' USING ERRCODE='23514'; END IF;
$arm$ from begin_at+5 for 0);
 EXECUTE body;
END $migration$;
