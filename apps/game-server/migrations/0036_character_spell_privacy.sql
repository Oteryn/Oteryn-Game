-- Candidate EXIVA-OWNER-1. Preferences are target-owned; no privacy or staff default is seeded.
-- Source fields mirror pinned Canary Player::ExivaRestrictions. Missing rows remain Unknown.
CREATE TABLE game_character_spell_privacy (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
    preferences JSONB NOT NULL CHECK(jsonb_typeof(preferences)='object'),
    last_receipt UUID NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_character_spell_privacy_receipts (
    receipt_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(receipt_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
    revision_before NUMERIC(20,0) NOT NULL CHECK(revision_before>=0),
    revision_after NUMERIC(20,0) NOT NULL CHECK(revision_after=revision_before+1),
    preferences JSONB NOT NULL CHECK(jsonb_typeof(preferences)='object'),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id)
);
ALTER TABLE game_character_spell_privacy ADD CONSTRAINT game_spell_privacy_receipt_fk
FOREIGN KEY(last_receipt) REFERENCES game_character_spell_privacy_receipts(receipt_id);
-- Security access is a separate administration/admission projection. The preference
-- writer has no API to grant staff access. No missing role is treated as ordinary access.
CREATE TABLE game_character_spell_access (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    staff_access BOOLEAN NOT NULL,
    source_policy TEXT NOT NULL CHECK(length(source_policy)>0),
    source_policy_digest BYTEA NOT NULL CHECK(octet_length(source_policy_digest)=32)
);
CREATE FUNCTION game_spell_privacy_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' AND NEW.revision<>1 THEN
        RAISE EXCEPTION 'privacy starts with an explicit first receipt' USING ERRCODE='23514';
    ELSIF TG_OP='UPDATE' AND (NEW.character_id<>OLD.character_id OR NEW.revision<>OLD.revision+1
            OR NEW.last_receipt=OLD.last_receipt) THEN
        RAISE EXCEPTION 'privacy needs exact receipt-bound successor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_spell_privacy_guard BEFORE INSERT OR UPDATE ON game_character_spell_privacy
FOR EACH ROW EXECUTE FUNCTION game_spell_privacy_guard();
CREATE FUNCTION game_spell_privacy_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE before_revision NUMERIC(20,0);
BEGIN
    before_revision := CASE WHEN TG_OP='INSERT' THEN 0 ELSE OLD.revision END;
    IF NOT EXISTS(SELECT 1 FROM game_character_spell_privacy_receipts r
        WHERE r.receipt_id=NEW.last_receipt AND r.character_id=NEW.character_id
          AND r.revision_before=before_revision AND r.revision_after=NEW.revision
          AND r.preferences=NEW.preferences AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'privacy lacks same-transaction owner receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_privacy_proven AFTER INSERT OR UPDATE ON game_character_spell_privacy
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_privacy_proven();
CREATE FUNCTION game_spell_privacy_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'privacy receipts are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_spell_privacy_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_spell_privacy_receipts
FOR EACH ROW EXECUTE FUNCTION game_spell_privacy_receipt_immutable();
-- Runtime owns target preference writes, never staff projection grants.
REVOKE ALL ON game_character_spell_privacy,game_character_spell_privacy_receipts,game_character_spell_access FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_spell_privacy TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_spell_privacy_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_spell_access TO oteryn_game_runtime;
GRANT SELECT ON game_character_spell_privacy,game_character_spell_privacy_receipts TO oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_character_spell_access TO oteryn_game_control;
CREATE FUNCTION game_spell_privacy_stamp_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_spell_privacy_stamp_receipt BEFORE INSERT ON game_character_spell_privacy_receipts FOR EACH ROW EXECUTE FUNCTION game_spell_privacy_stamp_receipt();
DO $$ DECLARE name TEXT; BEGIN
    FOREACH name IN ARRAY ARRAY['game_spell_privacy_guard','game_spell_privacy_proven','game_spell_privacy_receipt_immutable','game_spell_privacy_stamp_receipt'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $$;
