-- A return binds the inventory settlement to one occurrence mint and exact NPC command.
ALTER TABLE game_bank_robbery_bags
    ADD UNIQUE (world_id, reset_epoch, item_instance_id);

CREATE TABLE game_bank_robbery_returns (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    bag_id UUID NOT NULL UNIQUE,
    character_id UUID NOT NULL REFERENCES game_character_roots,
    npc_identity BYTEA NOT NULL CHECK (octet_length(npc_identity) = 16),
    npc_generation NUMERIC(20,0) NOT NULL CHECK (npc_generation BETWEEN 1 AND 18446744073709551615),
    npc_key TEXT NOT NULL,
    prompt_generation NUMERIC(20,0) NOT NULL CHECK (prompt_generation BETWEEN 1 AND 18446744073709551615),
    request_text TEXT NOT NULL CHECK (lower(btrim(request_text, ' ')) = 'yes' AND octet_length(request_text) <= 255),
    return_count BIGINT NOT NULL CHECK (return_count > 0),
    retirement UUID NOT NULL DEFAULT uuidv7() UNIQUE,
    PRIMARY KEY (game_session_id, command_id),
    UNIQUE (world_id, reset_epoch, character_id),
    UNIQUE (character_id, return_count),
    FOREIGN KEY (world_id, reset_epoch, bag_id)
        REFERENCES game_bank_robbery_bags(world_id, reset_epoch, item_instance_id),
    FOREIGN KEY (game_session_id, command_id)
        REFERENCES game_reward_claim_mint_receipts DEFERRABLE INITIALLY DEFERRED
);
REVOKE ALL ON game_bank_robbery_returns FROM PUBLIC;
GRANT SELECT, INSERT ON game_bank_robbery_returns TO oteryn_game_runtime;
GRANT SELECT ON game_bank_robbery_returns TO oteryn_game_control;

-- A pending Bank confirmation rejected at fresh settlement remains rejected on replay.
CREATE TABLE game_npc_bank_robbery_suspensions (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    PRIMARY KEY (game_session_id, command_id),
    FOREIGN KEY (game_session_id, command_id) REFERENCES game_npc_bank_confirmations
);
REVOKE ALL ON game_npc_bank_robbery_suspensions FROM PUBLIC;
GRANT SELECT, INSERT ON game_npc_bank_robbery_suspensions TO oteryn_game_runtime;
GRANT SELECT ON game_npc_bank_robbery_suspensions TO oteryn_game_control;

-- Reuse unit 1's retirement proof and custody guards. Only a receipt for
-- the exact current, carried bag may establish a return retirement.
CREATE FUNCTION game_bank_robbery_return_bag() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    occurrence_state SMALLINT;
    occurrence_city TEXT;
    custody TEXT;
BEGIN
    PERFORM 1 FROM game_world_reset_read_locked(NEW.world_id) WHERE reset_epoch = NEW.reset_epoch;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'robbery return requires the active save epoch' USING ERRCODE = '23514';
    END IF;
    SELECT state, city INTO occurrence_state, occurrence_city
        FROM game_bank_robbery_occurrences
        WHERE (world_id, reset_epoch) = (NEW.world_id, NEW.reset_epoch) FOR UPDATE;
    IF occurrence_state IS NULL OR occurrence_state NOT IN (2, 3)
      OR NOT coalesce(CASE occurrence_city
        WHEN 'thais' THEN NEW.npc_key IN ('oteryn:npc.naji', 'oteryn:npc.suzy')
        WHEN 'carlin' THEN NEW.npc_key = 'oteryn:npc.eva'
        WHEN 'venore' THEN NEW.npc_key = 'oteryn:npc.rokyn'
        WHEN 'ab_dendriel' THEN NEW.npc_key = 'oteryn:npc.finarfin'
      END, false) THEN
        RAISE EXCEPTION 'robbery return requires the affected bank' USING ERRCODE = '23514';
    END IF;
    SELECT c.kind INTO custody
        FROM game_item_instances i
        JOIN game_quest_callback_carried(NEW.character_id, NEW.world_id) c ON c.item = i.item_instance_id
        WHERE i.item_instance_id = NEW.bag_id AND i.world_id = NEW.world_id
          AND i.lifecycle = 1 AND i.quantity = 1 AND i.definition_production_key = 'oteryn:item.tibia.i13429'
          AND c.kind IN ('container', 'nested')
        FOR UPDATE OF i;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'robbery return requires current bag custody' USING ERRCODE = '23514';
    END IF;
    UPDATE game_bank_robbery_bags
        SET retired_transaction_id = NEW.retirement, retired_xid = pg_current_xact_id()
        WHERE item_instance_id = NEW.bag_id AND retired_transaction_id IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'robbery bag is already retired' USING ERRCODE = '23514';
    END IF;
    UPDATE game_item_instances
        SET quantity = 0, lifecycle = 2, state_revision = state_revision + 1,
            last_transaction_id = NEW.retirement
        WHERE item_instance_id = NEW.bag_id;
    IF custody = 'container' THEN
        DELETE FROM game_item_container_entries WHERE item_instance_id = NEW.bag_id;
    ELSE
        DELETE FROM game_item_nested_entries WHERE item_instance_id = NEW.bag_id;
    END IF;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'robbery bag location changed during return' USING ERRCODE = '23514';
    END IF;
    UPDATE game_bank_robbery_occurrences SET state = 3
        WHERE (world_id, reset_epoch) = (NEW.world_id, NEW.reset_epoch) AND state = 2;
    RETURN NEW;
END
$$;

CREATE FUNCTION game_bank_robbery_return_proven() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
        JOIN game_item_instances i USING (item_instance_id)
        WHERE (r.game_session_id, r.command_id, r.character_id) =
            (NEW.game_session_id, NEW.command_id, NEW.character_id)
          AND r.created_xact_id = pg_current_xact_id()
          AND r.quantity = 10 AND i.world_id = NEW.world_id
          AND i.definition_production_key = 'oteryn:item.tibia.i3035'
          AND r.claim_revision_ref = 'npc-program-r1'
          AND r.claim_production_key = 'oteryn:reward-claim.npc-program.' ||
            encode(sha256(convert_to('oteryn:npc-reward/bank-robbery/' ||
              replace(NEW.bag_id::text, '-', ''), 'UTF8')), 'hex')
    ) THEN
        RAISE EXCEPTION 'robbery return requires its same-transaction platinum MINT' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;

CREATE TRIGGER game_bank_robbery_return_bag BEFORE INSERT ON game_bank_robbery_returns
FOR EACH ROW EXECUTE FUNCTION game_bank_robbery_return_bag();
CREATE CONSTRAINT TRIGGER game_bank_robbery_return_proven AFTER INSERT ON game_bank_robbery_returns
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bank_robbery_return_proven();

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_bank_robbery_return_bag() SET search_path = pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_bank_robbery_return_proven() SET search_path = pg_catalog,%I,pg_temp', current_schema());
END
$migration$;
REVOKE ALL ON FUNCTION game_bank_robbery_return_bag() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_bank_robbery_return_proven() FROM PUBLIC;
