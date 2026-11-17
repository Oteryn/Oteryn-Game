-- Crystal 201189f8, data-global/npc/sinatuki.lua:54-75.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_quest_item_exchange_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_quest_item_exchange_check CHECK (
    side<>3 OR (catalogue_revision>0 AND offer_index=0 AND quantity=1 AND unit_price=0
        AND total_gold_units=0 AND item_definition_family='Item'
        AND item_definition_revision_ref='definition-r1' AND burned_gold_units=0
        AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND house_key IS NULL
        AND ((item_units=1 AND (
               (npc_key='oteryn:npc.al_dee' AND item_definition_production_key='oteryn:item.tibia.i3456' AND line_count=2)
            OR (npc_key='oteryn:npc.uzgod' AND item_definition_production_key='oteryn:item.tibia.i5908' AND line_count=3)
            OR (npc_key='oteryn:npc.seymour' AND item_definition_production_key='oteryn:item.tibia.i3374' AND line_count=2)))
          OR (npc_key='oteryn:npc.sinatuki' AND item_definition_production_key='oteryn:item.tibia.i7290'
            AND item_units=5 AND line_count BETWEEN 2 AND 4 AND fluid_subtype IS NULL AND full_charges IS NULL))));

-- Source storages 41174 (Barbarian Test) and 41276 (The Ice Islands).
CREATE FUNCTION game_sinatuki_exchange_permitted(subject UUID) RETURNS BOOLEAN
LANGUAGE sql SECURITY INVOKER STABLE AS $$
    SELECT count(*)=2 FROM game_character_quest_tracks
    WHERE character_id=subject
      AND ((track_key='oteryn:quest-progress/quest/u8_0/barbarian_test/questline' AND value>=3)
        OR (track_key='oteryn:quest-progress/quest/u8_0/the_ice_islands/questline' AND value>=5))
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_sinatuki_exchange_permitted(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_sinatuki_exchange_permitted(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_sinatuki_exchange_permitted(UUID) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_sinatuki_exchange_permitted(UUID) TO oteryn_game_control;

DO $migration$
DECLARE body TEXT; anchor TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    anchor:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing fixed exchange definition guard'; END IF;
    body:=replace(body,anchor, 'CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN NEW.npc_key=''oteryn:npc.sinatuki'' THEN NEW.item_definition_production_key WHEN');
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing NPC trade consistency body'; END IF;
    body:=overlay(body placing $guard$
    IF NEW.side=3 AND NEW.npc_key='oteryn:npc.sinatuki' AND (
         NOT game_sinatuki_exchange_permitted(NEW.character_id)
         OR NOT EXISTS (
            SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
            WHERE l.transaction_id=NEW.transaction_id AND l.line_kind=3
            GROUP BY i.definition_production_key
            HAVING i.definition_production_key IN ('oteryn:item.tibia.i7158','oteryn:item.tibia.i7159')
              AND sum(l.quantity_before-l.quantity_after) IN (99,100)
              AND count(*)=NEW.line_count-1)
    ) THEN
        RAISE EXCEPTION 'Sinatuki requires both quest prerequisites and 99 or 100 fish of one type'
            USING ERRCODE='23514';
    END IF;
$guard$ from begin_at+5 for 0);
    EXECUTE body;
END $migration$;
