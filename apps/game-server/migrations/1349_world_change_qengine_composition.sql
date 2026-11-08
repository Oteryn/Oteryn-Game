-- Compose WORLDCHG's receipt-only handover with main's QENGINE value-only owner.
-- Canonical migrations 0999-1006 stay unchanged; neither owner admits the other's proofs.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
    (cardinality(track_keys) BETWEEN 0 AND 8
             OR (cause_kind = 'command'
                 AND NOT completes
                 AND quest_key = 'oteryn:quest.a_father_s_burden_quest'
                 AND transition_key = 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'
                 AND track_keys = ARRAY['oteryn:quest-progress/quest/u8_6/afathers_burden/cloth','oteryn:quest-progress/quest/u8_6/afathers_burden/crystal','oteryn:quest-progress/quest/u8_6/afathers_burden/iron','oteryn:quest-progress/quest/u8_6/afathers_burden/progress','oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log','oteryn:quest-progress/quest/u8_6/afathers_burden/root','oteryn:quest-progress/quest/u8_6/afathers_burden/scale','oteryn:quest-progress/quest/u8_6/afathers_burden/silk','oteryn:quest-progress/quest/u8_6/afathers_burden/sinew','oteryn:quest-progress/quest/u8_6/afathers_burden/status','oteryn:quest-progress/quest/u8_6/afathers_burden/wood']::text[]
                 AND values_after = ARRAY[1,1,1,0,1,1,1,1,1,1,1]::bigint[]))
    AND (cardinality(track_keys)>0 OR completes OR transition_key LIKE 'oteryn:quest-reward/%' OR (
        transition_key='oteryn:quest-transition/worldchange/benevola/handover'
        AND quest_key='oteryn:quest/worldchange/benevola' AND cause_kind='command' AND NOT completes))
    AND (cardinality(track_keys)=0 OR (array_ndims(track_keys)=1 AND array_lower(track_keys,1)=1
        AND array_ndims(values_before)=1 AND array_lower(values_before,1)=1
        AND array_ndims(values_after)=1 AND array_lower(values_after,1)=1))
    AND cardinality(values_before)=cardinality(track_keys) AND cardinality(values_after)=cardinality(track_keys)
    AND array_position(track_keys,NULL) IS NULL AND array_position(values_before,NULL) IS NULL
    AND array_position(values_after,NULL) IS NULL);

DO $migration$
DECLARE body TEXT; needle TEXT := 'IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
BEGIN
    body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'missing or ambiguous canonical value-only quest guard';
    END IF;
    body:=replace(body,needle,needle || $arm$
    IF NEW.transition_key='oteryn:quest-transition/worldchange/benevola/handover' THEN
        IF NEW.quest_key='oteryn:quest/worldchange/benevola' AND NEW.cause_kind='command'
            AND NEW.created_xact_id=pg_current_xact_id()
            AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
                WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                    AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                    AND c.created_xact_id=pg_current_xact_id() AND c.quantity=1 AND c.replacement IS NULL
                    AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i12369'
                    AND convert_from(c.proof,'UTF8')::jsonb->'native_npc'->>'native_npc'='oteryn:npc.benevola'
                    AND convert_from(c.proof,'UTF8')::jsonb->'native_npc'->'source'->'position'='[32596,31745,7]'::jsonb
                    AND EXISTS(SELECT 1 FROM game_world_change_credit_receipts r
                        WHERE r.world_id=(c.before_item->>'world_id')::uuid AND r.source_kind=3
                            AND r.occurrence=decode(convert_from(c.proof,'UTF8')::jsonb->'native_npc'->>'counter_occurrence','hex')))
        THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'Benevola handover lacks same-transaction carried proof' USING ERRCODE='23514';
    END IF;
$arm$);
    EXECUTE body;
END $migration$;
