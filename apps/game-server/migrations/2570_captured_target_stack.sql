-- M22: preserve the exact captured target debit in the existing craft receipt.
-- Legacy recipes retain the one-unit policy; only the source-qualified talisman
-- row captures a whole antlers stack and replaces it with one nonstacking object.
ALTER TABLE game_item_craft_records
 ADD COLUMN ingredient_consumed_quantity BIGINT NOT NULL DEFAULT 1
 CHECK (ingredient_consumed_quantity BETWEEN 1 AND 100),
 ADD CONSTRAINT game_item_craft_captured_debit_agrees
 CHECK (ingredient_consumed_quantity=ingredient_count);

DO $migration$
DECLARE body TEXT; before TEXT; after TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 before:=$old$ THEN RAISE EXCEPTION 'craft requires unchanged plain carried input'$old$;
 after:=$new$
    OR NEW.ingredient_consumed_quantity IS DISTINCT FROM
       (CASE WHEN NEW.recipe='oteryn:item-conversion.tainted-souls-antler-talisman'
         THEN (NEW.ingredient_before_item->>'quantity')::bigint ELSE 1 END)
    OR (NEW.recipe='oteryn:item-conversion.tainted-souls-antler-talisman' AND (
        NEW.ingredient_item_instance_id IS NULL OR NOT NEW.consume_source
        OR NEW.before_item->>'definition_production_key'<>'oteryn:item.tibia.i3736'
        OR NEW.before_item->>'definition_revision_ref'<>'definition-r1'
        OR NEW.ingredient_before_item->>'definition_production_key'<>'oteryn:item.tibia.i10297'
        OR NEW.ingredient_before_item->>'definition_revision_ref'<>'definition-r1'
        OR jsonb_array_length(NEW.outputs)<>1
        OR NEW.outputs->0->>'key'<>'oteryn:item.tibia.i22008'
        OR NEW.outputs->0->>'revision'<>'definition-r1'
        OR (NEW.outputs->0->>'count')::bigint<>1))
 THEN RAISE EXCEPTION 'craft requires unchanged plain carried input'$new$;
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft target admission'; END IF;
 EXECUTE replace(body,before,after);

 -- 2030 already routes both completion and the instance mutation through the
 -- captured ingredient_count. The equality constraint above makes the later
 -- exact talisman admission use that same debit without replacing either owner.
END $migration$;
