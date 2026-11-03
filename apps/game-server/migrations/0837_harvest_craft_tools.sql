-- M08: harvest yields use the existing gathering craft/terrain transaction.
-- Keep every receipt, custody, replay and zero-debit guard; admit only the
-- scythe, its multi-tool variant and the sickle reached by the harvest owner.
-- Rollback: a forward migration restoring the original two-tool identity list.
DO $migration$
DECLARE
    body TEXT;
    before TEXT := $guard$tool.definition_production_key IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')$guard$;
    after TEXT := $guard$tool.definition_production_key IN ('oteryn:item.tibia.i3293','oteryn:item.tibia.i3453','oteryn:item.tibia.i5908','oteryn:item.tibia.i5942','oteryn:item.tibia.i9596')$guard$;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    IF length(body) - length(replace(body, before, '')) <> length(before) THEN
        RAISE EXCEPTION 'expected exactly one gathering harvest-tool admission guard';
    END IF;
    -- CREATE OR REPLACE retains the function identity, owner and permissions.
    EXECUTE replace(body, before, after);
END $migration$;
