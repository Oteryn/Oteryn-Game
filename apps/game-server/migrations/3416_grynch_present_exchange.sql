-- Global: https://tibia.fandom.com/wiki/Christmas_Present_Bag
-- Ruprecht exchanges one stolen present bag for one Christmas Token.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_quest_item_exchange_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_quest_item_exchange_check CHECK (
    side<>3 OR (catalogue_revision>0 AND offer_index=0 AND quantity=1 AND unit_price=0
        AND total_gold_units=0 AND item_definition_family='Item'
        AND item_definition_revision_ref='definition-r1' AND burned_gold_units=0
        AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND house_key IS NULL
        AND ((item_units=1 AND (
               (npc_key='oteryn:npc.al_dee' AND item_definition_production_key='oteryn:item.tibia.i3456' AND line_count=2)
            OR (npc_key='oteryn:npc.ruprecht' AND item_definition_production_key='oteryn:item.tibia.i6526' AND line_count=2
                AND fluid_subtype IS NULL AND full_charges IS NULL)
            OR (npc_key='oteryn:npc.seymour' AND item_definition_production_key='oteryn:item.tibia.i3374' AND line_count=2)
            OR (npc_key='oteryn:npc.uzgod' AND item_definition_production_key='oteryn:item.tibia.i5908' AND line_count=3)))
          OR (npc_key='oteryn:npc.sinatuki' AND item_definition_production_key='oteryn:item.tibia.i7290'
            AND item_units=5 AND line_count BETWEEN 2 AND 4 AND fluid_subtype IS NULL AND full_charges IS NULL))));

DO $migration$
DECLARE body TEXT; anchor TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    anchor:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN NEW.npc_key=''oteryn:npc.sinatuki''';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing fixed exchange definition guard'; END IF;
    body:=replace(body,anchor,
        'CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN NEW.npc_key=''oteryn:npc.ruprecht'' AND i.definition_production_key=''oteryn:item.tibia.i6496'' THEN NEW.item_definition_production_key WHEN NEW.npc_key=''oteryn:npc.sinatuki''');
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing NPC trade consistency body'; END IF;
    body:=overlay(body placing $guard$
    IF NEW.side=3 AND NEW.npc_key='oteryn:npc.ruprecht' AND NOT EXISTS (
        SELECT 1 FROM game_npc_trade_lines burn
        JOIN game_item_instances bag ON bag.item_instance_id=burn.item_instance_id
        WHERE burn.transaction_id=NEW.transaction_id AND burn.line_ordinal=1 AND burn.line_kind=3
          AND burn.quantity_before=1 AND burn.quantity_after=0
          AND bag.definition_family='Item' AND bag.definition_production_key='oteryn:item.tibia.i6496'
          AND bag.definition_revision_ref='definition-r1'
          AND NOT EXISTS (SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=bag.item_instance_id)
          AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=bag.item_instance_id)
          AND NOT EXISTS (SELECT 1 FROM game_item_timed_states s WHERE s.item_instance_id=bag.item_instance_id)
          AND NOT EXISTS (SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=bag.item_instance_id)
          AND NOT EXISTS (SELECT 1 FROM game_item_fluid_states f WHERE f.item_instance_id=bag.item_instance_id AND f.fluid_subtype<>0)
    ) THEN
        RAISE EXCEPTION 'Ruprecht requires one Christmas Present Bag' USING ERRCODE='23514';
    END IF;
$guard$ from begin_at+5 for 0);
    EXECUTE body;
END $migration$;
