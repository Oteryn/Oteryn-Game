-- WORLD-SRC W12: preserve legacy progress and scope pools to canonical main areas.
ALTER TABLE game_character_cyclopedia
 DROP CONSTRAINT game_character_cyclopedia_progress_check,
 ADD CONSTRAINT game_character_cyclopedia_progress_check CHECK(octet_length(progress::text)<=262144);
ALTER TABLE game_world_cyclopedia ADD COLUMN area_id BIGINT NOT NULL DEFAULT 0 CHECK(area_id BETWEEN 0 AND 1024);
ALTER TABLE game_world_cyclopedia_epochs ADD COLUMN area_id BIGINT NOT NULL DEFAULT 0 CHECK(area_id BETWEEN 0 AND 1024);
ALTER TABLE game_character_cyclopedia_receipts ADD COLUMN area_id BIGINT NOT NULL DEFAULT 0 CHECK(area_id BETWEEN 0 AND 1024);
ALTER TABLE game_world_cyclopedia ADD COLUMN legacy_imported BOOLEAN NOT NULL DEFAULT false;
CREATE UNIQUE INDEX game_cyclopedia_one_legacy_import ON game_world_cyclopedia(world_id) WHERE legacy_imported;
ALTER TABLE game_world_cyclopedia_epochs DROP CONSTRAINT game_world_cyclopedia_epochs_world_id_fkey,
 DROP CONSTRAINT game_world_cyclopedia_epochs_pkey;
ALTER TABLE game_world_cyclopedia DROP CONSTRAINT game_world_cyclopedia_pkey,
 ADD PRIMARY KEY(world_id,area_id);
ALTER TABLE game_world_cyclopedia_epochs ADD PRIMARY KEY(world_id,area_id,epoch),
 ADD FOREIGN KEY(world_id,area_id) REFERENCES game_world_cyclopedia(world_id,area_id);
CREATE OR REPLACE FUNCTION game_cyclopedia_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE entry RECORD;
BEGIN
 IF TG_OP='UPDATE' THEN
  IF COALESCE((OLD.progress->'legacy'->>'discovered')::boolean,(OLD.progress->>'discovered')::boolean,false)
   AND NOT COALESCE((NEW.progress->'legacy'->>'discovered')::boolean,(NEW.progress->>'discovered')::boolean,false) THEN
   RAISE EXCEPTION 'legacy discovery cannot regress' USING ERRCODE='23514';
  END IF;
  FOR entry IN SELECT key,value FROM jsonb_each(COALESCE(OLD.progress->'subareas','{}'::jsonb)) LOOP
   IF (entry.value->>'discovered')::boolean AND NOT COALESCE((NEW.progress->'subareas'->entry.key->>'discovered')::boolean,false) THEN
    RAISE EXCEPTION 'canonical discovery cannot regress' USING ERRCODE='23514';
   END IF;
  END LOOP;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
   WHERE r.occurrence=NEW.occurrence AND r.character_id=NEW.character_id
     AND r.committed_character_revision IS NOT NULL AND r.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'map state requires this transaction Character receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
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
      AND NEW.pool=OLD.pool-10000000 AND e.pool_before=OLD.pool
      AND e.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'area pool requires atomic donation or reset selection' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cyclopedia_initial_pool_consistent AFTER INSERT ON game_world_cyclopedia
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cyclopedia_pool_guard();
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_cyclopedia_state_guard()','game_cyclopedia_pool_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_cyclopedia_state_guard(),game_cyclopedia_pool_guard() FROM PUBLIC;
