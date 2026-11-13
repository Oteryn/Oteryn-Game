-- G-3 repeatable services: never repurpose the create-only 0087 table.
CREATE TABLE game_character_service_receipts (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 command TEXT NOT NULL, variant TEXT, source_revision NUMERIC(20,0),
 intent BYTEA NOT NULL CHECK(octet_length(intent)<=4096),
 receipt BYTEA NOT NULL CHECK(octet_length(receipt) BETWEEN 1 AND 1024),
 result_code TEXT NOT NULL, recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation>0),
 decided_at BIGINT NOT NULL CHECK(decided_at>0), created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((variant IS NULL)=(source_revision IS NULL)),
 CHECK(source_revision IS NULL OR source_revision BETWEEN 1 AND 18446744073709551615)
);
CREATE UNIQUE INDEX game_character_service_receipts_revision ON game_character_service_receipts(variant,source_revision)
 WHERE variant IS NOT NULL AND result_code<>'CHAR_CMD_OPERATION_CONFLICT';
CREATE TRIGGER game_character_service_receipts_immutable BEFORE UPDATE OR DELETE ON game_character_service_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_service_receipts_no_truncate BEFORE TRUNCATE ON game_character_service_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_character_service_operation_guard() RETURNS trigger LANGUAGE plpgsql
 SET search_path=pg_catalog,public AS $$ BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:'||encode(uuid_send(NEW.operation_id),'hex'),0));
 IF TG_TABLE_NAME='game_character_service_receipts' THEN
  IF EXISTS(SELECT 1 FROM game_character_command_receipts WHERE operation_id=NEW.operation_id)
   OR EXISTS(SELECT 1 FROM game_character_operation_receipts WHERE operation_id=NEW.operation_id)
   OR EXISTS(SELECT 1 FROM game_bazaar_decisions WHERE issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND operation_id=NEW.operation_id)
  THEN RAISE EXCEPTION 'character operation belongs to another authority decision' USING ERRCODE='23505';END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_character_service_receipts WHERE operation_id=NEW.operation_id)
  THEN RAISE EXCEPTION 'character operation belongs to services v2' USING ERRCODE='23505';END IF;
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER game_character_service_operation_guard BEFORE INSERT ON game_character_service_receipts FOR EACH ROW EXECUTE FUNCTION game_character_service_operation_guard();
CREATE TRIGGER game_character_service_v1_operation_guard BEFORE INSERT ON game_character_command_receipts FOR EACH ROW EXECUTE FUNCTION game_character_service_operation_guard();
CREATE TRIGGER game_character_service_bootstrap_operation_guard BEFORE INSERT ON game_character_operation_receipts FOR EACH ROW EXECUTE FUNCTION game_character_service_operation_guard();
CREATE TRIGGER game_character_service_bazaar_operation_guard BEFORE INSERT ON game_bazaar_decisions FOR EACH ROW WHEN(NEW.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY') EXECUTE FUNCTION game_character_service_operation_guard();
REVOKE ALL ON game_character_service_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_service_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_service_receipts TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_character_service_operation_guard() FROM PUBLIC;
