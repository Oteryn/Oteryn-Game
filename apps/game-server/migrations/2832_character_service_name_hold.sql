CREATE TABLE game_character_service_name_holds (
 name_key TEXT PRIMARY KEY, character_id UUID NOT NULL REFERENCES game_character_roots,
 account_id UUID NOT NULL, released_at BIGINT NOT NULL, available_at BIGINT NOT NULL CHECK(available_at=released_at+2592000)
);
CREATE INDEX game_character_service_name_holds_character ON game_character_service_name_holds(character_id);
CREATE INDEX game_character_service_name_holds_due ON game_character_service_name_holds(available_at,name_key);
DO $$ DECLARE body TEXT;BEGIN
 SELECT pg_get_functiondef('game_account_closure_name_guard()'::regprocedure) INTO STRICT body;
 body:=replace(body,E'BEGIN\n',E'BEGIN\n IF TG_OP=\'DELETE\' AND EXISTS(SELECT 1 FROM game_character_service_name_holds h WHERE h.name_key=OLD.name_key AND h.character_id=OLD.character_id AND game_character_service_now()>=h.available_at) AND NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=OLD.character_id AND r.name_key=OLD.name_key AND r.lifecycle<>4) THEN RETURN OLD;END IF;\n');
 EXECUTE body;
END $$;
CREATE FUNCTION game_character_service_release_name(k TEXT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE h game_character_service_name_holds;n BIGINT;BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_service_name_holds WHERE name_key=k) THEN RETURN;END IF;
 n:=game_character_service_now();
 IF n IS NULL OR n<=0 THEN RAISE EXCEPTION 'trusted service clock unavailable';END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-name:'||k,0));
 SELECT * INTO h FROM game_character_service_name_holds WHERE name_key=k FOR UPDATE;
 IF FOUND AND n>=h.available_at AND NOT EXISTS(SELECT 1 FROM game_character_service_changes c WHERE c.character_id=h.character_id AND c.occurred_at>n)
 AND NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=h.character_id AND r.name_key=k AND r.lifecycle<>4) THEN
 DELETE FROM game_character_name_reservations WHERE name_key=k AND character_id=h.character_id;
 DELETE FROM game_character_service_name_holds WHERE name_key=k;
 END IF;
END $$;
CREATE FUNCTION game_character_service_name_changed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE n BIGINT:=game_character_service_now();BEGIN
 IF n IS NULL THEN RAISE EXCEPTION 'trusted service clock unavailable';END IF;
 INSERT INTO game_character_service_name_holds VALUES(OLD.name_key,OLD.character_id,OLD.account_id,n,n+2592000);
 IF NEW.lifecycle<>4 THEN
  INSERT INTO game_character_name_reservations(name_key,naming_policy_revision,character_id,reserved_at) VALUES(NEW.name_key,1,NEW.character_id,n*1000);
 END IF;RETURN NULL;
END $$;
CREATE TRIGGER game_character_service_name_changed AFTER UPDATE ON game_character_roots
 FOR EACH ROW WHEN(NEW.name IS DISTINCT FROM OLD.name OR NEW.lifecycle=4 AND OLD.lifecycle<>4) EXECUTE FUNCTION game_character_service_name_changed();
REVOKE ALL ON game_character_service_name_holds FROM PUBLIC;
GRANT SELECT ON game_character_service_name_holds TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_character_service_release_name(TEXT),game_character_service_name_changed() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_service_release_name(TEXT) TO oteryn_game_runtime;
