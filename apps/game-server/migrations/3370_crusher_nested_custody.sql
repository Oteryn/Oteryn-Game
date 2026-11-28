-- Bind nested Crusher custody to the existing craft receipt and timed write.
DO $migration$
DECLARE
    previous TEXT;
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous
    FROM pg_constraint
    WHERE conrelid='game_item_craft_records'::regclass
      AND conname='game_item_craft_event_source_location_check';

    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
    EXECUTE format($constraint$
        ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check
        CHECK ((%s) OR (
            recipe='oteryn:gem-atelier.crush' AND consume_source
            AND jsonb_typeof(event_source_location)='object'
            AND octet_length(event_source_location::text) BETWEEN 2 AND 4096
            AND event_source_location ?& ARRAY['item_instance_id','parent_item_instance_id',
                'world_id','placement_ordinal','placed_transaction_id']
        ))$constraint$, previous);
END $migration$;

CREATE TABLE game_item_crusher_nested_tool_proofs (
    craft_transaction_id UUID PRIMARY KEY REFERENCES game_item_craft_records(transaction_id),
    timed_transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(timed_transaction_id)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    location JSONB NOT NULL CHECK (jsonb_typeof(location)='object'
        AND octet_length(location::text) BETWEEN 2 AND 4096
        AND location ?& ARRAY['item_instance_id','parent_item_instance_id',
            'world_id','placement_ordinal','placed_transaction_id']),
    CHECK (timed_transaction_id<>craft_transaction_id)
);

CREATE INDEX game_item_crusher_nested_tool_item
ON game_item_crusher_nested_tool_proofs(item_instance_id);

CREATE FUNCTION game_item_crusher_nested_tool_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_craft_records r
        JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        JOIN game_item_nested_entries e USING (item_instance_id)
        JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
        WHERE r.transaction_id=NEW.craft_transaction_id
          AND r.created_xact_id=pg_current_xact_id()
          AND r.recipe='oteryn:gem-atelier.crush' AND r.consume_source
          AND r.source_item_instance_id<>i.item_instance_id
          AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i46627'
          AND i.lifecycle=1 AND i.quantity=1 AND i.world_id=r.world_id AND e.world_id=r.world_id
          AND to_jsonb(e)=NEW.location AND owner.owner_character=r.character_id
          AND owner.depth BETWEEN 1 AND 7
          AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries child
                          WHERE child.parent_item_instance_id=i.item_instance_id)
        FOR UPDATE OF i,e
    ) THEN
        RAISE EXCEPTION 'Crusher tool must be a nested leaf held by the craft owner'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_item_crusher_nested_tool_admit BEFORE INSERT ON game_item_crusher_nested_tool_proofs
FOR EACH ROW EXECUTE FUNCTION game_item_crusher_nested_tool_admit();

-- At checkpoint the recorded edge must still exist. Expiry invokes this same
-- read before deleting the edge and proves its removal at commit.
CREATE FUNCTION game_item_crusher_nested_tool_held(p_item UUID,p_holder UUID,p_transaction UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_item_crusher_nested_tool_proofs p
        JOIN game_item_craft_records r ON r.transaction_id=p.craft_transaction_id
        JOIN game_item_nested_entries e ON e.item_instance_id=p.item_instance_id
        JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true
        WHERE p.timed_transaction_id=p_transaction AND p.item_instance_id=p_item
          AND r.created_xact_id=pg_current_xact_id() AND r.character_id=p_holder
          AND e.world_id=r.world_id AND to_jsonb(e)=p.location
          AND owner.owner_character=p_holder AND owner.depth BETWEEN 1 AND 7
    );
$$;

CREATE FUNCTION game_item_crusher_nested_tool_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_craft_records r
        JOIN game_item_timed_state_writes w ON w.item_instance_id=NEW.item_instance_id
            AND w.transaction_id=NEW.timed_transaction_id AND w.created_xact_id=r.created_xact_id
        JOIN game_item_instances i ON i.item_instance_id=w.item_instance_id
        WHERE r.transaction_id=NEW.craft_transaction_id AND r.created_xact_id=pg_current_xact_id()
          AND w.holder_character_id=r.character_id AND i.world_id=r.world_id
          AND w.definition_before_family='Item' AND w.definition_after_family='Item'
          AND w.definition_before_production_key='oteryn:item.tibia.i46627'
          AND w.definition_after_production_key=w.definition_before_production_key
          AND w.definition_after_revision_ref=w.definition_before_revision_ref
          AND w.remaining_ms_before IS NULL AND w.remaining_ms_after IS NULL
          AND w.deadline_before IS NULL AND w.deadline_after IS NULL
          AND (
            (w.cause=1 AND w.charges_after=w.charges_before-1 AND i.lifecycle=1 AND i.quantity=1
             AND game_item_crusher_nested_tool_held(i.item_instance_id,r.character_id,w.transaction_id))
            OR (w.cause=3 AND w.charges_before=1 AND i.lifecycle=2 AND i.quantity=0
                AND i.last_transaction_id=w.transaction_id
                AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries e
                                WHERE e.item_instance_id=i.item_instance_id))
          )
    ) THEN
        RAISE EXCEPTION 'nested Crusher craft must spend one charge in the same transaction'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_crusher_nested_tool_complete AFTER INSERT
ON game_item_crusher_nested_tool_proofs DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_item_crusher_nested_tool_complete();

DO $migration$
DECLARE
    body TEXT;
    anchor TEXT;
    begin_at INT;
    signature TEXT;
BEGIN
    body := pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure);
    anchor := 'OR game_item_dispenser_checkpoint_held(NEW.item_instance_id,NEW.holder_character_id,NEW.transaction_id)';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing timed checkpoint custody guard'; END IF;
    EXECUTE replace(body,anchor,$arm$OR (NEW.cause=1 AND game_item_crusher_nested_tool_held(
            NEW.item_instance_id,NEW.holder_character_id,NEW.transaction_id))
        $arm$||anchor);

    body := pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing nested custody guard'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF TG_OP='DELETE' AND EXISTS (
        SELECT 1 FROM game_item_crusher_nested_tool_proofs p
        JOIN game_item_craft_records r ON r.transaction_id=p.craft_transaction_id
        JOIN game_item_timed_state_writes w ON w.transaction_id=p.timed_transaction_id
            AND w.item_instance_id=p.item_instance_id AND w.created_xact_id=r.created_xact_id
        JOIN game_item_instances i ON i.item_instance_id=p.item_instance_id
        WHERE r.created_xact_id=pg_current_xact_id() AND p.item_instance_id=OLD.item_instance_id
          AND w.cause=3 AND w.charges_before=1 AND i.last_transaction_id=w.transaction_id
          AND i.lifecycle=2 AND i.quantity=0 AND p.location=to_jsonb(OLD)
    ) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);

    FOREACH signature IN ARRAY ARRAY[
        'game_item_crusher_nested_tool_admit()',
        'game_item_crusher_nested_tool_complete()',
        'game_item_crusher_nested_tool_held(uuid,uuid,uuid)'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp',
            signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $migration$;

REVOKE ALL ON game_item_crusher_nested_tool_proofs FROM PUBLIC;
GRANT INSERT ON game_item_crusher_nested_tool_proofs TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_item_crusher_nested_tool_held(UUID,UUID,UUID) TO oteryn_game_runtime;
