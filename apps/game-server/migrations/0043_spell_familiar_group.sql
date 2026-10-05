-- Local candidate GAME-GROUP-1. Explicit ordinary Game owner initialization, never
-- absence-as-false. Account security and privileged Control authority remain separate.
-- No public privileged mutation and no CharacterRevision/XP/build/stance write.
CREATE TABLE game_character_source_group_receipts (
    source_session_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(source_session_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    revision NUMERIC(20,0) NOT NULL CHECK(revision=1),
    group_id INTEGER NOT NULL CHECK(group_id=1),
    account_type INTEGER NOT NULL CHECK(account_type=1),
    source_document JSONB NOT NULL CHECK(jsonb_typeof(source_document)='object' AND octet_length(source_document::text)<=8192),
    semantic_binding BYTEA NOT NULL CHECK(octet_length(semantic_binding)=32),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(character_id)
);
CREATE TABLE game_character_source_group (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    revision NUMERIC(20,0) NOT NULL CHECK(revision=1),
    source_session_id UUID NOT NULL UNIQUE REFERENCES game_character_source_group_receipts(source_session_id),
    group_id INTEGER NOT NULL CHECK(group_id=1),
    account_type INTEGER NOT NULL CHECK(account_type=1)
);
CREATE FUNCTION game_character_source_group_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'source Game group projection/receipt is immutable; privileged mutation requires its Control owner' USING ERRCODE='23514'; END; $$;
CREATE TRIGGER source_group_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_source_group_receipts FOR EACH ROW EXECUTE FUNCTION game_character_source_group_immutable();
CREATE TRIGGER source_group_projection_immutable BEFORE UPDATE OR DELETE ON game_character_source_group FOR EACH ROW EXECUTE FUNCTION game_character_source_group_immutable();
CREATE FUNCTION game_character_source_group_consistent() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_character_source_group_receipts r FULL JOIN game_character_source_group s USING(character_id)
        WHERE (r.character_id=NEW.character_id OR s.character_id=NEW.character_id)
        AND (r.character_id IS NULL OR s.character_id IS NULL OR r.source_session_id<>s.source_session_id
             OR r.revision<>s.revision OR r.group_id<>s.group_id OR r.account_type<>s.account_type)) THEN
        RAISE EXCEPTION 'source Game group receipt/projection mismatch' USING ERRCODE='23514';
    END IF;RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER source_group_receipt_consistent AFTER INSERT ON game_character_source_group_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_source_group_consistent();
CREATE CONSTRAINT TRIGGER source_group_projection_consistent AFTER INSERT ON game_character_source_group DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_source_group_consistent();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_source_group_immutable() SET search_path=%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_character_source_group_consistent() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_character_source_group_receipts,game_character_source_group FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_source_group_immutable(),game_character_source_group_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_source_group_receipts,game_character_source_group TO oteryn_game_runtime;
GRANT SELECT ON game_character_source_group_receipts,game_character_source_group TO oteryn_game_control;
