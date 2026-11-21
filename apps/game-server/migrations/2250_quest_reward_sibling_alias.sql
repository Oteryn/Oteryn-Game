-- M12: the sibling proof added in 2100 shares a PL/pgSQL scope with the
-- pre-existing BIGINT variable `actual`. Qualify the JSON row alias without
-- changing any proof predicate, quantity, reward identity or receipt guard.
DO $migration$
DECLARE
    body TEXT;
    alias_anchor TEXT := 'jsonb_array_elements(r.additional_plan) actual';
    value_anchor TEXT := 'actual->>';
BEGIN
    body := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body) - length(replace(body, alias_anchor, ''))) / length(alias_anchor) <> 1
       OR (length(body) - length(replace(body, value_anchor, ''))) / length(value_anchor) <> 3
       OR position('minted_sibling' IN body) <> 0 THEN
        RAISE EXCEPTION 'unexpected quest reward sibling guard';
    END IF;
    body := replace(body, alias_anchor, 'jsonb_array_elements(r.additional_plan) minted_sibling');
    body := replace(body, value_anchor, 'minted_sibling->>');
    EXECUTE body;
END
$migration$;
