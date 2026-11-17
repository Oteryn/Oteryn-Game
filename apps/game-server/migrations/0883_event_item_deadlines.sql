-- W19b exact map USE/AddItem and the existing absolute Item deadline owners.
-- Forward rollback disables these source actions; immutable receipts remain.

-- W19 map actions extend the terrain/craft owners. A literal map USE has no
-- carried tool; retain that absence instead of inventing an ItemInstance UUID.
ALTER TABLE game_terrain_tool_receipts ALTER COLUMN tool_item_instance_id DROP NOT NULL;
ALTER TABLE game_terrain_tool_receipts ADD COLUMN created_xact_id XID8 NOT NULL
    DEFAULT pg_current_xact_id();
DO $migration$
DECLARE body TEXT; before TEXT;
BEGIN
    body := pg_get_functiondef('game_apply_terrain_tool_transform(uuid,numeric,uuid,uuid,bytea,uuid,uuid,bytea,text,bytea,bigint,bigint,bigint,bigint,bigint[],bigint[])'::regprocedure);
    before := 'prior.tool_item_instance_id=p_tool_item_instance_id';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected terrain replay proof'; END IF;
    EXECUTE replace(body,before,'prior.tool_item_instance_id IS NOT DISTINCT FROM p_tool_item_instance_id');
END $migration$;

-- A source seed has no minted output: it is consumed together with the map CAS.
-- Keep the existing gathering-only zero-debit arm unchanged.
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
CHECK (jsonb_typeof(outputs) = 'array' AND
    CASE recipe
        WHEN 'oteryn:event-container.6506' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6506'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.6507' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6507'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.6508' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i6508'
            AND jsonb_array_length(outputs) = 7
        WHEN 'oteryn:event-container.39704' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39704'
            AND jsonb_array_length(outputs) = 8
        WHEN 'oteryn:event-container.39705' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39705'
            AND jsonb_array_length(outputs) = 13
        WHEN 'oteryn:event-container.39706' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39706'
            AND jsonb_array_length(outputs) = 10
        WHEN 'oteryn:event-container.39710' THEN
            consume_source
            AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM 'oteryn:item.tibia.i39710'
            AND jsonb_array_length(outputs) = 14
        ELSE jsonb_array_length(outputs) BETWEEN 0 AND 4
            AND (jsonb_array_length(outputs) > 0
                OR (NOT consume_source AND recipe = 'oteryn:gathering.crystal')
                OR (consume_source AND recipe = 'oteryn:soulpit.entry')
                OR (consume_source AND recipe = 'oteryn:planting.crystal'
                    AND before_item->>'definition_production_key' = 'oteryn:item.tibia.i13844'))
    END);

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected craft admission body'; END IF;
    arm := $arm$
    IF NEW.recipe='oteryn:planting.crystal' THEN
        IF NOT NEW.consume_source OR NEW.outputs<>'[]'::jsonb
            OR NEW.before_item->>'definition_production_key'<>'oteryn:item.tibia.i13844'
            OR NOT EXISTS (
                SELECT 1 FROM game_terrain_tool_receipts r
                JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
                    =(r.world_id,r.channel_id,r.placement,r.reset_epoch)
                WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
                    AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
                    AND r.intent=NEW.intent AND r.tool_item_instance_id=NEW.source_item_instance_id
                    AND r.created_xact_id=pg_current_xact_id()
                    AND r.transformed_item_id=t.state_item_id
                    AND ((t.state_item_id=14029 AND t.decay_states='{}'::bigint[] AND t.decay_delays='{}'::bigint[])
                        OR (t.state_item_id=14031 AND t.decay_states=ARRAY[14032,306]::bigint[]
                            AND t.decay_delays=ARRAY[72000000,72000000]::bigint[])))
        THEN RAISE EXCEPTION 'planting requires its source seed and terrain receipt' USING ERRCODE='23514'; END IF;
    END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- W19 literal map USE: one real terrain revision, at most one minted yield stack,
-- existing Character custody/Ground and audit owners. No synthetic carried input.
CREATE TABLE game_map_harvest_records (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
    placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
    reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
    object_revision BIGINT NOT NULL CHECK(object_revision>=0),
    before_item_id BIGINT NOT NULL,
    after_item_id BIGINT NOT NULL,
    output_item_instance_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(output_item_instance_id)),
    output_key TEXT NOT NULL,
    output_revision TEXT NOT NULL CHECK(octet_length(output_revision) BETWEEN 1 AND 512),
    output_quantity BIGINT NOT NULL,
    parent_item_instance_id UUID,
    placement_ordinal NUMERIC(20,0),
    ground_position BYTEA,
    map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
    content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 512),
    intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 529),
    occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
    envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    UNIQUE(world_id,channel_id,placement,reset_epoch,object_revision),
    CHECK((before_item_id,after_item_id,output_key,output_quantity) IN
        ((3699,3700,'oteryn:item.tibia.i3588',3),(13842,13843,'oteryn:item.tibia.i13844',1))),
    CHECK((parent_item_instance_id IS NOT NULL AND placement_ordinal IS NOT NULL
            AND placement_ordinal BETWEEN 1 AND 18446744073709551615 AND ground_position IS NULL)
        OR (parent_item_instance_id IS NULL AND placement_ordinal IS NULL
            AND ground_position IS NOT NULL AND octet_length(ground_position)=10)),
    CHECK(before_item_id<>3699 OR parent_item_instance_id IS NULL)
);

