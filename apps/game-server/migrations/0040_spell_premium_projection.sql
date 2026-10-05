-- LOCAL CANDIDATE PREMDEL0 consumer fence. These rows consume authenticated Platform facts;
-- no Game operation creates an entitlement or clears an observed conflict.
CREATE TABLE game_spell_premium_accounts (
 account_id UUID PRIMARY KEY,
 source_authority TEXT NOT NULL CHECK(octet_length(source_authority) BETWEEN 1 AND 128),
 authority_revision NUMERIC(20,0) NOT NULL CHECK(authority_revision BETWEEN 1 AND 18446744073709551615),
 fingerprint BYTEA NOT NULL CHECK(octet_length(fingerprint)=32),
 evidence BYTEA NOT NULL CHECK(octet_length(evidence) BETWEEN 1 AND 1024),
 conflicting BOOLEAN NOT NULL DEFAULT FALSE,
 recorded_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_spell_premium_entitlements (
 account_id UUID NOT NULL REFERENCES game_spell_premium_accounts(account_id),
 entitlement_id UUID NOT NULL,
 lifecycle_revision NUMERIC(20,0) NOT NULL CHECK(lifecycle_revision BETWEEN 1 AND 18446744073709551615),
 authority_revision NUMERIC(20,0) NOT NULL CHECK(authority_revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(account_id,entitlement_id)
);
CREATE TABLE game_spell_premium_history (
 account_id UUID NOT NULL REFERENCES game_spell_premium_accounts(account_id),
 authority_revision NUMERIC(20,0) NOT NULL CHECK(authority_revision BETWEEN 1 AND 18446744073709551615),
 fingerprint BYTEA NOT NULL CHECK(octet_length(fingerprint)=32),
 recorded_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 recorded_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(account_id,authority_revision)
);
CREATE TABLE game_spell_premium_security_audit (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 account_id UUID NOT NULL REFERENCES game_spell_premium_accounts(account_id),
 authority_revision NUMERIC(20,0) NOT NULL CHECK(authority_revision BETWEEN 1 AND 18446744073709551615),
 reason TEXT NOT NULL CHECK(reason IN('AUTHENTICATED_EQUIVOCATION','UNSUPPORTED_SEMANTICS','SOURCE_CHANGED','LIFECYCLE_ROLLBACK')),
 recorded_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_spell_premium_account_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
  IF NEW.account_id<>OLD.account_id OR NEW.source_authority<>OLD.source_authority
    OR NEW.authority_revision<OLD.authority_revision OR (OLD.conflicting AND NOT NEW.conflicting)
    OR (NEW.authority_revision=OLD.authority_revision AND (NEW.fingerprint<>OLD.fingerprint OR NEW.evidence<>OLD.evidence)) THEN
   RAISE EXCEPTION 'premium high water/source/conflict cannot roll back' USING ERRCODE='23514';
  END IF;
 END IF;
 NEW.recorded_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_spell_premium_account_guard BEFORE INSERT OR UPDATE ON game_spell_premium_accounts FOR EACH ROW EXECUTE FUNCTION game_spell_premium_account_guard();
CREATE FUNCTION game_spell_premium_account_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_premium_history h WHERE h.account_id=NEW.account_id
   AND h.authority_revision=NEW.authority_revision AND h.fingerprint=NEW.fingerprint) THEN
  RAISE EXCEPTION 'premium high water requires retained same-revision fingerprint' USING ERRCODE='23514';
 END IF;
 IF NEW.conflicting AND NOT EXISTS(SELECT 1 FROM game_spell_premium_security_audit a WHERE a.account_id=NEW.account_id) THEN
  RAISE EXCEPTION 'premium conflict requires durable security audit' USING ERRCODE='23514';
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_premium_account_proven AFTER INSERT OR UPDATE ON game_spell_premium_accounts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_premium_account_proven();
CREATE FUNCTION game_spell_premium_entitlement_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.account_id<>OLD.account_id OR NEW.entitlement_id<>OLD.entitlement_id
   OR NEW.lifecycle_revision<OLD.lifecycle_revision OR NEW.authority_revision<=OLD.authority_revision) THEN
  RAISE EXCEPTION 'premium entitlement lifecycle/authority cannot roll back' USING ERRCODE='23514';
 END IF; RETURN NEW;
END $$;
CREATE TRIGGER game_spell_premium_entitlement_guard BEFORE UPDATE ON game_spell_premium_entitlements FOR EACH ROW EXECUTE FUNCTION game_spell_premium_entitlement_guard();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_spell_premium_history','game_spell_premium_security_audit'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n||'_immutable',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_spell_premium_accounts','game_spell_premium_entitlements'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n||'_no_delete',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_spell_premium_accounts','game_spell_premium_entitlements','game_spell_premium_history','game_spell_premium_security_audit'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',n||'_no_truncate',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_spell_premium_account_guard','game_spell_premium_account_proven','game_spell_premium_entitlement_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
REVOKE ALL ON game_spell_premium_accounts,game_spell_premium_entitlements,game_spell_premium_history,game_spell_premium_security_audit FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_spell_premium_accounts,game_spell_premium_entitlements TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_spell_premium_history,game_spell_premium_security_audit TO oteryn_game_runtime;
GRANT SELECT ON game_spell_premium_accounts,game_spell_premium_entitlements,game_spell_premium_history,game_spell_premium_security_audit TO oteryn_game_control;
