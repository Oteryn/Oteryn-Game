-- CRAFT-1 C8: Seymour takes an empty present (2856), gives a legion helmet (3374).
-- Source: Present Quest/Spoiler transcript; same fenced NPC item writer as Al Dee.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_quest_item_exchange_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_quest_item_exchange_check CHECK (
    side<>3 OR (catalogue_revision>0 AND offer_index=0 AND quantity=1 AND unit_price=0
        AND total_gold_units=0 AND item_units=1 AND item_definition_family='Item'
        AND item_definition_revision_ref='definition-r1' AND burned_gold_units=0
        AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND house_key IS NULL
        AND ((npc_key='oteryn:npc.al_dee' AND item_definition_production_key='oteryn:item.tibia.i3456' AND line_count=2)
          OR (npc_key='oteryn:npc.uzgod' AND item_definition_production_key='oteryn:item.tibia.i5908' AND line_count=3)
          OR (npc_key='oteryn:npc.seymour' AND item_definition_production_key='oteryn:item.tibia.i3374' AND line_count=2))));

DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    before:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN i.definition_production_key=CASE WHEN NEW.npc_key=''oteryn:npc.uzgod'' THEN CASE l.line_ordinal WHEN 1 THEN ''oteryn:item.tibia.i5889'' ELSE ''oteryn:item.tibia.i3313'' END ELSE ''oteryn:item.tibia.i3462'' END THEN NEW.item_definition_production_key ELSE NULL END ELSE i.definition_production_key END';
    after:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN i.definition_production_key=CASE WHEN NEW.npc_key=''oteryn:npc.uzgod'' THEN CASE l.line_ordinal WHEN 1 THEN ''oteryn:item.tibia.i5889'' ELSE ''oteryn:item.tibia.i3313'' END WHEN NEW.npc_key=''oteryn:npc.seymour'' THEN ''oteryn:item.tibia.i2856'' ELSE ''oteryn:item.tibia.i3462'' END THEN NEW.item_definition_production_key ELSE NULL END ELSE i.definition_production_key END';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'missing fixed exchange definition guard'; END IF;
    body:=replace(body,before,after);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected NPC trade consistency function'; END IF;
    after:=$guard$BEGIN
    IF NEW.side=3 AND NEW.npc_key='oteryn:npc.seymour' AND NOT EXISTS (
        SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
        WHERE l.transaction_id=NEW.transaction_id AND l.line_ordinal=1 AND l.line_kind=3
          AND l.quantity_before=1 AND l.quantity_after=0 AND l.coin_worth IS NULL
          AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i2856'
          AND i.definition_revision_ref='definition-r1'
          AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=i.item_instance_id)
    ) THEN RAISE EXCEPTION 'Seymour requires one unchanged empty present' USING ERRCODE='23514'; END IF;
    $guard$;
    body:=overlay(body placing after from begin_at for 5);
    EXECUTE body;
END $migration$;
