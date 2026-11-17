-- MAPDISC: Measuring Tibia quest progress fed by Cyclopedia subarea discoveries.
-- Crystal 201189f8 discovery_system.lua:528-563 (grantMilestones): stage 1 after the first
-- fully explored main area, 2 at 50% and 3 at 100% of all main areas. The counts are the
-- values seen by the latest advancing discovery; a reached stage never regresses (a shuffle
-- may reset one subarea, but a milestone already earned stays).
CREATE TABLE game_character_measuring_tibia (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 discovered_subareas INTEGER NOT NULL CHECK(discovered_subareas BETWEEN 0 AND 1024),
 completed_areas INTEGER NOT NULL CHECK(completed_areas BETWEEN 0 AND 1024),
 main_areas INTEGER NOT NULL CHECK(main_areas BETWEEN 1 AND 1024),
 stage SMALLINT NOT NULL CHECK(stage BETWEEN 0 AND 3),
 occurrence UUID NOT NULL REFERENCES game_character_cyclopedia_receipts DEFERRABLE INITIALLY DEFERRED,
 CHECK(completed_areas<=main_areas)
);
CREATE INDEX game_character_measuring_tibia_occurrence ON game_character_measuring_tibia(occurrence);
CREATE FUNCTION game_measuring_tibia_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.stage<OLD.stage THEN
  RAISE EXCEPTION 'Measuring Tibia stage cannot regress' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
   WHERE r.occurrence=NEW.occurrence AND r.character_id=NEW.character_id
     AND r.committed_character_revision IS NOT NULL AND r.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Measuring Tibia progress requires this transaction Character receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_measuring_tibia_consistent AFTER INSERT OR UPDATE ON game_character_measuring_tibia
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_measuring_tibia_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_measuring_tibia_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_measuring_tibia_guard() FROM PUBLIC;
REVOKE ALL ON game_character_measuring_tibia FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_measuring_tibia TO oteryn_game_runtime;
GRANT SELECT ON game_character_measuring_tibia TO oteryn_game_control;
