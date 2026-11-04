-- FIX-2857: Crystal's Yalahar sewer catch consumes a nail, not a worm.
-- Extend the existing rod/bait arm; retain every custody, terrain receipt,
-- intent, world, replay and debit proof. Only the admitted 4597 -> 4609 catch
-- may use a nail. Rollback is a forward migration restoring the worm-only arm.
DO $migration$
DECLARE
    body TEXT;
    before TEXT := $guard$NEW.consume_source AND tool.definition_production_key IN ('oteryn:item.tibia.i3483','oteryn:item.tibia.i9306')
                        AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3492'$guard$;
    after TEXT := $guard$NEW.consume_source AND (
                        (tool.definition_production_key IN ('oteryn:item.tibia.i3483','oteryn:item.tibia.i9306')
                            AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i3492')
                        OR (tool.definition_production_key='oteryn:item.tibia.i9306'
                            AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i953'
                            AND r.transformed_item_id=4609))$guard$;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    IF strpos(body, after)>0 THEN
        RETURN;
    END IF;
    IF length(body) - length(replace(body, before, '')) <> length(before) THEN
        RAISE EXCEPTION 'expected exactly one gathering fishing-rod admission guard';
    END IF;
    -- CREATE OR REPLACE retains the function identity, owner and permissions.
    EXECUTE replace(body, before, after);
END $migration$;