CREATE FUNCTION game_map_harvest_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_map_harvest_records%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_map_harvest_records WHERE transaction_id=p_transaction;
    IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
        OR NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=r.character_id AND c.world_id=r.world_id)
        OR NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts t JOIN game_terrain_tool_transforms s
            ON (s.world_id,s.channel_id,s.placement,s.reset_epoch)=(t.world_id,t.channel_id,t.placement,t.reset_epoch)
            WHERE t.game_session_id=r.game_session_id AND t.command_id=r.command_id AND t.character_id=r.character_id
                AND t.world_id=r.world_id AND t.channel_id=r.channel_id AND t.placement=r.placement AND t.reset_epoch=r.reset_epoch
                AND t.intent=r.intent AND t.tool_item_instance_id IS NULL AND t.transformed_item_id=r.after_item_id
                AND t.created_xact_id=pg_current_xact_id() AND s.object_revision=r.object_revision+1
                AND s.state_item_id=r.after_item_id AND s.map_revision=r.map_revision
                AND ((r.before_item_id=3699 AND s.decay_states=ARRAY[3699]::bigint[] AND s.decay_delays=ARRAY[300000]::bigint[])
                    OR (r.before_item_id=13842 AND s.decay_states=ARRAY[13842]::bigint[] AND s.decay_delays=ARRAY[14400000]::bigint[])))
        OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=r.output_item_instance_id
            AND i.world_id=r.world_id AND i.definition_family='Item' AND i.definition_production_key=r.output_key
            AND i.definition_revision_ref=r.output_revision AND i.quantity=r.output_quantity AND i.lifecycle=1
            AND i.state_revision=1 AND i.last_transaction_id IS NULL AND i.minted_transaction_id=r.transaction_id)
        OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
        OR EXISTS(SELECT 1 FROM game_item_instances WHERE last_transaction_id=r.transaction_id)
        OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
        OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
            AND a.item_instance_id=r.output_item_instance_id AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
            AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL
            AND a.transaction_ordinal=1 AND a.transaction_count=1)
        OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=r.output_item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=r.output_item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=r.output_item_instance_id)
    THEN RAISE EXCEPTION 'incomplete atomic map harvest' USING ERRCODE='23514'; END IF;
    IF r.parent_item_instance_id IS NOT NULL THEN
        IF EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.output_item_instance_id)
            OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                ON b.item_instance_id=e.parent_item_instance_id AND b.character_id=e.character_id
                WHERE e.item_instance_id=r.output_item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id
                    AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=r.placement_ordinal
                    AND e.placed_transaction_id=r.transaction_id AND b.world_id=r.world_id)
            OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
        THEN RAISE EXCEPTION 'map harvest backpack custody mismatch' USING ERRCODE='23514'; END IF;
    ELSE
        IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.output_item_instance_id)
            OR NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=r.output_item_instance_id
                AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.scope_generation
                AND g.spatial_position=r.ground_position AND g.corpse_ref=convert_to('map-harvest-v1','UTF8')
                AND g.map_revision=r.map_revision AND g.content_revision=r.content_revision
                AND g.native_room_placement_context=substring(r.placement from 1 for 32))
            OR (r.before_item_id=3699 AND r.ground_position<>
                int4send(get_byte(r.placement,34)*256+get_byte(r.placement,35))
                ||int4send(get_byte(r.placement,36)*256+get_byte(r.placement,37))
                ||int2send((-get_byte(r.placement,38))::smallint))
        THEN RAISE EXCEPTION 'map harvest Ground custody mismatch' USING ERRCODE='23514'; END IF;
    END IF;
END $$;

CREATE FUNCTION game_map_harvest_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_map_harvest_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_map_harvest_stamp BEFORE INSERT ON game_map_harvest_records
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE CONSTRAINT TRIGGER game_map_harvest_consistent AFTER INSERT ON game_map_harvest_records
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_map_harvest_consistent();
CREATE TRIGGER game_map_harvest_immutable BEFORE UPDATE OR DELETE ON game_map_harvest_records
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_map_harvest_no_truncate BEFORE TRUNCATE ON game_map_harvest_records
    EXECUTE FUNCTION game_item_reject_truncate();

-- Retain every existing proof arm. These arms prove only this current transaction's
-- closed, source-qualified yield and call the complete map/custody/audit check.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_ground_insertion_guard','game_item_placement_proven'] LOOP
        body:=pg_get_functiondef((n||'()')::regprocedure);
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'unexpected map harvest proof body: %',n; END IF;
        IF n='game_item_placement_proven' THEN
            arm:=$arm$
            IF TG_TABLE_NAME='game_item_container_entries' THEN
            IF EXISTS(SELECT 1 FROM game_map_harvest_records r
                WHERE r.output_item_instance_id=NEW.item_instance_id AND r.world_id=NEW.world_id
                    AND r.character_id=NEW.character_id AND r.parent_item_instance_id=NEW.parent_item_instance_id
                    AND r.placement_ordinal=NEW.placement_ordinal AND r.transaction_id=NEW.placed_transaction_id
                    AND r.created_xact_id=pg_current_xact_id()) THEN
                PERFORM game_map_harvest_complete(NEW.placed_transaction_id); RETURN NULL;
            END IF;
            END IF;
$arm$;
        ELSE
            arm:=$arm$
            IF EXISTS(SELECT 1 FROM game_map_harvest_records r WHERE r.output_item_instance_id=NEW.item_instance_id
                AND r.created_xact_id=pg_current_xact_id()) THEN
                PERFORM game_map_harvest_complete((SELECT transaction_id FROM game_map_harvest_records
                    WHERE output_item_instance_id=NEW.item_instance_id)); RETURN NULL;
            END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_map_harvest_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_map_harvest_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_map_harvest_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_map_harvest_complete(UUID),game_map_harvest_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_map_harvest_records TO oteryn_game_runtime;
GRANT SELECT ON game_map_harvest_records TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_map_harvest_complete(UUID) TO oteryn_game_runtime;

