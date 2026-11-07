-- P11: the admitted mechanical rod uses the same bait/terrain/craft receipt proof.
-- Keep 0549's guard and the current function definition; extend only the rod identity.
DO $migration$
DECLARE
    body TEXT;
    before TEXT := $guard$NEW.consume_source AND tool.definition_production_key='oteryn:item.tibia.i3483'
                        AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3492'$guard$;
    after TEXT := $guard$NEW.consume_source AND tool.definition_production_key IN ('oteryn:item.tibia.i3483','oteryn:item.tibia.i9306')
                        AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3492'$guard$;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    IF length(body) - length(replace(body, before, '')) <> length(before) THEN
        RAISE EXCEPTION 'expected exactly one gathering fishing-rod admission guard';
    END IF;
    -- pg_get_functiondef retains the current attributes and search_path. Its
    -- CREATE OR REPLACE preserves this function's identity, owner and permissions.
    EXECUTE replace(body, before, after);
END $migration$;
