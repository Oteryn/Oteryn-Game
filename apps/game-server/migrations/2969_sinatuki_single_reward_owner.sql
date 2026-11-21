-- The quest owner now handles fresh Sinatuki awards. Historical NPC trade
-- rows and the receipt replay path remain valid.
CREATE OR REPLACE FUNCTION game_sinatuki_exchange_permitted(subject UUID) RETURNS BOOLEAN
LANGUAGE sql SECURITY INVOKER STABLE AS $$
    SELECT FALSE
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_sinatuki_exchange_permitted(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

CREATE INDEX game_npc_trades_sinatuki_legacy_shards ON game_npc_trades(character_id)
WHERE side=3 AND npc_key='oteryn:npc.sinatuki'
  AND item_definition_production_key='oteryn:item.tibia.i7290' AND item_units=5;
