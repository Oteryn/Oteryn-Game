CREATE OR REPLACE FUNCTION game_cyclopedia_pool_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.bundle_digest<>OLD.bundle_digest OR NEW.catalogue_digest<>OLD.catalogue_digest THEN
  RAISE EXCEPTION 'map bundle change needs explicit migration' USING ERRCODE='23514';
 END IF;
 IF NEW.pool<>OLD.pool AND NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
    WHERE r.world_id=NEW.world_id AND r.pool_before=OLD.pool AND r.pool_after=NEW.pool
      AND r.donation>0 AND r.created_xact_id=pg_current_xact_id())
   AND NOT EXISTS(SELECT 1 FROM game_world_cyclopedia_epochs e
    WHERE e.world_id=NEW.world_id AND e.donation_winner AND e.pool_before=OLD.pool
      AND NEW.pool=e.pool_before-10000000 AND e.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'area pool requires atomic donation or reset selection' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_cyclopedia_pool_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_cyclopedia_pool_guard() FROM PUBLIC;

-- I11 carried cooking consumes a second, independently fenced backpack item.  The original
-- CRAFT-1 receipt proved only its source, so a successful two-item recipe was rejected by the
-- item-instance guard after the transaction had prepared its result.
ALTER TABLE game_item_craft_records
 ADD COLUMN ingredient_item_instance_id UUID REFERENCES game_item_instances,
 ADD COLUMN ingredient_before_item JSONB,
 ADD CHECK ((ingredient_item_instance_id IS NULL)=(ingredient_before_item IS NULL)),
 ADD CHECK (ingredient_item_instance_id IS NULL OR ingredient_item_instance_id<>source_item_instance_id);

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; before TEXT; after TEXT; begin_at INT;
BEGIN
 -- Admit the second item only while its exact row and direct main-backpack custody still match.
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 before:=E' THEN RAISE EXCEPTION \'craft requires unchanged plain carried input\'';
 after:=E'    OR (NEW.ingredient_item_instance_id IS NOT NULL AND (\n'
  || E'      NOT EXISTS(SELECT 1 FROM game_item_instances x JOIN game_item_container_entries e USING(item_instance_id)\n'
  || E'        JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id\n'
  || E'       WHERE x.item_instance_id=NEW.ingredient_item_instance_id AND to_jsonb(x)=NEW.ingredient_before_item\n'
  || E'         AND x.lifecycle=1 AND x.quantity>0 AND x.world_id=NEW.world_id AND e.character_id=NEW.character_id\n'
  || E'         AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id)))\n'
  || E' THEN RAISE EXCEPTION \'craft requires unchanged plain carried input\'';
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft carried-input guard'; END IF;
 body:=replace(body,before,after);
 EXECUTE body;

 -- The completion constraint counts and checks both decrements for a carried recipe.
 body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
 before:=$find$(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.consume_source THEN 1 ELSE 0 END)$find$;
 after:=$replacement$(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.consume_source THEN 1 ELSE 0 END)+(CASE WHEN r.ingredient_item_instance_id IS NULL THEN 0 ELSE 1 END)
    OR (r.ingredient_item_instance_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_instances x WHERE x.item_instance_id=r.ingredient_item_instance_id
      AND x.quantity=(r.ingredient_before_item->>'quantity')::bigint-1 AND x.lifecycle=CASE WHEN x.quantity=0 THEN 2 ELSE 1 END
      AND x.last_transaction_id=r.transaction_id AND (to_jsonb(x)-ARRAY['quantity','lifecycle','last_transaction_id']) IS NOT DISTINCT FROM
          (r.ingredient_before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
      AND (x.quantity>0 OR NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=x.item_instance_id))))$replacement$;
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft write count'; END IF;
 body:=replace(body,before,after);
 EXECUTE body;

 -- Extend the existing craft proof arms; do not create a parallel item mutation path.
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
     'game_item_container_entry_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_craft_records r
      WHERE r.ingredient_item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
        AND r.ingredient_before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-1 OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'craft decrements exactly one ingredient unit' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.ingredient_item_instance_id=NEW.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_craft_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSE arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
       ON i.item_instance_id=r.ingredient_item_instance_id WHERE r.ingredient_item_instance_id=OLD.item_instance_id
       AND r.character_id=OLD.character_id AND r.parent_item_instance_id=OLD.parent_item_instance_id
       AND r.world_id=OLD.world_id AND i.quantity=0 AND i.lifecycle=2
       AND i.last_transaction_id=r.transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
