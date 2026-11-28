-- Keep the mint-time deadline for legacy unscheduled corpses. Scheduled
-- Koshei corpses use the ordinary zero-duration terminal-stage rule.
DO $koshei_remains$
DECLARE
    body TEXT := pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure);
    old_branch CONSTANT TEXT := $old$  WHEN m.loot_table_production_key='oteryn:item.tibia.i7535'
    AND original.d[array_length(original.d,1)]=0 THEN
   CASE WHEN EXISTS(
    SELECT 1 FROM game_item_ground_locations g
    JOIN game_item_instances i USING(item_instance_id,world_id)
    JOIN game_item_mint_receipts newer ON newer.transaction_id=i.minted_transaction_id
                                      AND newer.item_instance_id=i.item_instance_id
    WHERE g.world_id=m.death_world_id AND g.scope_kind=m.death_scope_kind
      AND g.channel_id IS NOT DISTINCT FROM m.death_channel_id
      AND g.instance_id IS NOT DISTINCT FROM m.death_instance_id
      AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i7540'
      AND newer.loot_purpose_key='CORPSE_MATERIALIZATION' AND newer.draw_ordinal=0
      AND newer.loot_table_production_key=m.loot_table_production_key
      AND (newer.materialized_at,newer.item_instance_id)>(m.materialized_at,m.item_instance_id)
   ) THEN m.materialized_at+(SELECT sum(duration) FROM pg_catalog.unnest(original.d) AS source(duration)) END
$old$;
    replacement CONSTANT TEXT := '  -- Scheduled Koshei remains (i7540) are permanent.
';
BEGIN
    IF strpos(body, replacement) > 0 THEN RETURN; END IF;
    IF (length(body) - length(replace(body, old_branch, ''))) / length(old_branch) <> 1 THEN
        RAISE EXCEPTION 'Koshei retirement deadline branch changed';
    END IF;
    EXECUTE replace(body, old_branch, replacement);
END $koshei_remains$;
