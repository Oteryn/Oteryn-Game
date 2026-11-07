-- Candidate HOUSE-ALETA-1: owner explicitly authorized Aleta alongside GUI.
-- House identity is World-global, never a Channel-local property copy.
-- Allocation/Premium eligibility belongs to the accepted acquisition owner;
-- this migration seeds no ownership and grants runtime no ownership writer.
CREATE TABLE game_house_ownership (
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL CHECK(octet_length(house_key)<=512 AND house_key~'^oteryn:content\.house\.[a-z0-9_]+$'),
    owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 1 AND 18446744073709551615),
    acquisition_receipt_ref UUID NOT NULL CHECK(game_character_is_uuid_v7(acquisition_receipt_ref)),
    PRIMARY KEY(world_id,house_key)
);
CREATE FUNCTION game_house_owner_in_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.owner_character_id AND r.world_id=NEW.world_id AND r.lifecycle=1) THEN
        RAISE EXCEPTION 'House owner must be an active Character of this World' USING ERRCODE='23514';
    END IF;
    IF TG_OP='UPDATE' AND (NEW.world_id<>OLD.world_id OR NEW.house_key<>OLD.house_key OR NEW.ownership_revision<>OLD.ownership_revision+1 OR NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref) THEN
        RAISE EXCEPTION 'House ownership requires an exact acquisition successor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_owner_in_world BEFORE INSERT OR UPDATE ON game_house_ownership FOR EACH ROW EXECUTE FUNCTION game_house_owner_in_world();
CREATE TABLE game_house_acl_receipts (
    receipt_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(receipt_id)),
    world_id UUID NOT NULL, house_key TEXT NOT NULL,
    list_id BIGINT NOT NULL CHECK(list_id BETWEEN -2 AND 4294967295 AND list_id<>0),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
    ownership_revision NUMERIC(20,0) NOT NULL,
    revision_before NUMERIC(20,0) NOT NULL CHECK(revision_before>=0),
    revision_after NUMERIC(20,0) NOT NULL CHECK(revision_after=revision_before+1),
    allow_everyone BOOLEAN NOT NULL,
    members UUID[] NOT NULL CHECK(cardinality(members)<=100),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key)
);
CREATE TABLE game_house_acl (
    world_id UUID NOT NULL, house_key TEXT NOT NULL,
    -- -1 Guest, -2 Subowner; positive source door number in this House.
    list_id BIGINT NOT NULL CHECK(list_id BETWEEN -2 AND 4294967295 AND list_id<>0),
    revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
    allow_everyone BOOLEAN NOT NULL,
    members UUID[] NOT NULL CHECK(cardinality(members)<=100),
    last_receipt UUID NOT NULL REFERENCES game_house_acl_receipts(receipt_id),
    PRIMARY KEY(world_id,house_key,list_id),
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key)
);
CREATE TABLE game_house_editors (
    editor_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(editor_id)),
    world_id UUID NOT NULL, house_key TEXT NOT NULL,
    list_id BIGINT NOT NULL CHECK(list_id BETWEEN -2 AND 4294967295 AND list_id<>0),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    opening_command_id NUMERIC(20,0) NOT NULL CHECK(opening_command_id>0),
    ownership_revision NUMERIC(20,0) NOT NULL,
    acl_revision NUMERIC(20,0) NOT NULL CHECK(acl_revision>=0),
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
    content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(game_session_id,opening_command_id),
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key)
);
CREATE FUNCTION game_house_acl_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous NUMERIC(20,0);
BEGIN
    previous:=CASE WHEN TG_OP='INSERT' THEN 0 ELSE OLD.revision END;
    IF NEW.revision<>previous+1 OR (TG_OP='UPDATE' AND (NEW.world_id,NEW.house_key,NEW.list_id)<>(OLD.world_id,OLD.house_key,OLD.list_id)) OR NOT EXISTS(
        SELECT 1 FROM game_house_acl_receipts r JOIN game_house_ownership o USING(world_id,house_key)
        WHERE r.receipt_id=NEW.last_receipt AND r.world_id=NEW.world_id AND r.house_key=NEW.house_key AND r.list_id=NEW.list_id
          AND r.ownership_revision=o.ownership_revision AND r.revision_before=previous AND r.revision_after=NEW.revision
          AND r.allow_everyone=NEW.allow_everyone AND r.members=NEW.members AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'House ACL requires same-transaction exact owner receipt' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM unnest(NEW.members) member WHERE NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=member AND r.world_id=NEW.world_id AND r.lifecycle=1)) THEN
        RAISE EXCEPTION 'House ACL member must be an active Character of this World' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_acl_proven AFTER INSERT OR UPDATE ON game_house_acl DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_acl_proven();
CREATE FUNCTION game_house_acl_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'House ACL receipts are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_house_acl_receipt_immutable BEFORE UPDATE OR DELETE ON game_house_acl_receipts FOR EACH ROW EXECUTE FUNCTION game_house_acl_receipt_immutable();
REVOKE ALL ON game_house_ownership,game_house_acl,game_house_acl_receipts,game_house_editors FROM PUBLIC;
GRANT SELECT ON game_house_ownership TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_house_acl,game_house_editors TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_house_acl_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_house_ownership TO oteryn_game_control;
-- Actual administration/admission projection, distinct from player ACL edits.
-- No default or inferred staff/CanEditHouses row is created by Aleta.
CREATE TABLE game_character_house_privileges (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    can_edit_houses BOOLEAN NOT NULL,
    source_policy TEXT NOT NULL CHECK(length(source_policy)>0),
    source_policy_digest BYTEA NOT NULL CHECK(octet_length(source_policy_digest)=32)
);
REVOKE ALL ON game_character_house_privileges FROM PUBLIC;
GRANT SELECT ON game_character_house_privileges TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_house_privileges TO oteryn_game_control;
CREATE FUNCTION game_house_editor_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.consumed OR NOT NEW.consumed OR (to_jsonb(NEW)-'consumed')<>(to_jsonb(OLD)-'consumed') THEN
        RAISE EXCEPTION 'House editor token can only be consumed once' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_editor_guard BEFORE UPDATE ON game_house_editors FOR EACH ROW EXECUTE FUNCTION game_house_editor_guard();
CREATE FUNCTION game_house_acl_stamp_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_house_acl_stamp_receipt BEFORE INSERT ON game_house_acl_receipts FOR EACH ROW EXECUTE FUNCTION game_house_acl_stamp_receipt();
DO $$ DECLARE name TEXT; BEGIN
    FOREACH name IN ARRAY ARRAY['game_house_owner_in_world','game_house_acl_proven','game_house_acl_receipt_immutable','game_house_editor_guard','game_house_acl_stamp_receipt'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $$;
