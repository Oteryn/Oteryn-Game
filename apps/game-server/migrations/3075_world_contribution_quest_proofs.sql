-- The selected pump and the NPC donation both burn one carried coal into the
-- existing world contribution ledger. Neither produces a personal reward.
CREATE FUNCTION game_quest_coal_contribution_proven(r game_character_quest_receipts)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $proof$
    SELECT EXISTS (
        SELECT 1
        FROM game_quest_callback_consumptions c
        JOIN game_world_reset_active a ON a.world_id=(c.before_item->>'world_id')::uuid
        JOIN game_world_change_contributions w USING (world_id, reset_epoch)
        CROSS JOIN LATERAL (SELECT convert_from(c.proof,'UTF8')::jsonb AS input) p
        WHERE c.character_id=r.character_id AND c.cause_id=r.cause_id
          AND c.cause_ordinal=r.cause_ordinal AND c.transition_key=r.transition_key
          AND c.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
          AND r.cause_ordinal BETWEEN 1 AND 18446744073709551615
          AND c.location_kind IN ('container','nested','backpack','equipment')
          AND c.quantity=1 AND c.replacement IS NULL
          AND c.before_item->>'definition_family'='Item'
          AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i12600'
          AND c.before_item->>'definition_revision_ref'='definition-r1'
          AND NOT EXISTS (SELECT 1 FROM game_quest_callback_consumptions other
              WHERE other.character_id=r.character_id AND other.cause_id=r.cause_id
                AND other.cause_ordinal=r.cause_ordinal AND other.transition_key=r.transition_key
                AND other.item_instance_id<>c.item_instance_id)
          AND w.xmin=(pg_current_xact_id()::text::numeric % 4294967296)::text::xid
          AND w.amount=1
          AND w.operation_id=substring(sha256(convert_to('oteryn:world-contribution-command:v1','UTF8')
              || uuid_send(r.cause_id) || int8send((CASE WHEN r.cause_ordinal<=9223372036854775807
                  THEN r.cause_ordinal ELSE r.cause_ordinal-18446744073709551616 END)::bigint)),1,16)
          AND NOT r.completes AND cardinality(r.track_keys)=0
          AND r.level_after=r.level_before AND r.experience_after=r.experience_before
          AND ((r.cause_kind='use' AND r.quest_key='oteryn:quest.awash_world_change'
              AND w.resource='awash_coal' AND p.input #>> '{context,reset_epoch}'=a.reset_epoch::text
              AND p.input #> '{context,selected_tool,source}'=to_jsonb(ARRAY(
                  SELECT get_byte(uuid_send(c.item_instance_id), i) FROM generate_series(0,15) i ORDER BY i))
              AND p.input #>> '{callback,key}'=r.transition_key
              AND p.input #>> '{callback,trigger}'='use'
              AND p.input #> '{callback,selector,native}'=CASE r.transition_key
                  WHEN 'oteryn:quest-callback/authored-awash-pump-north'
                    THEN '{"position":{"x":32558,"y":32007,"floor":-9},"ordinal":1,"appearance":12666}'::jsonb
                  WHEN 'oteryn:quest-callback/authored-awash-pump-middle'
                    THEN '{"position":{"x":32560,"y":32014,"floor":-9},"ordinal":1,"appearance":12667}'::jsonb
                  WHEN 'oteryn:quest-callback/authored-awash-pump-south'
                    THEN '{"position":{"x":32567,"y":32024,"floor":-9},"ordinal":1,"appearance":12666}'::jsonb
                  END)
            OR (r.cause_kind='command' AND r.quest_key='oteryn:quest.steamship_world_change'
              AND r.transition_key='oteryn:quest-reward/authored/steamship/donation' AND w.resource='coal'
              AND EXISTS (SELECT 1 FROM game_quest_reward_sources s
                  WHERE s.character_id=r.character_id AND s.cause_id=r.cause_id
                    AND s.cause_ordinal=r.cause_ordinal AND s.transition_key=r.transition_key
                    AND s.created_xact_id=pg_current_xact_id() AND s.mint_transaction_id IS NULL
                    AND cardinality(s.grant_keys)=0 AND s.plan->>'npc'='oteryn:npc.junkar_thais'
                    AND s.plan->>'quest'=r.quest_key AND s.plan->'repeatable'='true'::jsonb
                    AND s.plan->'give'='null'::jsonb AND s.plan->'storage'='[]'::jsonb
                    AND s.plan->>'fee' IS NULL AND s.plan->>'experience' IS NULL AND s.plan->>'outfit' IS NULL
                    AND s.plan->>'world_claim' IS NULL AND s.plan->>'achievement' IS NULL
                    AND coalesce(s.plan->'state_ops','[]'::jsonb)='[]'::jsonb
                    AND coalesce(s.plan->'outfits','[]'::jsonb)='[]'::jsonb
                    AND coalesce(s.plan->'achievement_progress','[]'::jsonb)='[]'::jsonb
                    AND jsonb_array_length(s.plan->'take')=1
                    AND s.plan #>> '{take,0,item}'='oteryn:item.tibia.i12600'
                    AND s.plan #>> '{take,0,count}'='1'
                    AND s.plan #>> '{take,0,fluid_subtype}' IS NULL)))
    )
$proof$;

DO $migration$
DECLARE predicate TEXT; body TEXT;
    value_anchor TEXT := 'IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
    repeat_anchor TEXT := 'IF NEW.plan->>''repeatable''=''true'' AND NOT game_quest_carried_repeatable_complete(NEW) AND (';
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_coal_contribution_proven(game_character_quest_receipts) SET search_path=pg_catalog,%I,pg_temp', current_schema());
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT predicate FROM pg_constraint
        WHERE conrelid='game_character_quest_receipts'::regclass AND conname='game_character_quest_receipt_effects_shape';
    ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (('
        || predicate || $shape$) OR (quest_key='oteryn:quest.awash_world_change' AND cause_kind='use'
        AND transition_key IN ('oteryn:quest-callback/authored-awash-pump-north',
            'oteryn:quest-callback/authored-awash-pump-middle','oteryn:quest-callback/authored-awash-pump-south')
        AND NOT completes AND cardinality(track_keys)=0 AND cardinality(values_before)=0 AND cardinality(values_after)=0
        AND level_after=level_before AND experience_after=experience_before))$shape$;

    body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    IF (length(body)-length(replace(body,value_anchor,'')))/length(value_anchor)<>1 THEN
        RAISE EXCEPTION 'missing canonical value-only quest guard';
    END IF;
    EXECUTE replace(body,value_anchor,value_anchor || $arm$
    IF NEW.transition_key IN ('oteryn:quest-callback/authored-awash-pump-north',
        'oteryn:quest-callback/authored-awash-pump-middle','oteryn:quest-callback/authored-awash-pump-south',
        'oteryn:quest-reward/authored/steamship/donation') THEN
        IF game_quest_coal_contribution_proven(NEW) THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'coal contribution lacks its exact carried payment and world credit' USING ERRCODE='23514';
    END IF;
$arm$);

    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,repeat_anchor,'')))/length(repeat_anchor)<>1 THEN
        RAISE EXCEPTION 'missing canonical repeatable quest guard';
    END IF;
    EXECUTE replace(body,repeat_anchor,$arm$IF NEW.plan->>'repeatable'='true'
        AND NOT game_quest_carried_repeatable_complete(NEW)
        AND NOT EXISTS (SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
              AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
              AND r.transition_key='oteryn:quest-reward/authored/steamship/donation'
              AND game_quest_coal_contribution_proven(r)) AND ($arm$);
END $migration$;

REVOKE ALL ON FUNCTION game_quest_coal_contribution_proven(game_character_quest_receipts) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_coal_contribution_proven(game_character_quest_receipts) TO oteryn_game_runtime;
