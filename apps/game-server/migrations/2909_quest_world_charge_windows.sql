-- Source-qualified shared charge windows are changed by the Channel quest writer.
-- A charge debit commits alongside the selected tool burn and Character quest receipt.
CREATE TABLE game_quest_world_charge_windows (
    world_id UUID NOT NULL,
    map_revision TEXT NOT NULL CHECK (octet_length(map_revision) BETWEEN 1 AND 128),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    charge_group TEXT NOT NULL CHECK (octet_length(charge_group) BETWEEN 1 AND 128),
    opening_session UUID NOT NULL,
    opening_command NUMERIC(20,0) NOT NULL CHECK (opening_command BETWEEN 1 AND 18446744073709551615),
    ready_at BIGINT NOT NULL CHECK (ready_at > 0),
    expires_at BIGINT NOT NULL CHECK (expires_at > ready_at),
    remaining SMALLINT NOT NULL CHECK (remaining BETWEEN 0 AND 64),
    PRIMARY KEY (world_id, map_revision, reset_epoch, charge_group)
);
REVOKE ALL ON game_quest_world_charge_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_quest_world_charge_windows TO oteryn_game_runtime;

-- Volatile source projections become actual items only when a current MOVE
-- takes the exact occurrence. The append-only receipt doubles as its replay key.
CREATE TABLE game_quest_temporary_pickups (
    occurrence BYTEA PRIMARY KEY CHECK (octet_length(occurrence)=32),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    item_instance_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(item_instance_id)),
    world_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots,
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    definition_key TEXT NOT NULL CHECK (octet_length(definition_key) BETWEEN 1 AND 192),
    definition_revision TEXT NOT NULL CHECK (octet_length(definition_revision) BETWEEN 1 AND 192),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256)=32),
    occurred_at BIGINT NOT NULL CHECK (occurred_at>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (game_session_id,command_id)
);

CREATE FUNCTION game_quest_temporary_pickup_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_quest_temporary_pickups%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_quest_temporary_pickups WHERE item_instance_id=NEW.item_instance_id;
    IF r.item_instance_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
        OR NOT EXISTS (SELECT 1 FROM game_item_instances i
            WHERE (i.item_instance_id,i.world_id,i.definition_family,i.definition_production_key,
                   i.definition_revision_ref,i.quantity,i.lifecycle,i.state_revision,i.minted_transaction_id)
                =(r.item_instance_id,r.world_id,'Item',r.definition_key,r.definition_revision,1,1,1,r.transaction_id))
        OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
        OR NOT EXISTS (SELECT 1 FROM game_item_container_entries e
            JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id
              AND b.character_id=e.character_id AND b.world_id=e.world_id
            WHERE (e.item_instance_id,e.world_id,e.character_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
                =(r.item_instance_id,r.world_id,r.character_id,r.parent_item_instance_id,r.placement_ordinal,r.transaction_id))
        OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
            WHERE (a.item_instance_id,a.transaction_id,a.event_id,a.occurred_at,a.envelope_sha256)
                =(r.item_instance_id,r.transaction_id,r.event_id,r.occurred_at,r.envelope_sha256))
    THEN
        RAISE EXCEPTION 'temporary quest pickup lacks exact Item custody and audit' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER game_quest_temporary_pickup_complete
AFTER INSERT ON game_quest_temporary_pickups DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_quest_temporary_pickup_complete();
CREATE TRIGGER game_quest_temporary_pickup_stamp BEFORE INSERT ON game_quest_temporary_pickups
FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_quest_temporary_pickup_immutable BEFORE UPDATE OR DELETE ON game_quest_temporary_pickups
FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_temporary_pickup_no_truncate BEFORE TRUNCATE ON game_quest_temporary_pickups
EXECUTE FUNCTION game_item_reject_truncate();

DO $migration$
DECLARE name TEXT; body TEXT; start_at INT; arm TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_placement_proven'] LOOP
        body := pg_get_functiondef((name||'()')::regprocedure);
        start_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF start_at=0 THEN RAISE EXCEPTION 'unexpected temporary quest pickup guard: %',name; END IF;
        arm := $arm$
            IF TG_TABLE_NAME IN ('game_item_instances','game_item_container_entries')
                AND EXISTS (SELECT 1 FROM game_quest_temporary_pickups
                    WHERE item_instance_id=NEW.item_instance_id AND created_xact_id=pg_current_xact_id())
            THEN RETURN NULL; END IF;
$arm$;
        EXECUTE overlay(body PLACING E'BEGIN\n'||arm FROM start_at FOR 5);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_quest_temporary_pickup_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON game_quest_temporary_pickups FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_temporary_pickup_complete() FROM PUBLIC;
GRANT SELECT, INSERT ON game_quest_temporary_pickups TO oteryn_game_runtime;
