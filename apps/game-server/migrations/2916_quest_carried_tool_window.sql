-- Repeatable carried transformations use the existing World callback consumption
-- and Item deadline owners. Crystal items.xml:24803-24809 returns heated 9201
-- to 9234 after ten seconds even when it has not struck a worm.
CREATE TABLE game_quest_tool_windows (
    start_transaction_id UUID NOT NULL,
    item_instance_id UUID NOT NULL,
    duration_ms INTEGER NOT NULL CHECK(duration_ms BETWEEN 1 AND 600000),
    started_ms BIGINT NOT NULL DEFAULT floor(extract(epoch FROM statement_timestamp())*1000),
    deadline_ms BIGINT GENERATED ALWAYS AS (started_ms + duration_ms) STORED,
    PRIMARY KEY(start_transaction_id,item_instance_id),
    FOREIGN KEY(start_transaction_id,item_instance_id)
        REFERENCES game_quest_callback_consumptions(transaction_id,item_instance_id)
);
CREATE INDEX game_quest_tool_windows_due ON game_quest_tool_windows(deadline_ms,item_instance_id);
CREATE INDEX game_quest_tool_windows_item ON game_quest_tool_windows(item_instance_id,deadline_ms);

CREATE TABLE game_quest_tool_expiries (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    start_transaction_id UUID NOT NULL,
    item_instance_id UUID NOT NULL,
    before_item JSONB NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(start_transaction_id,item_instance_id)
        REFERENCES game_quest_tool_windows(start_transaction_id,item_instance_id),
    UNIQUE(start_transaction_id,item_instance_id)
);

CREATE FUNCTION game_quest_tool_window_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_quest_callback_consumptions c
        WHERE c.transaction_id=NEW.start_transaction_id AND c.item_instance_id=NEW.item_instance_id
          AND c.created_xact_id=pg_current_xact_id() AND c.quantity=1 AND c.replacement IS NOT NULL
          AND c.location_kind IN ('container','nested','backpack','equipment')
          AND (c.before_item->>'quantity')::bigint=1
          AND c.replacement<>c.before_item->>'definition_production_key'
          AND convert_from(c.proof,'UTF8')::jsonb #> '{callback,effects,carried_tool_window}'
                = jsonb_build_object('kind','start','duration_ms',NEW.duration_ms)
    ) OR EXISTS (
        SELECT 1 FROM game_quest_tool_windows w
        WHERE w.item_instance_id=NEW.item_instance_id
          AND NOT EXISTS (
              SELECT 1 FROM game_quest_tool_expiries x
              WHERE (x.start_transaction_id,x.item_instance_id)=(w.start_transaction_id,w.item_instance_id)
          )
    ) THEN
        RAISE EXCEPTION 'quest tool window requires an exact new carried transform' USING ERRCODE='23514';
    END IF;
    NEW.started_ms:=floor(extract(epoch FROM statement_timestamp())*1000);
    RETURN NEW;
END $$;

CREATE FUNCTION game_quest_tool_expiry_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_quest_tool_windows w
        JOIN game_quest_callback_consumptions c
          ON (c.transaction_id,c.item_instance_id)=(w.start_transaction_id,w.item_instance_id)
        JOIN game_item_instances i ON i.item_instance_id=w.item_instance_id
        WHERE (w.start_transaction_id,w.item_instance_id)=(NEW.start_transaction_id,NEW.item_instance_id)
          AND w.deadline_ms<=floor(extract(epoch FROM statement_timestamp())*1000)
          AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
          AND i.definition_production_key=c.replacement
          AND i.definition_revision_ref=c.before_item->>'definition_revision_ref'
          AND to_jsonb(i)=NEW.before_item
        FOR UPDATE OF i
    ) THEN
        RAISE EXCEPTION 'quest tool expiry requires its due exact item' USING ERRCODE='23514';
    END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    RETURN NEW;
END $$;

CREATE FUNCTION game_quest_tool_expiry_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_quest_callback_consumptions c
        JOIN game_item_instances i USING(item_instance_id)
        WHERE (c.transaction_id,c.item_instance_id)=(NEW.start_transaction_id,NEW.item_instance_id)
          AND i.last_transaction_id=NEW.transaction_id
          AND i.definition_production_key=c.before_item->>'definition_production_key'
          AND i.state_revision=(NEW.before_item->>'state_revision')::numeric+1
          AND NEW.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'quest tool deadline is missing its same-transaction successor' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER game_quest_tool_window_admit BEFORE INSERT ON game_quest_tool_windows
    FOR EACH ROW EXECUTE FUNCTION game_quest_tool_window_admit();
CREATE TRIGGER game_quest_tool_expiry_admit BEFORE INSERT ON game_quest_tool_expiries
    FOR EACH ROW EXECUTE FUNCTION game_quest_tool_expiry_admit();
CREATE CONSTRAINT TRIGGER game_quest_tool_expiry_complete AFTER INSERT ON game_quest_tool_expiries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_tool_expiry_complete();

DO $migration$
DECLARE body TEXT; anchor INT;
BEGIN
    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    anchor:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF anchor=0 THEN RAISE EXCEPTION 'missing Item instance guard body'; END IF;
    EXECUTE overlay(body PLACING $guard$BEGIN
    IF TG_OP='UPDATE' AND EXISTS(
        SELECT 1 FROM game_quest_tool_expiries x JOIN game_quest_callback_consumptions c
          ON (c.transaction_id,c.item_instance_id)=(x.start_transaction_id,x.item_instance_id)
        WHERE (x.item_instance_id,x.transaction_id)=(OLD.item_instance_id,NEW.last_transaction_id)
          AND x.created_xact_id=pg_current_xact_id() AND x.before_item=to_jsonb(OLD)
          AND NEW.definition_production_key=c.before_item->>'definition_production_key'
          AND (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
                =(to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
    ) THEN
        NEW.state_revision:=OLD.state_revision+1;
        INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before)
            VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
        RETURN NEW;
    END IF;
$guard$ FROM anchor FOR 5);
    body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
    anchor:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF anchor=0 THEN RAISE EXCEPTION 'missing Item change proof body'; END IF;
    EXECUTE overlay(body PLACING $proof$BEGIN
    IF EXISTS(
        SELECT 1 FROM game_quest_tool_expiries x
        WHERE (x.item_instance_id,x.transaction_id)=(NEW.item_instance_id,NEW.last_transaction_id)
          AND x.created_xact_id=pg_current_xact_id()
    ) THEN RETURN NULL; END IF;
$proof$ FROM anchor FOR 5);
END $migration$;

CREATE TRIGGER game_quest_tool_windows_immutable BEFORE UPDATE OR DELETE ON game_quest_tool_windows
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_tool_expiries_immutable BEFORE UPDATE OR DELETE ON game_quest_tool_expiries
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_tool_windows_no_truncate BEFORE TRUNCATE ON game_quest_tool_windows
    EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_quest_tool_expiries_no_truncate BEFORE TRUNCATE ON game_quest_tool_expiries
    EXECUTE FUNCTION game_item_reject_truncate();
DO $permissions$
DECLARE name TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_quest_tool_window_admit','game_quest_tool_expiry_admit','game_quest_tool_expiry_complete'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $permissions$;
REVOKE ALL ON game_quest_tool_windows,game_quest_tool_expiries FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_tool_windows,game_quest_tool_expiries TO oteryn_game_runtime;
GRANT SELECT ON game_quest_tool_windows,game_quest_tool_expiries TO oteryn_game_control;
