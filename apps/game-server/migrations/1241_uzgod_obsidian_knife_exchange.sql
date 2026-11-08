-- Extend the same closed NPC exchange owner to Uzgod, with a durable20h cooldown.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_al_dee_exchange_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_quest_item_exchange_check CHECK (
    side<>3 OR (catalogue_revision>0 AND offer_index=0 AND quantity=1 AND unit_price=0
        AND total_gold_units=0 AND item_units=1 AND item_definition_family='Item'
        AND item_definition_revision_ref='definition-r1' AND burned_gold_units=0
        AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND house_key IS NULL
        AND ((npc_key='oteryn:npc.al_dee' AND item_definition_production_key='oteryn:item.tibia.i3456' AND line_count=2)
          OR (npc_key='oteryn:npc.uzgod' AND item_definition_production_key='oteryn:item.tibia.i5908' AND line_count=3))));

CREATE INDEX game_npc_trades_uzgod_cooldown ON game_npc_trades(character_id,committed_at DESC)
    WHERE side=3 AND npc_key='oteryn:npc.uzgod';

DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    before:='IF NEW.side=3 AND NOT EXISTS (';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'missing Al Dee source guard'; END IF;
    body:=replace(body,before,'IF NEW.side=3 AND NEW.npc_key=''oteryn:npc.al_dee'' AND NOT EXISTS (');
    before:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN i.definition_production_key=''oteryn:item.tibia.i3462'' THEN NEW.item_definition_production_key ELSE NULL END ELSE i.definition_production_key END';
    after:='CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN i.definition_production_key=CASE WHEN NEW.npc_key=''oteryn:npc.uzgod'' THEN CASE l.line_ordinal WHEN 1 THEN ''oteryn:item.tibia.i5889'' ELSE ''oteryn:item.tibia.i3313'' END ELSE ''oteryn:item.tibia.i3462'' END THEN NEW.item_definition_production_key ELSE NULL END ELSE i.definition_production_key END';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'missing fixed exchange definition guard'; END IF;
    body:=replace(body,before,after);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected NPC trade consistency function'; END IF;
    after:=$guard$BEGIN
    IF NEW.side=3 AND NEW.npc_key='oteryn:npc.uzgod' THEN
        IF EXISTS(SELECT 1 FROM game_npc_trades t WHERE t.character_id=NEW.character_id
            AND t.side=3 AND t.npc_key='oteryn:npc.uzgod' AND t.transaction_id<>NEW.transaction_id
            AND t.committed_at>NEW.committed_at-72000000) THEN
            RAISE EXCEPTION 'Uzgod exchange cooldown has not elapsed' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS(SELECT 1 FROM game_npc_trade_lines s JOIN game_item_instances i USING(item_instance_id)
            WHERE s.transaction_id=NEW.transaction_id AND s.line_ordinal=1 AND s.line_kind=3
              AND s.coin_worth IS NULL AND s.quantity_before BETWEEN 1 AND 100
              AND s.quantity_after=s.quantity_before-1 AND i.definition_family='Item'
              AND i.definition_production_key='oteryn:item.tibia.i5889' AND i.definition_revision_ref='definition-r1')
          OR NOT EXISTS(SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
            WHERE l.transaction_id=NEW.transaction_id AND l.line_ordinal=2 AND l.line_kind=3
              AND l.coin_worth IS NULL AND l.quantity_before=1 AND l.quantity_after=0
              AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3313'
              AND i.definition_revision_ref='definition-r1')
          OR EXISTS(SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=NEW.transaction_id
            AND l.line_kind=3 AND (
                EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=l.item_instance_id)
             OR EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=l.item_instance_id)
             OR EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=l.item_instance_id)
             OR EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=l.item_instance_id))) THEN
            RAISE EXCEPTION 'Uzgod requires exactly one plain steel and lance' USING ERRCODE='23514';
        END IF;
    END IF;
    $guard$;
    body:=overlay(body placing after from begin_at for 5);
    EXECUTE body;
END $migration$;
