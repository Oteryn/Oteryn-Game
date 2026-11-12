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

-- The expiry of an unpublishable receipt is a future projection source change,
-- not a reason to stall the Character forever when no new death occurs.
CREATE TABLE game_public_death_rechecks (
 character_id UUID PRIMARY KEY,
 due_at_seconds BIGINT NOT NULL CHECK(due_at_seconds>=0)
);
CREATE INDEX game_public_death_rechecks_due ON game_public_death_rechecks(due_at_seconds,character_id);
CREATE INDEX game_public_death_recent_window ON game_character_death_receipts(character_id,(committed_at/1000) DESC,death_occurrence_id DESC);
REVOKE ALL ON game_public_death_rechecks FROM PUBLIC;
GRANT SELECT ON game_public_death_rechecks TO oteryn_game_control;
CREATE TRIGGER public_death_rechecks_no_truncate BEFORE TRUNCATE ON game_public_death_rechecks
 EXECUTE FUNCTION game_character_reject_truncate();

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

-- PUBPROJ-DEATH-RECHECK-BATCH (resource10093): index reads at most32 due
-- candidates, then locks them in stable UUID order. Do not take a recheck row
-- lock before the source advisory lock used by deaths(), or a source writer
-- updating the same deadline could invert the lock order.
CREATE FUNCTION game_public_death_recheck_due() RETURNS INTEGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID; done INTEGER:=0; cut_s BIGINT;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 cut_s:=floor(extract(epoch FROM clock_timestamp()))::bigint;
 FOR c IN SELECT character_id FROM (
  SELECT character_id FROM game_public_death_rechecks WHERE due_at_seconds<=cut_s
  ORDER BY due_at_seconds,character_id LIMIT 32
 ) due ORDER BY character_id LOOP
  PERFORM game_public_v2_subject_lock('deaths',c::text);
  IF EXISTS(SELECT 1 FROM game_public_death_rechecks WHERE character_id=c AND due_at_seconds<=cut_s) THEN
   PERFORM game_public_v2_deaths(c);
   done:=done+1;
  END IF;
 END LOOP;
 RETURN done;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_public_death_recheck_due() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_public_death_recheck_due() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_public_death_recheck_due() TO oteryn_game_runtime;

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