-- Ground browsing/pickup recognizes only the immutable value receipt, not its marker.
CREATE FUNCTION game_item_map_harvest_source(p_item uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS(SELECT 1 FROM game_map_harvest_records WHERE output_item_instance_id=p_item)
$$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_item_map_harvest_source(uuid) SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_map_harvest_source(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_map_harvest_source(uuid) TO oteryn_game_runtime;

-- Crystal data/scripts/movements/seeds.lua:5-16: one AddItem unit is consumed
-- after an ordinary proven drop, in the same transaction as target306's map CAS.
CREATE TABLE game_map_seed_drop_records (
    transaction_id uuid PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    event_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
    move_transaction_id uuid NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
    game_session_id uuid NOT NULL,
    command_id numeric(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id uuid NOT NULL REFERENCES game_character_roots,
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    item_instance_id uuid NOT NULL REFERENCES game_item_instances,
    before_item jsonb NOT NULL CHECK(jsonb_typeof(before_item)='object'),
    before_ground jsonb NOT NULL CHECK(jsonb_typeof(before_ground)='object'),
    intent bytea NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 4096),
    placement bytea NOT NULL CHECK(octet_length(placement)=40),
    reset_epoch bigint NOT NULL CHECK(reset_epoch>=0),
    object_revision bigint NOT NULL CHECK(object_revision>=0),
    after_item_id bigint NOT NULL CHECK(after_item_id IN(324,14031)),
    map_revision text NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
    occurred_at bigint NOT NULL CHECK(occurred_at>0),
    envelope_sha256 bytea NOT NULL CHECK(octet_length(envelope_sha256)=32),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    CHECK((before_item->>'definition_production_key',after_item_id) IN
        (('oteryn:item.tibia.i647',324),('oteryn:item.tibia.i13844',14031)))
);

CREATE FUNCTION game_map_seed_drop_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_xact_id:=pg_current_xact_id();
    IF NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
        JOIN game_item_loop_move_lines l USING(item_instance_id)
        JOIN game_item_loop_move_receipts r USING(transaction_id)
        WHERE i.item_instance_id=NEW.item_instance_id AND to_jsonb(i)=NEW.before_item AND to_jsonb(g)=NEW.before_ground
            AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item'
            AND r.transaction_id=NEW.move_transaction_id AND r.created_xact_id=pg_current_xact_id()
            AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id AND r.character_id=NEW.character_id
            AND r.world_id=NEW.world_id AND r.channel_id=NEW.channel_id AND r.intent=NEW.intent
            AND l.to_kind=3 AND l.quantity_after=i.quantity AND l.to_spatial_position=g.spatial_position
            AND g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id AND g.map_revision=NEW.map_revision
            AND g.runtime_scope_ownership_generation=r.scope_generation
            AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id)
            AND g.spatial_position=int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
                ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))
                ||int2send((-get_byte(NEW.placement,38))::smallint)
            AND ((l.from_kind=0 AND i.minted_transaction_id=r.transaction_id AND i.last_transaction_id IS NULL AND i.state_revision=1)
                OR(l.from_kind<>0 AND i.last_transaction_id=r.transaction_id AND i.state_revision=l.state_revision_before+1)))
    THEN RAISE EXCEPTION 'seed planting requires the exact newly dropped live seed' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION game_map_seed_drop_complete(p_transaction uuid) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_map_seed_drop_records%ROWTYPE; i game_item_instances%ROWTYPE; expected bigint;
BEGIN
    SELECT * INTO r FROM game_map_seed_drop_records WHERE transaction_id=p_transaction;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
    expected:=(r.before_item->>'quantity')::bigint-1;
    IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL
        OR i.quantity<>expected OR i.lifecycle<>(CASE WHEN expected=0 THEN 2 ELSE 1 END)
        OR i.state_revision<>(r.before_item->>'state_revision')::numeric+1 OR i.last_transaction_id<>r.transaction_id
        OR (to_jsonb(i)-ARRAY['quantity','lifecycle','state_revision','last_transaction_id']) IS DISTINCT FROM
            (r.before_item-ARRAY['quantity','lifecycle','state_revision','last_transaction_id'])
        OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
        OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
        OR NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts t JOIN game_terrain_tool_transforms s
            ON(s.world_id,s.channel_id,s.placement,s.reset_epoch)=(t.world_id,t.channel_id,t.placement,t.reset_epoch)
            WHERE t.game_session_id=r.game_session_id AND t.command_id=r.command_id AND t.character_id=r.character_id
                AND t.world_id=r.world_id AND t.channel_id=r.channel_id AND t.placement=r.placement AND t.reset_epoch=r.reset_epoch
                AND t.intent=sha256(r.intent) AND t.tool_item_instance_id IS NULL AND t.created_xact_id=pg_current_xact_id()
                AND t.transformed_item_id=r.after_item_id AND s.state_item_id=r.after_item_id
                AND s.object_revision=r.object_revision+1 AND s.map_revision=r.map_revision
                AND s.decay_states=ARRAY[CASE WHEN r.after_item_id=324 THEN 330 ELSE 14032 END,306]::bigint[]
                AND s.decay_delays=ARRAY[72000000,72000000]::bigint[])
        OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
        OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=r.transaction_id
            AND a.item_instance_id=r.item_instance_id AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
            AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
            AND a.publication_state=1 AND a.published_at IS NULL)
    THEN RAISE EXCEPTION 'seed drop requires its one-unit debit, map CAS and audit' USING ERRCODE='23514'; END IF;
    IF expected=0 THEN
        IF EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
        THEN RAISE EXCEPTION 'consumed seed retains Ground custody' USING ERRCODE='23514'; END IF;
    ELSE
        IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=r.item_instance_id AND to_jsonb(g)=r.before_ground)
        THEN RAISE EXCEPTION 'seed stack remainder changed Ground custody' USING ERRCODE='23514'; END IF;
    END IF;
END $$;

