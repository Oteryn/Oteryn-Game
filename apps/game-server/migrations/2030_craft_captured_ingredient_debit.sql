-- M22: retain CRAFT-1's fenced receipt and mutation owners. Existing recipes
-- debit one target unit; capture-enabled recipes may debit its entire snapshot.
CREATE FUNCTION game_craft_ingredient_debit_valid(
    has_ingredient BOOLEAN, before_ingredient JSONB, debit INTEGER
) RETURNS BOOLEAN LANGUAGE SQL IMMUTABLE PARALLEL SAFE AS $function$
    SELECT CASE
        WHEN has_ingredient IS NULL OR debit IS NULL OR debit NOT BETWEEN 1 AND 100 THEN FALSE
        WHEN NOT has_ingredient THEN debit=1 AND before_ingredient IS NULL
        WHEN jsonb_typeof(before_ingredient->'quantity') IS DISTINCT FROM 'number' THEN FALSE
        WHEN (before_ingredient->>'quantity') !~ '^[0-9]{1,3}$' THEN FALSE
        ELSE (before_ingredient->>'quantity')::integer BETWEEN 1 AND 100
            AND (debit=1 OR debit=(before_ingredient->>'quantity')::integer)
    END
$function$;

-- This immutable predicate only inspects its arguments and exposes no rows.
-- Its EXECUTE permission remains available to the existing craft INSERT owner.
ALTER TABLE game_item_craft_records
    ADD COLUMN ingredient_count INTEGER NOT NULL DEFAULT 1,
    ADD CONSTRAINT game_item_craft_ingredient_debit_check CHECK (
        game_craft_ingredient_debit_valid(
            ingredient_item_instance_id IS NOT NULL, ingredient_before_item, ingredient_count
        )
    );

DO $migration$
DECLARE body TEXT; before_text TEXT; after_text TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    before_text:=$old$x.quantity=(r.ingredient_before_item->>'quantity')::bigint-1$old$;
    after_text:=$new$x.quantity=(r.ingredient_before_item->>'quantity')::bigint-r.ingredient_count$new$;
    IF (length(body)-length(replace(body,before_text,'')))/length(before_text)<>1 THEN
        RAISE EXCEPTION 'M22 requires exactly one existing craft ingredient completion arm';
    END IF;
    EXECUTE replace(body,before_text,after_text);

    -- The matching Item mutation debit is composed after accepted audit guards by migration 2815.
END $migration$;
