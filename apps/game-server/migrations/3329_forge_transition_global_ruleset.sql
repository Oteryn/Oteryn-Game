-- Restore the Global Transfer predicates from 3323 after the chest proof
-- replaced the trigger body. CREATE OR REPLACE retains ownership and grants.
DO $migration$
DECLARE
    definition TEXT := pg_get_functiondef('game_forge_transition_proven()'::regprocedure);
    patch RECORD;
BEGIN
    FOR patch IN SELECT * FROM (VALUES
        ($old$r.ruleset_revision<>'crystal-201189f-forge-v1'$old$,
         $new$r.ruleset_revision IS DISTINCT FROM (CASE WHEN p.operation=3 THEN 'global-forge-v2' ELSE 'crystal-201189f-forge-v1' END)$new$),
        ($old$WHEN 4 THEN price.convergence_transfer ELSE price.gold END$old$,
         $new$WHEN 4 THEN price.convergence_transfer WHEN 3 THEN (SELECT admitted.gold FROM game_forge_admitted_price(pa.classification,source) AS admitted) ELSE price.gold END$new$),
        ($old$p.gold_spent<>gold$old$, $new$p.gold_spent IS DISTINCT FROM gold$new$)
    ) AS updates(old_text, new_text)
    LOOP
        IF strpos(definition, patch.old_text) = 0 THEN
            RAISE EXCEPTION 'Forge transition guard is missing its expected definition fragment: %', patch.old_text;
        END IF;
        definition := replace(definition, patch.old_text, patch.new_text);
    END LOOP;
    EXECUTE definition;
END;
$migration$;