CREATE FUNCTION game_map_seed_drop_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_map_seed_drop_complete(NEW.transaction_id); RETURN NULL; END $$;

-- Extend only the exact current seed receipt arm; all existing item rules remain intact.
DO $migration$
DECLARE body text; arm text; begin_at int;
BEGIN
    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected seed instance guard'; END IF;
    arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_map_seed_drop_records r
        WHERE r.item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
            AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
        IF NEW.quantity<>OLD.quantity-1 OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
            OR(to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
              (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
        THEN RAISE EXCEPTION 'planting consumes exactly one dropped seed' USING ERRCODE='23514'; END IF;
        -- This receipt arm returns before the shared owner's revision increment.
        NEW.state_revision := OLD.state_revision + 1;
        RETURN NEW;
    END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    EXECUTE format('ALTER FUNCTION game_map_seed_drop_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_map_seed_drop_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_map_seed_drop_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
CREATE TRIGGER game_map_seed_drop_admit BEFORE INSERT ON game_map_seed_drop_records
    FOR EACH ROW EXECUTE FUNCTION game_map_seed_drop_admit();
CREATE CONSTRAINT TRIGGER game_map_seed_drop_consistent AFTER INSERT ON game_map_seed_drop_records
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_map_seed_drop_consistent();
CREATE TRIGGER game_map_seed_drop_immutable BEFORE UPDATE OR DELETE ON game_map_seed_drop_records
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_map_seed_drop_no_truncate BEFORE TRUNCATE ON game_map_seed_drop_records
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_map_seed_drop_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_map_seed_drop_admit(),game_map_seed_drop_complete(uuid),game_map_seed_drop_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_map_seed_drop_records TO oteryn_game_runtime;
GRANT SELECT ON game_map_seed_drop_records TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_map_seed_drop_complete(uuid) TO oteryn_game_runtime;


-- The existing deadline table now accepts either original spell-mint or original
-- action-mint provenance. Both routes retain their same-XID deferred proof.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 SELECT conname INTO STRICT name FROM pg_constraint
  WHERE conrelid='game_spell_item_temporal_schedules'::regclass AND contype='f'
    AND confrelid='game_spell_item_lines'::regclass;
 EXECUTE format('ALTER TABLE game_spell_item_temporal_schedules DROP CONSTRAINT %I',name);
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
 arm:=$arm$
  IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,
    LATERAL jsonb_array_elements(r.outputs) WITH ORDINALITY o(value,ordinal),game_item_instances i
    WHERE r.transaction_id=NEW.source_transaction_id AND o.ordinal=NEW.source_ordinal
      AND (o.value->>'item')::uuid=NEW.item_instance_id AND i.item_instance_id=NEW.item_instance_id
      AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.world_id AND i.quantity=1 AND i.lifecycle=1
      AND NEW.definition_family='Item' AND (o.value->>'count')::bigint=1
      AND (i.definition_production_key,i.definition_revision_ref)=(NEW.definition_production_key,NEW.definition_revision)
      AND (o.value->>'key',o.value->>'revision')=(NEW.definition_production_key,NEW.definition_revision)
      AND r.content_digest=NEW.content_digest AND NEW.expires_at_unix_ms=r.occurred_at+NEW.duration_millis
      AND r.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=r.created_xact_id
      AND o.value->'decay'=jsonb_build_object('duration_millis',NEW.duration_millis,'target',
        CASE WHEN NEW.target_family IS NULL THEN NULL ELSE jsonb_build_object('family',NEW.target_family,
          'key',NEW.target_production_key,'revision',NEW.target_revision) END)) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- Absolute deadlines keep their original time while a trumpet is below a bag.
-- The existing custody kind2 denotes carried container custody at depth1..8.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_capture()'::regprocedure);
 original:='SELECT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.source_parent AND e.placement_ordinal=r.source_ordinal) INTO matches;';
 replacement:='SELECT (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.source_parent AND e.placement_ordinal=r.source_ordinal) OR EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner WHERE n.item_instance_id=OLD.item_instance_id AND n.world_id=r.world_id AND n.parent_item_instance_id=r.source_parent AND n.placement_ordinal=r.source_ordinal AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)) INTO matches;';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing carried timer predecessor proof'; END IF;
 EXECUTE replace(body,original,replacement);
 body:=pg_get_functiondef('game_spell_item_temporal_receipt_proven()'::regprocedure);
 original:='(NEW.custody_kind=2 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal))';
 replacement:='(NEW.custody_kind=2 AND (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
     WHERE n.item_instance_id=i.item_instance_id AND n.world_id=NEW.world_id AND n.parent_item_instance_id=NEW.source_parent
      AND n.placement_ordinal=NEW.source_ordinal AND owner.owner_character=NEW.character_id AND owner.depth BETWEEN 0 AND 7)))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing carried timer successor proof'; END IF;
 body:=replace(body,original,replacement);
 original:='OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing retired timer custody proof'; END IF;
 body:=replace(body,original,original||' OR EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id)');
 EXECUTE body;
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing nested timer removal proof'; END IF;
 arm:=$arm$
  IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id)
     WHERE r.custody_kind=2 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
      AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
      AND r.created_xact_id=pg_current_xact_id() AND p.target_family IS NULL
      AND i.last_transaction_id=r.transaction_id AND i.lifecycle=2 AND i.quantity=0) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;


-- Source action deadlines are hosted by the existing absolute owner for Ground,
-- main and nested carried custody. Moving a trumpet keeps its immutable schedule.
-- Other spell/field temporal sources retain their previous movement admission.
DO $migration$ DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing unhosted absolute timer movement guard'; END IF;
 EXECUTE replace(body,original,'('||original||' AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p JOIN game_item_consume_transform_records action_receipt ON action_receipt.transaction_id=p.source_transaction_id,LATERAL jsonb_array_elements(action_receipt.outputs) o WHERE p.item_instance_id=NEW.item_instance_id AND (o->>''item'')::uuid=p.item_instance_id AND o->''decay'' IS NOT NULL AND o->''decay''<>''null''::jsonb))');
