-- I10a: the durable fluid sub-type of a carried fluid container.
--
-- Crystal (201189f8) keeps a fluid container's content in the item's sub-type
-- (`data/scripts/actions/items/fluids.lua`: `item:transform(item:getId(), type)`, 0 = empty).
-- The sub-type lives beside the item rather than in `game_item_instances`, so the instance
-- guard's change proofs stay as they are: no row is the empty container. The row follows the
-- item identity through every move; an item never minted with a row is empty.
--
-- The runtime writes a fill or an empty only through `commit_item_fluid`, which runs under the
-- holder's current item fence and records one transition per changed container and CommandRef.

CREATE TABLE game_item_fluid_states (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    -- The client fluid sub-type (Crystal `item.type`), 1-255; 0 (empty) is no row.
    fluid_subtype SMALLINT NOT NULL CHECK (fluid_subtype BETWEEN 1 AND 255),
    last_transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(last_transaction_id)),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

CREATE TABLE game_item_fluid_transitions (
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    fluid_before SMALLINT NOT NULL CHECK (fluid_before BETWEEN 0 AND 255),
    fluid_after SMALLINT NOT NULL CHECK (fluid_after BETWEEN 0 AND 255),
    CHECK (fluid_before <> fluid_after),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    -- One command changes one container, or two when it pours one into the other.
    PRIMARY KEY (transaction_id, item_instance_id),
    UNIQUE (game_session_id, command_id, item_instance_id)
);

-- A state row changes only together with the transition that names it, in the same transaction.
CREATE FUNCTION game_item_fluid_state_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_item UUID := CASE WHEN TG_OP = 'DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
    v_transaction UUID := CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.last_transaction_id END;
    v_after SMALLINT := CASE WHEN TG_OP = 'DELETE' THEN 0 ELSE NEW.fluid_subtype END;
BEGIN
    IF TG_OP = 'UPDATE' AND (NEW.item_instance_id, NEW.world_id) IS DISTINCT FROM
                            (OLD.item_instance_id, OLD.world_id) THEN
        RAISE EXCEPTION 'ITEM_FLUID_IDENTITY' USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fluid_transitions t
         WHERE t.item_instance_id = v_item
           AND (v_transaction IS NULL OR t.transaction_id = v_transaction)
           AND t.fluid_after = v_after
           AND t.created_xact_id = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'ITEM_FLUID_UNPROVEN' USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

-- The proof's transaction is the inserting one, whatever the writer supplies.
CREATE FUNCTION game_item_fluid_transition_stamp() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_xact_id := pg_current_xact_id();
    RETURN NEW;
END $$;

CREATE TRIGGER game_item_fluid_transition_stamp
    BEFORE INSERT ON game_item_fluid_transitions
    FOR EACH ROW EXECUTE FUNCTION game_item_fluid_transition_stamp();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_item_fluid_transition_stamp() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_fluid_state_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

CREATE TRIGGER game_item_fluid_state_proven
    BEFORE INSERT OR UPDATE OR DELETE ON game_item_fluid_states
    FOR EACH ROW EXECUTE FUNCTION game_item_fluid_state_proven();

REVOKE ALL ON game_item_fluid_states, game_item_fluid_transitions FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_fluid_state_proven(), game_item_fluid_transition_stamp() FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_item_fluid_states TO oteryn_game_runtime;
GRANT UPDATE (fluid_subtype, last_transaction_id) ON game_item_fluid_states
    TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_item_fluid_transitions TO oteryn_game_runtime;
GRANT SELECT ON game_item_fluid_states, game_item_fluid_transitions TO oteryn_game_control;
