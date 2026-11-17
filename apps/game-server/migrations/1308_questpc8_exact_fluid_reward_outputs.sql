-- Preserve exact authored child output equality when consuming main's typed
-- fluid rewards. Legacy children keep their unchanged {item,count} projection.
-- Native fluid-state/custody proofs remain owned by canonical migration1251.
DO $migration$
DECLARE
    body TEXT;
    original TEXT := $old$jsonb_build_object(
                'item',e->>'production_key','count',(e->>'quantity')::bigint)$old$;
BEGIN
    body := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'unexpected installed exact quest output proof';
    END IF;
    EXECUTE replace(body,original,$new$(jsonb_build_object(
                'item',e->>'production_key','count',(e->>'quantity')::bigint)
                || CASE WHEN e ? 'fluid_subtype'
                    THEN jsonb_build_object('fluid_subtype',e->'fluid_subtype')
                    ELSE '{}'::jsonb END)$new$);
END $migration$;