END $migration$;


-- W19b/W19c: admit source-bound consume/transform outputs to the existing absolute
-- Item deadline owner. Original spell sources retain their original FK and proof.
ALTER TABLE game_spell_field_temporal_schedules
    ALTER COLUMN source_transaction_id DROP NOT NULL,
    ADD COLUMN event_source_transaction_id UUID REFERENCES game_item_consume_transform_records(transaction_id),
    ADD CONSTRAINT game_field_temporal_one_source CHECK
        (num_nonnulls(source_transaction_id,event_source_transaction_id)=1);
CREATE INDEX game_field_temporal_event_source
    ON game_spell_field_temporal_schedules(event_source_transaction_id)
    WHERE event_source_transaction_id IS NOT NULL;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'unexpected field temporal source proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF NEW.event_source_transaction_id IS NOT NULL THEN
  DECLARE r game_item_consume_transform_records%ROWTYPE; output JSONB; chain JSONB;
          stage JSONB; elapsed BIGINT; initial TEXT; expected_source TEXT; expected_target TEXT;
          expected_duration BIGINT; expected_count INT;
  BEGIN
   SELECT * INTO r FROM game_item_consume_transform_records
     WHERE transaction_id=NEW.event_source_transaction_id AND created_xact_id=pg_current_xact_id();
   IF NOT FOUND OR NEW.source_transaction_id IS NOT NULL OR NEW.source_ordinal<1
      OR NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id() THEN
    RAISE EXCEPTION 'event timer requires its original same-XID conversion' USING ERRCODE='23514';
   END IF;
   output:=r.outputs->(NEW.source_ordinal-1);
   chain:=output->'event_decay';
   initial:=output->>'key';
   -- Crystal items.xml:18237-42 and32372-84. Sources cannot author arbitrary timers.
   IF initial='oteryn:item.tibia.i14031' THEN
    expected_count:=2; expected_duration:=72000000;
    expected_source:=CASE NEW.source_stage WHEN 1 THEN initial WHEN 2 THEN 'oteryn:item.tibia.i14032' END;
    expected_target:=CASE NEW.source_stage WHEN 1 THEN 'oteryn:item.tibia.i14032' WHEN 2 THEN 'oteryn:item.tibia.i306' END;
    IF r.recipe IS DISTINCT FROM 'oteryn:planting.ice-flower'
       OR r.before_item->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i306'
       OR (r.before_secondary->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i13844'
           AND NOT game_event_pot_planting(r.source_kind,r.consume_source,r.source_count,
               r.before_item,r.effects,r.extra_inputs,r.outputs,r.recipe)) THEN
     RAISE EXCEPTION 'plant timer requires the pot and seed conversion' USING ERRCODE='23514';
    END IF;
   ELSE
    RAISE EXCEPTION 'event timer source is not admitted' USING ERRCODE='23514';
   END IF;
   IF jsonb_typeof(chain) IS DISTINCT FROM 'array' OR jsonb_array_length(chain) IS DISTINCT FROM expected_count
      OR NEW.source_stage NOT BETWEEN 1 AND expected_count
      OR (SELECT count(*) FROM game_spell_field_temporal_schedules WHERE item_instance_id=NEW.item_instance_id)<>expected_count
      OR (output->>'item')::uuid IS DISTINCT FROM NEW.item_instance_id
      OR (output->>'count')::int IS DISTINCT FROM 1
      OR decode(output->>'event_content_digest','hex') IS DISTINCT FROM NEW.content_digest
      OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=NEW.item_instance_id
          AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.world_id AND i.lifecycle=1 AND i.quantity=1
          AND i.definition_production_key=initial AND i.definition_revision_ref=output->>'revision') THEN
    RAISE EXCEPTION 'event timer output/source mismatch' USING ERRCODE='23514';
   END IF;
   stage:=chain->(NEW.source_stage-1);
   IF NEW.definition_family IS DISTINCT FROM 'Item' OR NEW.target_family IS DISTINCT FROM 'Item'
      OR NEW.definition_production_key IS DISTINCT FROM expected_source
      OR NEW.target_production_key IS DISTINCT FROM expected_target
      OR NEW.duration_millis IS DISTINCT FROM expected_duration
      OR NEW.definition_revision IS DISTINCT FROM output->>'revision'
      OR NEW.target_revision IS DISTINCT FROM output->>'revision'
      OR stage IS DISTINCT FROM jsonb_build_object(
          'definition',jsonb_build_object('family',NEW.definition_family,'key',NEW.definition_production_key,'revision',NEW.definition_revision),
          'duration_millis',NEW.duration_millis,
          'target',jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision),
          'blocks_movement',NEW.blocks_movement,'blocks_projectile',NEW.blocks_projectile,
          'immovable_block_solid',NEW.immovable_block_solid) THEN
    RAISE EXCEPTION 'event timer stage differs from immutable source' USING ERRCODE='23514';
   END IF;
   SELECT sum((value->>'duration_millis')::bigint) INTO elapsed
     FROM jsonb_array_elements(chain) WITH ORDINALITY t(value,ordinal) WHERE ordinal<=NEW.source_stage;
   IF NEW.expires_at_unix_ms IS DISTINCT FROM r.occurred_at+elapsed THEN
    RAISE EXCEPTION 'event timer deadline differs from original source time' USING ERRCODE='23514';
   END IF;
   RETURN NULL;
  END;
 END IF;
