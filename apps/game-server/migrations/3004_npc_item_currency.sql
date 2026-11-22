-- The trade receipt retains the authored unit price. Gold totals count only gold.
ALTER TABLE game_npc_trades
    ADD COLUMN currency_key TEXT,
    ADD COLUMN currency_revision TEXT,
    DROP CONSTRAINT game_npc_trades_check,
    ADD CONSTRAINT game_npc_trade_price_denomination CHECK (
        CASE WHEN currency_key IS NULL THEN
            currency_revision IS NULL AND total_gold_units = unit_price * quantity
        ELSE
            currency_revision IS NOT NULL
            AND octet_length(currency_key) BETWEEN 1 AND 512
            AND octet_length(currency_revision) BETWEEN 1 AND 512
            AND currency_key <> item_definition_production_key
            AND game_item_coin_worth(currency_key) IS NULL
            AND side IN (1, 2) AND house_key IS NULL
            AND unit_price * quantity BETWEEN 1 AND 2000
            AND total_gold_units = 0 AND burned_gold_units = 0
            AND minted_gold_units = 0 AND bank_debit_gold_units = 0
        END);

-- Currency lines use the same custody proof as the merchandise lines. Prove the
-- exact denomination and quantity independently of the merchant's merchandise.
CREATE FUNCTION game_npc_trade_item_currency_proven(trade game_npc_trades) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    WITH payment AS (
        SELECT l.*, i.definition_family, i.definition_production_key, i.definition_revision_ref,
               lead(l.placement_ordinal) OVER (ORDER BY l.line_ordinal) AS next_ordinal
        FROM game_npc_trade_lines l
        JOIN game_item_instances i USING (item_instance_id)
        WHERE l.transaction_id = trade.transaction_id
          AND l.line_kind = CASE trade.side WHEN 1 THEN 3 ELSE 4 END
    )
    SELECT count(*) BETWEEN 1 AND 20
       AND sum(abs(quantity_before - quantity_after)) = trade.unit_price * trade.quantity
       AND bool_and((definition_family, definition_production_key, definition_revision_ref)
                    = ('Item', trade.currency_key, trade.currency_revision))
       AND bool_and(trade.side = 2 OR next_ordinal IS NULL
                    OR (quantity_after = 0 AND placement_ordinal > next_ordinal))
    FROM payment
$$;

DO $$ BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_npc_trade_item_currency_proven(game_npc_trades) SET search_path=pg_catalog,%I,pg_temp',
        current_schema());
END $$;
REVOKE ALL ON FUNCTION game_npc_trade_item_currency_proven(game_npc_trades) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_trade_item_currency_proven(game_npc_trades) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_npc_trade_item_currency_proven(game_npc_trades) TO oteryn_game_control;

DO $migration$
DECLARE body TEXT; anchor TEXT; begin_at INT;
BEGIN
    body := pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    begin_at := regexp_instr(body, E'\\mBEGIN\\M', 1, 1, 0, 'i');
    IF begin_at = 0 THEN RAISE EXCEPTION 'missing NPC trade consistency body'; END IF;
    body := overlay(body placing $guard$
    IF NEW.currency_key IS NOT NULL
       AND game_npc_trade_item_currency_proven(NEW) IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'NPC trade must settle the exact item currency price' USING ERRCODE='23514';
    END IF;
$guard$ from begin_at+5 for 0);

    anchor := 'AND NOT (game_npc_yeo_barter(NEW) AND l.line_kind=3)';
    IF strpos(body, anchor) = 0 THEN RAISE EXCEPTION 'missing NPC trade line-kind guard'; END IF;
    body := replace(body, anchor, anchor || $guard$
        AND NOT (NEW.currency_key IS NOT NULL AND l.line_kind = CASE NEW.side WHEN 1 THEN 3 ELSE 4 END)
$guard$);

    anchor := '(l.line_kind IN (3, 4) AND (i.definition_family, CASE WHEN NEW.side=3';
    IF strpos(body, anchor) = 0 THEN RAISE EXCEPTION 'missing NPC trade merchandise guard'; END IF;
    body := replace(body, anchor, $guard$
        (l.line_kind IN (3, 4)
            AND NOT (NEW.currency_key IS NOT NULL AND l.line_kind = CASE NEW.side WHEN 1 THEN 3 ELSE 4 END)
            AND (i.definition_family, CASE WHEN NEW.side=3$guard$);
    EXECUTE body;

    body := pg_get_functiondef('game_npc_trade_fluid_guard()'::regprocedure);
    anchor := 'AND l.line_kind IN (3,4)';
    IF strpos(body, anchor) = 0 THEN RAISE EXCEPTION 'missing NPC trade merchandise subtype guard'; END IF;
    body := replace(body, anchor, anchor || $guard$
        AND NOT (NEW.currency_key IS NOT NULL AND l.line_kind = CASE NEW.side WHEN 1 THEN 3 ELSE 4 END)
$guard$);
    EXECUTE body;
END $migration$;
