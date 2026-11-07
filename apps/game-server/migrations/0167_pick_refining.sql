-- MINING-1: Reference onUsePick shiny-stone refining. Reuse the native one-unit
-- item-use TRANSFORM, retaining the pick and original wire binding separately.
-- No generic mining table, ore output or invented respawn timer is admitted.
-- Rollback is a successor migration disabling new refining; retain receipts.

ALTER TABLE game_item_use_records
    DROP CONSTRAINT game_item_use_records_cause_kind_check,
    ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK (cause_kind IN (1,2,3));

DO $$ DECLARE n TEXT; BEGIN
    -- 0083's unnamed food/flask shape constraint, and only that constraint.
    SELECT conname INTO STRICT n FROM pg_constraint
     WHERE conrelid='game_item_use_records'::regclass AND contype='c'
       AND pg_get_constraintdef(oid) LIKE '%cause_kind = 2%shape = 1%';
    EXECUTE format('ALTER TABLE game_item_use_records DROP CONSTRAINT %I',n);
END $$;
ALTER TABLE game_item_use_records ADD CONSTRAINT game_item_use_cause_shape
    CHECK ((cause_kind=1 AND shape=1) OR cause_kind=2 OR (cause_kind=3 AND shape IN (3,4)));

CREATE TABLE game_pick_refinements (
    transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id)
        DEFERRABLE INITIALLY DEFERRED,
    tool_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    wire_binding BYTEA NOT NULL CHECK (octet_length(wire_binding)=32),
    percentile SMALLINT NOT NULL CHECK (percentile BETWEEN 1 AND 100),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_pick_refining_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE u game_item_use_records%ROWTYPE;
        p game_pick_refinements%ROWTYPE;
        tool game_item_instances%ROWTYPE;
        expected_key TEXT;
BEGIN
    SELECT * INTO u FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
    SELECT * INTO p FROM game_pick_refinements WHERE transaction_id=NEW.transaction_id;
    SELECT * INTO tool FROM game_item_instances WHERE item_instance_id=p.tool_item_instance_id;
    expected_key := CASE WHEN p.percentile=1 THEN 'oteryn:item.tibia.i3043'
                         WHEN p.percentile<=6 THEN 'oteryn:item.tibia.i3031'
                         WHEN p.percentile<=51 THEN 'oteryn:item.tibia.i3035'
                         ELSE 'oteryn:item.tibia.i3028' END;
    IF u.transaction_id IS NULL OR p.transaction_id IS NULL OR u.cause_kind<>3
       OR u.created_xact_id<>pg_current_xact_id() OR p.created_xact_id<>pg_current_xact_id()
       OR u.source_definition_family<>'Item'
       OR u.source_definition_production_key<>'oteryn:item.tibia.i10310'
       OR u.flask_definition_family<>'Item' OR u.flask_definition_production_key<>expected_key
       OR tool.item_instance_id IS NULL OR tool.lifecycle<>1 OR tool.world_id<>u.world_id
       OR tool.definition_family<>'Item' OR tool.definition_production_key<>'oteryn:item.tibia.i3456'
       OR tool.item_instance_id IN (u.source_item_instance_id,u.flask_item_instance_id)
       OR tool.last_transaction_id IS NOT DISTINCT FROM u.transaction_id
       OR NOT EXISTS (SELECT 1 FROM game_item_loop_owner(tool.item_instance_id) o
                       WHERE o.owner_character=u.character_id AND o.depth BETWEEN 1 AND 8)
    THEN RAISE EXCEPTION 'pick refining requires its unchanged carried pick and atomic conversion'
        USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_pick_refining_proof
    AFTER INSERT ON game_pick_refinements DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_pick_refining_guard();
CREATE CONSTRAINT TRIGGER game_pick_refining_record
    AFTER INSERT ON game_item_use_records DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.cause_kind=3) EXECUTE FUNCTION game_pick_refining_guard();
CREATE TRIGGER game_pick_refinements_immutable BEFORE UPDATE OR DELETE ON game_pick_refinements
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_pick_refinements_no_truncate BEFORE TRUNCATE ON game_pick_refinements
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_pick_refinements_stamp_xact BEFORE INSERT ON game_pick_refinements
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_pick_refining_guard() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON game_pick_refinements FROM PUBLIC;
REVOKE ALL ON FUNCTION game_pick_refining_guard() FROM PUBLIC;
GRANT SELECT,INSERT ON game_pick_refinements TO oteryn_game_runtime;
GRANT SELECT ON game_pick_refinements TO oteryn_game_control;