$arm$ from begin_at for 5);
 EXECUTE format('ALTER FUNCTION game_spell_field_temporal_schedule_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_spell_field_temporal_schedule_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_spell_field_temporal_schedule_proven() TO oteryn_game_runtime;

-- A map materialization moves the already-running pot deadline into the same Item
-- chain. Its initial expiry is the terrain deadline, never pickup time plus duration.
ALTER TABLE game_spell_field_temporal_schedules
 ADD COLUMN map_source_transaction_id UUID REFERENCES game_map_item_mint_receipts(transaction_id);
ALTER TABLE game_spell_field_temporal_schedules DROP CONSTRAINT game_field_temporal_one_source;
ALTER TABLE game_spell_field_temporal_schedules ADD CONSTRAINT game_field_temporal_one_source
 CHECK(num_nonnulls(source_transaction_id,event_source_transaction_id,map_source_transaction_id)=1);
CREATE INDEX game_field_temporal_map_source ON game_spell_field_temporal_schedules(map_source_transaction_id)
 WHERE map_source_transaction_id IS NOT NULL;
DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing map Item chain source proof'; END IF;
 arm:=$arm$
 IF NEW.map_source_transaction_id IS NOT NULL THEN
  DECLARE r game_map_item_mint_receipts%ROWTYPE; t game_terrain_tool_transforms%ROWTYPE;
          due BIGINT; current_key TEXT; target_key TEXT; states BIGINT[]; delays BIGINT[];
  BEGIN
   SELECT * INTO r FROM game_map_item_mint_receipts
    WHERE transaction_id=NEW.map_source_transaction_id AND created_xact_id=pg_current_xact_id();
   SELECT * INTO t FROM game_terrain_tool_transforms WHERE world_id=r.world_id AND channel_id=r.channel_id
     AND placement=r.base_bundle_digest||int8send(r.placement_key::bigint) AND reset_epoch=r.reset_epoch;
   IF r.transaction_id IS NULL OR t.decay_at_unix_ms IS NULL OR r.quantity<>1
      OR NEW.source_transaction_id IS NOT NULL OR NEW.event_source_transaction_id IS NOT NULL
      OR NEW.created_xact_id<>pg_current_xact_id() OR NEW.item_instance_id<>r.item_instance_id
      OR NEW.source_ordinal<>1 OR NEW.content_digest<>t.content_generation
      OR r.definition_family<>'Item' OR r.definition_production_key<>'oteryn:item.tibia.i'||t.state_item_id
      OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=r.item_instance_id
        AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.world_id AND i.lifecycle=1 AND i.quantity=1
        AND i.definition_family=r.definition_family AND i.definition_production_key=r.definition_production_key
        AND i.definition_revision_ref=r.definition_revision_ref) THEN
     RAISE EXCEPTION 'map pot timer lacks its exact current materialization' USING ERRCODE='23514';
   END IF;
   states:=CASE t.state_item_id WHEN 324 THEN ARRAY[330,306]::bigint[]
      WHEN 330 THEN ARRAY[306]::bigint[] WHEN 14031 THEN ARRAY[14032,306]::bigint[]
      WHEN 14032 THEN ARRAY[306]::bigint[] END;
   IF states IS NULL OR t.decay_states[t.decay_step+1:cardinality(t.decay_states)] IS DISTINCT FROM states
      OR NEW.source_stage NOT BETWEEN 1 AND cardinality(states)
      OR (SELECT count(*) FROM game_spell_field_temporal_schedules WHERE item_instance_id=NEW.item_instance_id)<>cardinality(states)
      OR t.decay_delays[t.decay_step+1:cardinality(t.decay_delays)] IS DISTINCT FROM array_fill(72000000::bigint,ARRAY[cardinality(states)]) THEN
     RAISE EXCEPTION 'map pot timer differs from the authored plant chain' USING ERRCODE='23514';
   END IF;
   current_key:='oteryn:item.tibia.i'||CASE WHEN NEW.source_stage=1 THEN t.state_item_id ELSE states[NEW.source_stage-1] END;
   target_key:='oteryn:item.tibia.i'||states[NEW.source_stage];
   due:=t.decay_at_unix_ms+(NEW.source_stage-1)*72000000::bigint;
   IF NEW.definition_family<>'Item' OR NEW.target_family IS DISTINCT FROM 'Item'
      OR NEW.definition_production_key<>current_key OR NEW.target_production_key IS DISTINCT FROM target_key
      OR NEW.definition_revision<>r.definition_revision_ref OR NEW.target_revision IS DISTINCT FROM r.definition_revision_ref
      OR NEW.duration_millis<>72000000 OR NEW.expires_at_unix_ms<>due
      OR NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid THEN
     RAISE EXCEPTION 'map pot timer rewrites its original absolute deadline' USING ERRCODE='23514';
   END IF;
   RETURN NULL;
  END;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- Event pot chains remain eligible after a proven ordinary move between stages.
-- Original spell chains retain their exact previous-transaction contract.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
 original:='previous.transaction_id=OLD.last_transaction_id AND previous.state_revision_before+1=OLD.state_revision';
 replacement:='((previous.transaction_id=OLD.last_transaction_id AND previous.state_revision_before+1=OLD.state_revision) OR ((p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL) AND previous.state_revision_before+1<OLD.state_revision AND EXISTS(SELECT 1 FROM game_item_loop_move_lines l JOIN game_item_loop_move_receipts m USING(transaction_id) WHERE l.item_instance_id=OLD.item_instance_id AND l.transaction_id=OLD.last_transaction_id AND m.world_id=OLD.world_id AND l.state_revision_before+1=OLD.state_revision)))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing exact field chain predecessor'; END IF;
 EXECUTE replace(body,original,replacement);
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='EXISTS (SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing unhosted field chain move guard'; END IF;
 EXECUTE replace(body,original,'('||original||' AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=NEW.item_instance_id AND (p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL)))');
