-- PUBPROJ-B: only deaths captured after installation have a known public PvP
-- classification. Never reinterpret unclassified historical receipts as PvE.
CREATE TABLE game_public_death_classifications (
 death_occurrence_id UUID PRIMARY KEY REFERENCES game_character_death_receipts(death_occurrence_id),
 pvp BOOLEAN NOT NULL,
 publishable BOOLEAN NOT NULL DEFAULT true,
 -- Retained for future world x primary-killer x UTC-day admin analytics.
 -- Never infer this from damage/assist contributors. NULL is invalid-source
 -- quarantine; an actually unattributed final blow is the literal unknown.
 killer_key TEXT CHECK(killer_key ~ '^[A-Za-z0-9:._/-]{1,255}$'),
 CHECK(killer_key IS NOT NULL OR NOT publishable)
);
CREATE INDEX game_public_death_classification_killer ON game_public_death_classifications(killer_key);
REVOKE ALL ON game_public_death_classifications FROM PUBLIC;
GRANT SELECT ON game_public_death_classifications TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT ON game_public_death_classifications TO oteryn_game_runtime;

CREATE FUNCTION game_public_death_capture_insert_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_death_receipts d
    WHERE d.death_occurrence_id=NEW.death_occurrence_id
    AND d.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'public death identity requires its fenced death transaction' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_public_death_capture_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'public death history is immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER public_death_classification_insert BEFORE INSERT ON game_public_death_classifications
 FOR EACH ROW EXECUTE FUNCTION game_public_death_capture_insert_guard();
CREATE TRIGGER public_death_killer_insert BEFORE INSERT ON game_public_death_killers
 FOR EACH ROW EXECUTE FUNCTION game_public_death_capture_insert_guard();
CREATE TRIGGER public_death_classification_immutable BEFORE UPDATE OR DELETE ON game_public_death_classifications
 FOR EACH ROW EXECUTE FUNCTION game_public_death_capture_immutable();
CREATE TRIGGER public_death_killer_immutable BEFORE UPDATE OR DELETE ON game_public_death_killers
 FOR EACH ROW EXECUTE FUNCTION game_public_death_capture_immutable();
CREATE TRIGGER public_death_classification_no_truncate BEFORE TRUNCATE ON game_public_death_classifications
 EXECUTE FUNCTION game_public_death_capture_immutable();
CREATE TRIGGER public_death_killer_no_truncate BEFORE TRUNCATE ON game_public_death_killers
 EXECUTE FUNCTION game_public_death_capture_immutable();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_public_death_capture_insert_guard() SET search_path=%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_public_death_capture_immutable() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_death_capture_insert_guard(),game_public_death_capture_immutable() FROM PUBLIC;

-- The last capture insert is the publication boundary, after all private killers.
-- Runtime gets no grant to call an arbitrary projection-rebuilding definer.
CREATE FUNCTION game_public_death_classification_publish() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID;
BEGIN
 SELECT character_id INTO STRICT c FROM game_character_death_receipts
 WHERE death_occurrence_id=NEW.death_occurrence_id;
 PERFORM game_public_v2_deaths(c);
 RETURN NULL;
END $$;
CREATE TRIGGER public_death_classification_publish AFTER INSERT ON game_public_death_classifications
 FOR EACH ROW EXECUTE FUNCTION game_public_death_classification_publish();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_public_death_classification_publish() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_death_classification_publish() FROM PUBLIC;

-- Include deleted/hidden subjects for explicit safe suppression. Historical
-- receipts stay unclassified; the reader must never invent their missing facts.
DO $$
DECLARE c UUID;
BEGIN
 FOR c IN SELECT character_id FROM game_character_roots
          UNION SELECT character_id FROM game_character_death_receipts LOOP
  PERFORM game_public_v2_deaths(c);
 END LOOP;
END $$;
