-- Cael's extra book uses the carried-XP payment owner and the same clock as
-- his twelve progression hand-ins. Prove the stamp from the committed receipt.
DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_carried_repeatable_complete(game_quest_reward_sources)'::regprocedure);
    anchors TEXT[] := ARRAY[
        'expected_xp_amount BIGINT; consumed BIGINT;',
        'OR coalesce(p.plan->''state_ops'',''[]''::jsonb)<>''[]''::jsonb',
        'OR r.values_before IS DISTINCT FROM r.values_after
        OR cardinality(r.track_keys)<>jsonb_array_length(p.plan->''storage'')'
    ];
    replacements TEXT[] := ARRAY[
        $declare$expected_xp_amount BIGINT; consumed BIGINT;
        timed_tome BOOLEAN := coalesce(
            p.transition_key='oteryn:quest-reward/l2-cael-tome-13'
            AND p.plan->>'quest'='oteryn:quest.the_new_frontier_quest'
            AND p.plan->>'npc'='oteryn:npc.cael'
            AND p.plan->'experience'='5000'::jsonb
            AND p.plan->'give'='null'::jsonb
            AND p.plan->'take'='[{"item":"oteryn:item.tibia.i10217","count":1}]'::jsonb
            AND p.plan->'storage'='[{"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge","minimum":12,"maximum":12,"set":12,"preserve":true}]'::jsonb
            AND p.plan->'state_ops'='[{"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/last_tome_delivery","from":{"kind":"elapsed_at_least","seconds":72000},"effect":{"kind":"set_now"}}]'::jsonb,
            false);$declare$,
        $plan$OR (NOT timed_tome AND coalesce(p.plan->'state_ops','[]'::jsonb)<>'[]'::jsonb)$plan$,
        $receipt$OR (CASE WHEN timed_tome THEN NOT game_quest_reward_effects_valid(
            p.plan,r.track_keys,r.values_before,r.values_after)
        ELSE r.values_before IS DISTINCT FROM r.values_after
            OR cardinality(r.track_keys)<>jsonb_array_length(p.plan->'storage') END)$receipt$
    ];
BEGIN
    FOR i IN 1..cardinality(anchors) LOOP
        IF (length(body)-length(replace(body,anchors[i],'')))/length(anchors[i])<>1 THEN
            RAISE EXCEPTION 'Missing exact carried XP proof anchor % for the tome clock', i;
        END IF;
        body := replace(body,anchors[i],replacements[i]);
    END LOOP;
    EXECUTE body;
END $migration$;