END $migration$;

-- The same exact nested-custody proof also hosts the qualified event pot chain.
-- Absolute deadlines keep their original time while a trumpet is below a bag.
-- The existing custody kind2 denotes carried container custody at depth1..8.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
 original:='SELECT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.source_parent AND e.placement_ordinal=r.source_ordinal) INTO matches;';
 replacement:='SELECT (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.source_parent AND e.placement_ordinal=r.source_ordinal) OR EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner WHERE n.item_instance_id=OLD.item_instance_id AND n.world_id=r.world_id AND n.parent_item_instance_id=r.source_parent AND n.placement_ordinal=r.source_ordinal AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)) INTO matches;';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing carried timer predecessor proof'; END IF;
 EXECUTE replace(body,original,replacement);
 body:=pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
 original:='(NEW.custody_kind=2 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal))';
 replacement:='(NEW.custody_kind=2 AND (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
     WHERE n.item_instance_id=i.item_instance_id AND n.world_id=NEW.world_id AND n.parent_item_instance_id=NEW.source_parent
      AND n.placement_ordinal=NEW.source_ordinal AND owner.owner_character=NEW.character_id AND owner.depth BETWEEN 0 AND 7)))';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing carried timer successor proof'; END IF;
 body:=replace(body,original,replacement);
 original:='OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing retired timer custody proof'; END IF;
 body:=replace(body,original,original||' OR EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id)');
 EXECUTE body;
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing nested timer removal proof'; END IF;
 arm:=$arm$
  IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id)
     WHERE r.custody_kind=2 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
      AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
      AND r.created_xact_id=pg_current_xact_id() AND p.target_family IS NULL
      AND i.last_transaction_id=r.transaction_id AND i.lifecycle=2 AND i.quantity=0) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;


-- Restart/takeover keeps Ground time tied to its original event source. The current
-- Channel assignment, exact item/stack and the Rust owner fence still authorize drain.
CREATE FUNCTION game_event_item_timer_ground_current(p_item UUID,p_world UUID,p_channel UUID,p_generation NUMERIC,p_stack BIGINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g
  JOIN game_runtime_scope_assignments a ON a.world_id=g.world_id AND a.channel_id=g.channel_id
  WHERE g.item_instance_id=p_item AND g.world_id=p_world AND g.channel_id=p_channel AND g.stack_ordinal=p_stack
   AND a.state=1 AND a.ownership_generation=p_generation
   AND (EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=p_item
          AND (p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL))
     OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p JOIN game_item_consume_transform_records r
          ON r.transaction_id=p.source_transaction_id,LATERAL jsonb_array_elements(r.outputs) o
          WHERE p.item_instance_id=p_item AND (o->>'item')::uuid=p_item AND o->'decay' IS NOT NULL
            AND o->'decay'<>'null'::jsonb)))
$$;
DO $migration$
DECLARE body TEXT; f TEXT; original TEXT; replacement TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_spell_item_temporal_capture','game_spell_item_temporal_receipt_proven',
                         'game_spell_field_temporal_capture','game_spell_field_temporal_receipt_proven'] LOOP
  body:=pg_get_functiondef((f||'()')::regprocedure);
  IF f LIKE '%capture' THEN
   original:='g.runtime_scope_ownership_generation=r.ownership_generation';
   replacement:='(g.runtime_scope_ownership_generation=r.ownership_generation OR game_event_item_timer_ground_current(g.item_instance_id,r.world_id,r.channel_id,r.ownership_generation,r.source_stack_ordinal))';
  ELSE
   original:='g.runtime_scope_ownership_generation=NEW.ownership_generation';
   replacement:='(g.runtime_scope_ownership_generation=NEW.ownership_generation OR game_event_item_timer_ground_current(g.item_instance_id,NEW.world_id,NEW.channel_id,NEW.ownership_generation,NEW.source_stack_ordinal))';
  END IF;
  IF position(original IN body)=0 THEN RAISE EXCEPTION 'event timer Ground proof changed: %',f; END IF;
  EXECUTE replace(body,original,replacement);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint) TO oteryn_game_runtime;

-- A harvested ground yield is picked up only through its immutable exact output receipt.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='game_item_loop_map_mint_source(NEW.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing map harvest pickup admission'; END IF;
 IF position('game_item_map_harvest_source(NEW.item_instance_id)' IN body)<>0 THEN
  RAISE EXCEPTION 'map harvest pickup arm already installed';
 END IF;
 EXECUTE replace(body,original,'('||original||' OR game_item_map_harvest_source(NEW.item_instance_id))');
END $migration$;

