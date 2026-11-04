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
