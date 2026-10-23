-- Existing legacy permissions need explicit reconciliation, never silent reinterpretation.
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM game_house_acl) THEN
  RAISE EXCEPTION 'HOUSE_LEGACY_ACL_RECONCILIATION_REQUIRED';
 END IF;
END $$;
-- HOUSE-WIRE-1: management notices are durable intent, not a disposition receipt.
-- Ownership changes invalidate a notice; only a qualified disposition owner may execute it.
CREATE TABLE game_house_management_receipts (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 action_kind SMALLINT NOT NULL CHECK(action_kind IN(1,2,3)),
 list_key TEXT,
 CHECK((action_kind=1)=(list_key IS NOT NULL)),
 result_revision NUMERIC(20,0) NOT NULL CHECK(result_revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(game_session_id,command_id),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties
);
CREATE TABLE game_house_move_out_notices (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 1 AND 18446744073709551615),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 due_at_unix_ms BIGINT CHECK(due_at_unix_ms>0),
 last_session_id UUID NOT NULL,
 last_command_id NUMERIC(20,0) NOT NULL,
 PRIMARY KEY(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties,
 FOREIGN KEY(last_session_id,last_command_id) REFERENCES game_house_management_receipts DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_house_management_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'House management receipts are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_house_management_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_house_management_receipts FOR EACH ROW EXECUTE FUNCTION game_house_management_receipt_immutable();
REVOKE ALL ON game_house_management_receipts,game_house_move_out_notices FROM PUBLIC;
GRANT SELECT,INSERT ON game_house_management_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_house_move_out_notices TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_receipt_immutable() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_management_receipt_immutable() FROM PUBLIC;
CREATE FUNCTION game_house_notice_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key)<>(OLD.world_id,OLD.house_key)
   OR NEW.revision<>OLD.revision+1) THEN
  RAISE EXCEPTION 'House notice requires exact successor' USING ERRCODE='23514';
 END IF;
 IF (TG_OP='INSERT' AND NEW.revision<>1) OR NOT EXISTS(
  SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.game_session_id=NEW.last_session_id AND r.command_id=NEW.last_command_id
   AND r.world_id=NEW.world_id AND r.house_key=NEW.house_key
   AND r.result_revision=NEW.revision AND r.created_xact_id=pg_current_xact_id()
   AND p.owner_character_id=r.character_id AND p.ownership_generation=NEW.ownership_revision
   AND p.state IN(3,4)) THEN
  RAISE EXCEPTION 'House notice lacks same-transaction owner receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_notice_proven AFTER INSERT OR UPDATE ON game_house_move_out_notices
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_notice_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_notice_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_notice_proven() FROM PUBLIC;
-- OWN0 §10: canonical character-only management, distinct from dormant 0038 spell editors.
CREATE TABLE game_house_acl_entries (
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 list_key TEXT NOT NULL CHECK(list_key IN('guest','subowner') OR list_key ~ '^door:[0-9]{1,5}:[0-9]{1,5}:-?[0-9]{1,2}$'),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 PRIMARY KEY(world_id,house_key,list_key,character_id),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties
);
REVOKE ALL ON game_house_acl_entries FROM PUBLIC;
GRANT SELECT,INSERT,DELETE ON game_house_acl_entries TO oteryn_game_runtime;
CREATE FUNCTION game_house_management_acl_advance(s UUID,c NUMERIC) RETURNS void
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_management_receipts%ROWTYPE; p game_house_properties%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_management_receipts WHERE game_session_id=s AND command_id=c;
 SELECT * INTO p FROM game_house_properties WHERE world_id=r.world_id AND house_key=r.house_key FOR UPDATE;
 IF r.game_session_id IS NULL OR r.action_kind<>1 OR r.created_xact_id<>pg_current_xact_id()
  OR p.world_id IS NULL OR p.state NOT IN(3,4) OR p.acl_revision+1<>r.result_revision
  OR (p.owner_character_id<>r.character_id AND (r.list_key<>'guest' OR NOT EXISTS(
    SELECT 1 FROM game_house_acl_entries WHERE world_id=p.world_id AND house_key=p.house_key
     AND list_key='subowner' AND character_id=r.character_id))) THEN
  RAISE EXCEPTION 'House ACL lacks current owner receipt' USING ERRCODE='23514';
 END IF;
 UPDATE game_house_properties SET revision=revision+1,acl_revision=r.result_revision
  WHERE world_id=r.world_id AND house_key=r.house_key;
END $$;
CREATE OR REPLACE FUNCTION game_house_access_grant(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$
 SELECT CASE WHEN EXISTS(SELECT 1 FROM game_house_acl_entries WHERE world_id=p_world AND house_key=p_house
   AND list_key='subowner' AND character_id=p_character) THEN 'SUBOWNER'
  WHEN EXISTS(SELECT 1 FROM game_house_acl_entries WHERE world_id=p_world AND house_key=p_house
   AND list_key='guest' AND character_id=p_character) THEN 'GUEST' ELSE NULL END
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_acl_advance(UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_grant(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_management_acl_advance(UUID,NUMERIC),game_house_access_grant(UUID,TEXT,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_management_acl_advance(UUID,NUMERIC) TO oteryn_game_runtime;
CREATE FUNCTION game_house_management_acl_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE w UUID; h TEXT; k TEXT;
BEGIN
 w:=CASE WHEN TG_OP='DELETE' THEN OLD.world_id ELSE NEW.world_id END;
 h:=CASE WHEN TG_OP='DELETE' THEN OLD.house_key ELSE NEW.house_key END;
 k:=CASE WHEN TG_OP='DELETE' THEN OLD.list_key ELSE NEW.list_key END;
 IF NOT EXISTS(SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.world_id=w AND r.house_key=h AND r.list_key=k AND r.action_kind=1
   AND r.created_xact_id=pg_current_xact_id() AND r.result_revision=p.acl_revision)
  OR (SELECT count(*) FROM (SELECT 1 FROM game_house_acl_entries WHERE world_id=w AND house_key=h AND list_key=k LIMIT 201) bounded)>200 THEN
  RAISE EXCEPTION 'House ACL requires bounded same-transaction receipt' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' AND NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=w AND lifecycle=1) THEN
  RAISE EXCEPTION 'House ACL member must be active in this World' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_management_acl_proven AFTER INSERT OR DELETE ON game_house_acl_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_management_acl_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_acl_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_management_acl_proven() FROM PUBLIC;
