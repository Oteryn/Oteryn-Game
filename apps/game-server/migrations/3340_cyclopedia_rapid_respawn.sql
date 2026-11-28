-- Clear the selected donation pool only with its same-transaction epoch receipt.
CREATE OR REPLACE FUNCTION game_cyclopedia_pool_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.area_id=0 OR (NEW.pool<>0 AND (NOT NEW.legacy_imported OR NOT EXISTS(
    SELECT 1 FROM game_world_cyclopedia source WHERE source.world_id=NEW.world_id
      AND source.area_id=0 AND source.pool=NEW.pool))) THEN
   RAISE EXCEPTION 'initial pool requires retained legacy source' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
 IF NEW.area_id=0 THEN
  RAISE EXCEPTION 'retained legacy source is immutable' USING ERRCODE='23514';
 END IF;
 IF NEW.world_id<>OLD.world_id OR NEW.area_id<>OLD.area_id OR NEW.legacy_imported<>OLD.legacy_imported
    OR NEW.bundle_digest<>OLD.bundle_digest OR NEW.catalogue_digest<>OLD.catalogue_digest THEN
  RAISE EXCEPTION 'map identity change needs explicit migration' USING ERRCODE='23514';
 END IF;
 IF NEW.pool<>OLD.pool AND NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
    WHERE r.world_id=NEW.world_id AND r.area_id=NEW.area_id
      AND r.pool_before=OLD.pool AND r.pool_after=NEW.pool
      AND r.donation>0 AND r.created_xact_id=pg_current_xact_id())
   AND NOT EXISTS(SELECT 1 FROM game_world_cyclopedia_epochs e
    WHERE e.world_id=NEW.world_id AND e.area_id=NEW.area_id AND e.donation_winner
      AND NEW.pool=0 AND e.pool_before=OLD.pool
      AND e.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'area pool requires atomic donation or reset selection' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cyclopedia_pool_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cyclopedia_pool_guard() FROM PUBLIC;
