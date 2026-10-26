-- HOUSE-TRANSFER-1. Additive transfer receipts and the separate exit direction.
-- 0074's entry-only CHECK and House session proofs, and 0149's bed fences, stay
-- unchanged. A prepared receipt is source authority evidence only in its creating
-- transaction; committed receipts are history and never current authority.
CREATE TABLE game_house_transfer_receipts (
    transfer_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(transfer_id)),
    direction SMALLINT NOT NULL CHECK (direction IN (1,2)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    account_id UUID NOT NULL,
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    house_key TEXT NOT NULL CHECK (game_house_scope_key_valid(house_key)),
    origin_channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(origin_channel_id)),
    source_game_session_id UUID NOT NULL UNIQUE
        REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    source_connection_generation NUMERIC(20,0) NOT NULL
        CHECK (source_connection_generation BETWEEN 1 AND 18446744073709551615),
    source_character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (source_character_lease_generation BETWEEN 1 AND 18446744073709551614),
    source_scope_generation NUMERIC(20,0) NOT NULL
        CHECK (source_scope_generation BETWEEN 1 AND 18446744073709551615),
    source_transport BYTEA NOT NULL CHECK (octet_length(source_transport)=16),
    node_id UUID NOT NULL CHECK (game_node_is_uuid_v7(node_id)),
    node_registration_revision NUMERIC(20,0) NOT NULL
        CHECK (node_registration_revision BETWEEN 1 AND 18446744073709551615),
    destination_game_session_id UUID NOT NULL UNIQUE
        CHECK (game_node_is_uuid_v7(destination_game_session_id)),
    destination_scope_generation NUMERIC(20,0) NOT NULL
        CHECK (destination_scope_generation BETWEEN 1 AND 18446744073709551615),
    selection INTEGER[] NOT NULL CHECK (
        cardinality(selection) BETWEEN 1 AND 128 AND 86=ANY(selection)),
    tile_x INTEGER NOT NULL,
    tile_y INTEGER NOT NULL,
    tile_floor SMALLINT NOT NULL,
    state SMALLINT NOT NULL CHECK (state IN (1,2)),
    acknowledgement BYTEA NULL CHECK (octet_length(acknowledgement) BETWEEN 1 AND 2048),
    prepared_xact XID8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((state=2)=(acknowledgement IS NOT NULL)),
    CHECK (source_game_session_id<>destination_game_session_id)
);

CREATE FUNCTION game_house_transfer_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    s game_durability_reconnect_sessions%ROWTYPE;
    source_key BYTEA;
    destination_key BYTEA;
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'transfer receipts are retained' USING ERRCODE='23514';
    END IF;
    IF TG_OP='UPDATE' THEN
        IF OLD.state<>1 OR NEW.state<>2
           OR OLD.prepared_xact<>pg_current_xact_id()
           OR (to_jsonb(NEW)-'state'-'acknowledgement') IS DISTINCT FROM
              (to_jsonb(OLD)-'state'-'acknowledgement') THEN
            RAISE EXCEPTION 'a transfer only commits in its fenced transaction' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.state<>1 OR NEW.acknowledgement IS NOT NULL THEN
        RAISE EXCEPTION 'a transfer starts prepared' USING ERRCODE='23514';
    END IF;
    NEW.prepared_xact:=pg_current_xact_id();
    SELECT * INTO s FROM game_durability_reconnect_sessions
        WHERE game_session_id=NEW.source_game_session_id FOR UPDATE;
    IF NOT FOUND OR s.character_id<>NEW.character_id OR s.account_id<>NEW.account_id
       OR s.world_id<>NEW.world_id OR s.runtime_scope_world_id<>NEW.world_id
       OR s.session_state NOT IN (1,2)
       OR s.current_generation<>NEW.source_connection_generation
       OR s.character_lease_generation<>NEW.source_character_lease_generation
       OR s.scope_ownership_generation<>NEW.source_scope_generation
       OR s.current_transport_ref IS DISTINCT FROM NEW.source_transport THEN
        RAISE EXCEPTION 'transfer source session fence failed' USING ERRCODE='23514';
    END IF;
    IF NEW.direction=1 THEN
        IF s.runtime_scope_kind<>1 OR s.runtime_scope_channel_id IS DISTINCT FROM NEW.origin_channel_id
           OR s.runtime_scope_house_key IS NOT NULL OR s.runtime_scope_instance_id IS NOT NULL THEN
            RAISE EXCEPTION 'entry requires its origin Channel' USING ERRCODE='23514';
        END IF;
        source_key:='\x01'::BYTEA||uuid_send(NEW.world_id)||uuid_send(NEW.origin_channel_id);
        destination_key:=game_house_scope_key(NEW.world_id,NEW.house_key);
    ELSE
        IF s.runtime_scope_kind<>2 OR s.runtime_scope_house_key IS DISTINCT FROM NEW.house_key
           OR s.runtime_scope_instance_id IS DISTINCT FROM game_house_scope_instance_id(NEW.world_id,NEW.house_key)
           OR s.runtime_scope_channel_id IS NOT NULL
           OR s.origin_channel_id IS DISTINCT FROM NEW.origin_channel_id THEN
            RAISE EXCEPTION 'exit requires the named House and origin' USING ERRCODE='23514';
        END IF;
        source_key:=game_house_scope_key(NEW.world_id,NEW.house_key);
        destination_key:='\x01'::BYTEA||uuid_send(NEW.world_id)||uuid_send(NEW.origin_channel_id);
    END IF;
    -- Both assignment rows are independently current, same-node, and held until commit.
    PERFORM 1 FROM game_runtime_scope_assignments a
        WHERE a.scope_key IN (source_key,destination_key)
        ORDER BY a.scope_key FOR SHARE;
    IF NOT EXISTS (SELECT 1 FROM game_runtime_scope_assignments a
        WHERE a.scope_key=source_key AND a.state=1
          AND a.ownership_generation=NEW.source_scope_generation
          AND a.holder_node_id=NEW.node_id
          AND a.holder_registration_revision=NEW.node_registration_revision)
       OR NOT EXISTS (SELECT 1 FROM game_runtime_scope_assignments a
        WHERE a.scope_key=destination_key AND a.state=1
          AND a.ownership_generation=NEW.destination_scope_generation
          AND a.holder_node_id=NEW.node_id
          AND a.holder_registration_revision=NEW.node_registration_revision) THEN
        RAISE EXCEPTION 'transfer assignment fence failed' USING ERRCODE='23514';
    END IF;
    -- Session authority cannot outlive independent account/Character admission claims.
    PERFORM 1 FROM game_durability_admission_character_guards c
        JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
        WHERE c.character_id=NEW.character_id AND c.world_id=NEW.world_id
          AND c.account_id=NEW.account_id AND c.eligible
          AND c.lease_generation=NEW.source_character_lease_generation
          AND c.holder_game_session_id=NEW.source_game_session_id
          AND a.presence_character_id=NEW.character_id
          AND a.holder_game_session_id=NEW.source_game_session_id
        FOR SHARE OF c,a;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'transfer admission claims failed' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_transfer_receipt_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_house_transfer_receipts FOR EACH ROW EXECUTE FUNCTION game_house_transfer_receipt_guard();

