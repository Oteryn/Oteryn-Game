-- NEWS-HIST-CONTAINER-SORT-1: ordinal permutation in the existing item loop owner.
-- No new custody location or item mutation path. UPDATE is column-scoped and
-- admitted only by the exact old/new permutation in a same-transaction receipt.
CREATE TABLE game_item_container_sort_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation > 0),
    lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation > 0),
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation > 0),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest)=32),
    intent BYTEA NOT NULL CHECK (octet_length(intent)=20),
    permutation JSONB NOT NULL CHECK (jsonb_typeof(permutation)='array' AND jsonb_array_length(permutation)<=500),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id),
    -- One bounded sort per semantic transaction; also indexes trigger proof lookup.
    UNIQUE(created_xact_id)
);
CREATE TRIGGER game_item_container_sort_receipts_immutable BEFORE UPDATE OR DELETE
ON game_item_container_sort_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_container_sort_receipts_no_truncate BEFORE TRUNCATE
ON game_item_container_sort_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Preserve uniqueness at COMMIT while permitting a permutation in one UPDATE.
DO $$ DECLARE t TEXT; c TEXT; BEGIN
    FOREACH t IN ARRAY ARRAY['game_item_container_entries','game_item_nested_entries'] LOOP
        SELECT conname INTO STRICT c FROM pg_constraint
        WHERE conrelid=t::regclass AND contype='u'
          AND pg_get_constraintdef(oid)='UNIQUE (parent_item_instance_id, placement_ordinal)';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',t,c);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I UNIQUE(parent_item_instance_id,placement_ordinal) DEFERRABLE INITIALLY IMMEDIATE',t,t || '_sort_ordinals');
    END LOOP;
END $$;

CREATE FUNCTION game_item_container_sort_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE v_entry JSONB; v_count INT; BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'sort receipt must be current' USING ERRCODE='23514';
    END IF;
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=NEW.world_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'sort Character world' USING ERRCODE='23514'; END IF;
    IF (SELECT count(DISTINCT x->>'item') FROM jsonb_array_elements(NEW.permutation) x)<>jsonb_array_length(NEW.permutation) THEN
        RAISE EXCEPTION 'sort repeated item' USING ERRCODE='23514';
    END IF;
    FOR v_entry IN SELECT * FROM jsonb_array_elements(NEW.permutation) LOOP
        IF (SELECT owner_character FROM game_item_loop_owner((v_entry->>'item')::uuid)) IS DISTINCT FROM NEW.character_id THEN
            RAISE EXCEPTION 'sort requires carried custody' USING ERRCODE='23514';
        END IF;
        SELECT count(*) INTO v_count FROM (
            SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries
            UNION ALL
            SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries
        ) e WHERE e.item_instance_id=(v_entry->>'item')::uuid AND e.world_id=NEW.world_id
          AND e.parent_item_instance_id=(v_entry->>'parent')::uuid
          AND e.placement_ordinal=(v_entry->>'before')::numeric AND e.nested=(v_entry->>'nested')::boolean;
        IF v_count<>1 THEN RAISE EXCEPTION 'sort source changed' USING ERRCODE='23514'; END IF;
    END LOOP;
    -- Each parent's existing ordinal multiset must equal its successor multiset.
    IF EXISTS (
        (SELECT x->>'parent',x->>'before' FROM jsonb_array_elements(NEW.permutation) x
         EXCEPT ALL SELECT x->>'parent',x->>'after' FROM jsonb_array_elements(NEW.permutation) x)
        UNION ALL
        (SELECT x->>'parent',x->>'after' FROM jsonb_array_elements(NEW.permutation) x
         EXCEPT ALL SELECT x->>'parent',x->>'before' FROM jsonb_array_elements(NEW.permutation) x)
    ) THEN RAISE EXCEPTION 'sort must conserve ordinals' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_item_container_sort_admit BEFORE INSERT ON game_item_container_sort_receipts
FOR EACH ROW EXECUTE FUNCTION game_item_container_sort_admit();

CREATE FUNCTION game_item_container_sort_update() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
    IF (to_jsonb(NEW)-'placement_ordinal') IS DISTINCT FROM (to_jsonb(OLD)-'placement_ordinal')
       OR NOT EXISTS (
        SELECT 1 FROM game_item_container_sort_receipts r, jsonb_array_elements(r.permutation) x
        WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id
          AND x->>'item'=OLD.item_instance_id::text AND x->>'parent'=OLD.parent_item_instance_id::text
          AND (x->>'before')::numeric=OLD.placement_ordinal AND (x->>'after')::numeric=NEW.placement_ordinal
          AND (x->>'nested')::boolean=(TG_TABLE_NAME='game_item_nested_entries')
    ) THEN RAISE EXCEPTION 'ordinal update requires exact sort receipt' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
DROP TRIGGER game_item_container_entry_immutable ON game_item_container_entries;
CREATE TRIGGER game_item_container_entry_immutable BEFORE UPDATE ON game_item_container_entries
FOR EACH ROW EXECUTE FUNCTION game_item_container_sort_update();
DROP TRIGGER game_item_nested_entries_immutable ON game_item_nested_entries;
CREATE TRIGGER game_item_nested_entries_immutable BEFORE UPDATE ON game_item_nested_entries
FOR EACH ROW EXECUTE FUNCTION game_item_container_sort_update();

CREATE FUNCTION game_item_container_sort_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE v_entry JSONB; BEGIN
    FOR v_entry IN SELECT * FROM jsonb_array_elements(NEW.permutation) LOOP
        IF NOT EXISTS (
            SELECT 1 FROM (
                SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries
                UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
            ) e WHERE item_instance_id=(v_entry->>'item')::uuid AND parent_item_instance_id=(v_entry->>'parent')::uuid
                AND placement_ordinal=(v_entry->>'after')::numeric
        ) THEN RAISE EXCEPTION 'sort successor differs from receipt' USING ERRCODE='23514'; END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_container_sort_proven AFTER INSERT ON game_item_container_sort_receipts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_container_sort_proven();
REVOKE ALL ON game_item_container_sort_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_container_sort_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_item_container_sort_receipts TO oteryn_game_control;
GRANT UPDATE(placement_ordinal) ON game_item_container_entries,game_item_nested_entries TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_item_container_sort_admit(),game_item_container_sort_update(),game_item_container_sort_proven() FROM PUBLIC;