-- An exact carried or Ground pot consumes one exact carried seed. The same
-- immutable action receipt proves main/nested custody; Ground keeps its source tile.
CREATE FUNCTION game_event_pot_planting(p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,
 p_before JSONB,p_effects JSONB,p_extra JSONB,p_outputs JSONB,p_recipe TEXT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(p_kind IN(1,3,4) AND p_consume AND p_count=1
  AND p_recipe='oteryn:planting.ice-flower'
  AND p_before->>'definition_family'='Item'
  AND p_before->>'definition_production_key'='oteryn:item.tibia.i306'
  AND (p_before->>'quantity')::bigint=1
  AND jsonb_array_length(p_extra)=1 AND (p_extra->0->>'count')::bigint=1
  AND p_extra->0->'before'->>'definition_family'='Item'
  AND p_extra->0->'before'->>'definition_production_key'='oteryn:item.tibia.i13844'
  AND (p_extra->0->>'source_kind')::integer IN(1,4)
  AND jsonb_array_length(p_outputs)=1 AND p_outputs->0->>'key'='oteryn:item.tibia.i14031'
  AND (p_outputs->0->>'count')::bigint=1
  AND ((p_kind=3 AND jsonb_typeof(p_effects->'ground')='object')
    OR (p_kind IN(1,4) AND NOT (p_effects ? 'ground')))
  AND p_effects-'ground'=jsonb_build_object(
   'kind','transform','source_item_id',13844,
   'donor','crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/ice_flower_seeds.lua',
   'cues',CASE WHEN p_kind=3 THEN '[{"binding":"appearance:effect/poff","effect_id":3}]'::jsonb ELSE '[]'::jsonb END,
   'private_speech',FALSE,'require_head',FALSE,'require_player_target',FALSE,'achievement_progress','[]'::jsonb),FALSE)
$$;
CREATE FUNCTION game_event_planting_ground(p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,
 p_before JSONB,p_effects JSONB,p_extra JSONB,p_outputs JSONB,p_recipe TEXT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT p_kind=3 AND game_event_pot_planting(p_kind,p_consume,p_count,p_before,p_effects,p_extra,p_outputs,p_recipe)
$$;

-- Preservation spends one seed and leaves the exact mature plant and custody intact.
-- Its one achievement is granted by the existing account achievement writer/event.
CREATE FUNCTION game_event_seed_preservation(p_kind INTEGER,p_consume BOOLEAN,p_count BIGINT,
 p_before JSONB,p_effects JSONB,p_extra JSONB,p_outputs JSONB,p_recipe TEXT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(p_kind IN(1,3,4) AND NOT p_consume AND p_count=0
  AND p_recipe='oteryn:planting.preserve'
  AND p_before->>'definition_family'='Item'
  AND p_before->>'definition_production_key'='oteryn:item.tibia.i14029'
  AND (p_before->>'quantity')::bigint=1
  AND jsonb_array_length(p_extra)=1 AND (p_extra->0->>'count')::bigint=1
  AND p_extra->0->'before'->>'definition_family'='Item'
  AND p_extra->0->'before'->>'definition_production_key'='oteryn:item.tibia.i13844'
  AND (p_extra->0->>'source_kind')::integer IN(1,4)
  AND p_outputs='[]'::jsonb
  AND ((p_kind=3 AND jsonb_typeof(p_effects->'ground')='object')
    OR (p_kind IN(1,4) AND NOT (p_effects ? 'ground')))
  AND p_effects-'ground'=jsonb_build_object(
   'kind','transform','source_item_id',13844,
   'donor','crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/ice_flower_seeds.lua',
   'cues',CASE WHEN p_kind=3 THEN '[{"binding":"appearance:effect/poff","effect_id":3}]'::jsonb ELSE '[]'::jsonb END,
   'private_speech',FALSE,'require_head',FALSE,'require_player_target',FALSE,'achievement_progress','[]'::jsonb,
   'achievement','oteryn:achievement/preservationist'),FALSE)
$$;
DO $migration$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_event_pot_planting','game_event_planting_ground','game_event_seed_preservation'] LOOP
  EXECUTE format('ALTER FUNCTION %I(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text) SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text) FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %I(integer,boolean,bigint,jsonb,jsonb,jsonb,jsonb,text) TO oteryn_game_runtime',f);
 END LOOP;
END $migration$;

-- A map-created pot remains an actual pickup source after its own proven timer
-- changes the definition. Retain the original unchanged-MINT arm verbatim.
DO $migration$
DECLARE body TEXT;
BEGIN
 SELECT prosrc INTO body FROM pg_proc WHERE oid='game_item_loop_map_mint_source(uuid)'::regprocedure;
 IF body IS NULL OR position('game_map_item_mint_receipts' IN body)=0 THEN
  RAISE EXCEPTION 'missing exact map-mint pickup source proof';
 END IF;
 body:=rtrim(body,E' \n\r\t;')||$arm$
 OR EXISTS(
  SELECT 1 FROM game_map_item_mint_receipts m
   JOIN game_item_instances i USING(item_instance_id)
   JOIN game_item_ground_locations g USING(item_instance_id)
   JOIN game_spell_field_temporal_schedules p
    ON p.item_instance_id=m.item_instance_id AND p.map_source_transaction_id=m.transaction_id
   JOIN game_spell_field_temporal_receipts done
    ON done.item_instance_id=p.item_instance_id AND done.source_stage=p.source_stage
  WHERE m.item_instance_id=p_item AND m.quantity=1
   AND m.definition_family='Item'
   AND m.definition_production_key IN('oteryn:item.tibia.i324','oteryn:item.tibia.i330',
       'oteryn:item.tibia.i14031','oteryn:item.tibia.i14032')
   AND i.world_id=m.world_id AND g.world_id=m.world_id AND g.channel_id=m.channel_id
   AND i.lifecycle=1 AND i.quantity=1 AND i.minted_transaction_id=m.transaction_id
   AND i.last_transaction_id=done.transaction_id AND i.state_revision=done.state_revision_before+1
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
       (p.target_family,p.target_production_key,p.target_revision)
   AND done.world_id=m.world_id AND done.channel_id=m.channel_id
   AND done.custody_kind=1 AND done.quantity_before=1 AND done.source_stack_ordinal=g.stack_ordinal
   AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts later
      WHERE later.item_instance_id=p_item AND later.source_stage>done.source_stage)
   AND g.runtime_scope_ownership_generation=m.runtime_scope_ownership_generation
   AND g.spatial_position=m.spatial_position
   AND g.corpse_ref=convert_to('map-item-materialization-v1','UTF8')
   AND g.map_revision=m.map_revision AND g.content_revision=m.content_revision
   AND g.native_room_placement_context=m.base_bundle_digest AND g.stack_ordinal>0)
$arm$;
 EXECUTE format('CREATE OR REPLACE FUNCTION game_item_loop_map_mint_source(p_item uuid) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp AS %L',current_schema(),body);
END $migration$;