CREATE FUNCTION game_house_transfer_commit_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    r game_house_transfer_receipts%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_house_transfer_receipts WHERE transfer_id=NEW.transfer_id;
    IF r.state<>2 OR NOT EXISTS (
        SELECT 1 FROM game_durability_reconnect_sessions s
        WHERE s.game_session_id=r.source_game_session_id AND s.session_state=3
          AND s.current_transport_ref IS NULL)
       OR NOT EXISTS (
        SELECT 1 FROM game_durability_reconnect_sessions d
        WHERE d.game_session_id=r.destination_game_session_id
          AND d.character_id=r.character_id AND d.account_id=r.account_id AND d.world_id=r.world_id
          AND d.runtime_scope_world_id=r.world_id
          AND d.character_lease_generation=r.source_character_lease_generation+1
          AND d.scope_ownership_generation=r.destination_scope_generation
          AND d.current_generation=1 AND d.current_transport_ref=r.source_transport
          AND d.session_state IN (1,2)
          AND ((r.direction=1 AND d.runtime_scope_kind=2
                AND d.runtime_scope_house_key=r.house_key
                AND d.runtime_scope_instance_id=game_house_scope_instance_id(r.world_id,r.house_key)
                AND d.origin_channel_id=r.origin_channel_id)
            OR (r.direction=2 AND d.runtime_scope_kind=1
                AND d.runtime_scope_channel_id=r.origin_channel_id
                AND d.runtime_scope_house_key IS NULL AND d.runtime_scope_instance_id IS NULL))) THEN
        RAISE EXCEPTION 'transfer requires terminal source and exact destination' USING ERRCODE='23514';
    END IF;
    IF r.direction=1 AND NOT EXISTS (
        SELECT 1 FROM game_house_scope_handoffs h WHERE h.handoff_id=r.transfer_id
          AND h.state IN (2,3) AND h.destination_game_session_id=r.destination_game_session_id
          AND h.source_game_session_id=r.source_game_session_id) THEN
        RAISE EXCEPTION 'entry transfer requires the native handoff' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_transfer_commit_proven AFTER INSERT OR UPDATE
    ON game_house_transfer_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_house_transfer_commit_proven();
CREATE TRIGGER game_house_transfer_no_truncate BEFORE TRUNCATE ON game_house_transfer_receipts
    EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_house_transfer_receipts FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_house_transfer_receipts TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_house_transfer_receipt_guard(),game_house_transfer_commit_proven() FROM PUBLIC;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_transfer_receipt_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_house_transfer_commit_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
-- Rollback is a forward migration disabling new transfers. Retain receipts and
-- session-use history; neither terminal source nor committed destination is undone.
