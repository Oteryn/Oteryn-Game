-- Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:30-55.
-- The exact stack in the authenticated move retires with its audit and, when
-- occupied, the other player's Quest update and pending landing.
CREATE TABLE game_coal_basin_arrivals (
    transaction_id uuid PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id uuid NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    move_transaction_id uuid NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
    item_instance_id uuid NOT NULL REFERENCES game_item_instances,
    source_uid integer NOT NULL CHECK (source_uid BETWEEN 9033 AND 9040),
    before_item jsonb NOT NULL CHECK (jsonb_typeof(before_item)='object'),
    before_ground jsonb NOT NULL CHECK (jsonb_typeof(before_ground)='object'),
    placement bytea NOT NULL CHECK (octet_length(placement)=40),
    reset_epoch bigint NOT NULL CHECK (reset_epoch>=0),
    recipient uuid REFERENCES game_character_roots,
    destination_x integer CHECK (destination_x BETWEEN 0 AND 65535),
    destination_y integer CHECK (destination_y BETWEEN 0 AND 65535),
    destination_floor smallint CHECK (destination_floor BETWEEN 0 AND 15),
    quest_changed boolean NOT NULL,
    occurred_at bigint NOT NULL CHECK (occurred_at>0),
    envelope_sha256 bytea NOT NULL CHECK (octet_length(envelope_sha256)=32),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((recipient IS NULL AND NOT quest_changed AND num_nonnulls(destination_x,destination_y,destination_floor)=0)
        OR (recipient IS NOT NULL AND num_nonnulls(destination_x,destination_y,destination_floor)=3))
);
CREATE INDEX game_coal_basin_arrivals_item ON game_coal_basin_arrivals(item_instance_id);

ALTER TABLE game_character_pending_arrivals
    ADD COLUMN source_coal_basin boolean NOT NULL DEFAULT false;

DO $migration$
DECLARE name text; prior text;
BEGIN
    FOREACH name IN ARRAY ARRAY[
        'game_character_quest_receipts_cause_kind_check',
        'game_character_quest_receipt_cause_shape'
    ] LOOP
        SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
            WHERE conrelid='game_character_quest_receipts'::regclass AND conname=name;
        IF left(prior,6)<>'CHECK ' OR strpos(prior,'source_step')=0
            OR strpos(prior,'source_item_arrival')<>0 THEN
            RAISE EXCEPTION 'unexpected coal basin cause constraint predecessor: %',name;
        END IF;
        EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I',name);
        EXECUTE format('ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I CHECK (((cause_kind<>''source_item_arrival'' AND %s) OR (cause_kind=''source_item_arrival'' AND cause_ordinal=0 AND game_character_is_uuid_v7(cause_id))) IS TRUE)',name,substr(prior,7));
    END LOOP;
END $migration$;

CREATE FUNCTION game_coal_basin_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_xact_id:=pg_current_xact_id();
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
        JOIN game_item_loop_move_lines l USING(item_instance_id)
        JOIN game_item_loop_move_receipts m USING(transaction_id)
        WHERE i.item_instance_id=NEW.item_instance_id AND to_jsonb(i)=NEW.before_item
          AND to_jsonb(g)=NEW.before_ground AND i.definition_family='Item'
          AND i.definition_production_key='oteryn:item.tibia.i3042'
          AND i.definition_revision_ref='definition-r1' AND i.lifecycle=1
          AND i.quantity=l.quantity_after AND i.quantity>0
          AND m.transaction_id=NEW.move_transaction_id
          AND m.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
          AND l.to_kind=3 AND l.to_spatial_position=g.spatial_position
          AND ((l.from_kind=0 AND i.minted_transaction_id=m.transaction_id
              AND i.last_transaction_id IS NULL AND i.state_revision=1)
            OR (l.from_kind<>0 AND i.last_transaction_id=m.transaction_id
              AND i.state_revision=l.state_revision_before+1))
          AND g.world_id=m.world_id AND g.channel_id=m.channel_id
          AND g.runtime_scope_ownership_generation=m.scope_generation
          AND g.content_revision='sha256:'||encode(m.content_digest,'hex')
          AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(m.transaction_id)
          AND g.spatial_position=int4send(get_byte(NEW.placement,34)*256+get_byte(NEW.placement,35))
              ||int4send(get_byte(NEW.placement,36)*256+get_byte(NEW.placement,37))
              ||int2send((-get_byte(NEW.placement,38))::smallint)
          AND game_world_reset_map_epoch_current(m.world_id,NEW.reset_epoch::numeric)
    ) THEN
        RAISE EXCEPTION 'coal basin requires its exact newly arrived coin stack' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION game_coal_basin_complete(p_transaction uuid) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_coal_basin_arrivals%ROWTYPE; i game_item_instances%ROWTYPE; world uuid;
BEGIN
    SELECT * INTO STRICT r FROM game_coal_basin_arrivals WHERE transaction_id=p_transaction;
    SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=r.item_instance_id;
    SELECT world_id INTO STRICT world FROM game_item_loop_move_receipts WHERE transaction_id=r.move_transaction_id;
    IF r.created_xact_id<>pg_current_xact_id() OR i.quantity<>0 OR i.lifecycle<>2
        OR i.last_transaction_id<>r.transaction_id
        OR i.state_revision<>(r.before_item->>'state_revision')::numeric+1
        OR (to_jsonb(i)-ARRAY['quantity','lifecycle','state_revision','last_transaction_id']) IS DISTINCT FROM
           (r.before_item-ARRAY['quantity','lifecycle','state_revision','last_transaction_id'])
        OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
        OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
        OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=r.event_id
            AND a.transaction_id=r.transaction_id AND a.item_instance_id=r.item_instance_id
            AND a.envelope_sha256=r.envelope_sha256 AND a.occurred_at=r.occurred_at
            AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1
            AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL)
        OR (r.recipient IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM game_character_pending_arrivals a JOIN game_character_roots c USING(character_id)
            WHERE a.character_id=r.recipient AND a.occurrence_id=r.move_transaction_id
                AND a.world_id=world AND c.world_id=world AND a.source_coal_basin
                AND (a.tile_x,a.tile_y,a.tile_floor)=(r.destination_x,r.destination_y,r.destination_floor)
        ))
        OR r.quest_changed IS DISTINCT FROM EXISTS (
            SELECT 1 FROM game_character_quest_receipts q WHERE q.character_id=r.recipient
                AND q.cause_id=r.move_transaction_id AND q.cause_kind='source_item_arrival'
                AND q.quest_key='oteryn:quest.the_ancient_tombs_quest'
                AND q.transition_key='oteryn:quest-callback/ancient-tombs-coal-basin'
                AND NOT q.completes AND q.created_xact_id=pg_current_xact_id()
        )
    THEN
        RAISE EXCEPTION 'coal basin requires its debit, audit and atomic recipient outcome' USING ERRCODE='23514';
    END IF;
END $$;

CREATE FUNCTION game_coal_basin_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM game_coal_basin_complete(NEW.transaction_id);
    RETURN NULL;
END $$;

CREATE FUNCTION game_coal_basin_arrival_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='UPDATE' AND OLD.source_coal_basin THEN
        IF NEW IS NOT DISTINCT FROM OLD THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'coal basin landing is immutable until consumed' USING ERRCODE='23514';
    END IF;
    IF NEW.source_coal_basin AND NOT EXISTS (
        SELECT 1 FROM game_coal_basin_arrivals r JOIN game_item_loop_move_receipts m
          ON m.transaction_id=r.move_transaction_id
        WHERE r.recipient=NEW.character_id AND r.move_transaction_id=NEW.occurrence_id
            AND m.world_id=NEW.world_id AND r.created_xact_id=pg_current_xact_id()
            AND (r.destination_x,r.destination_y,r.destination_floor)=(NEW.tile_x,NEW.tile_y,NEW.tile_floor)
    ) THEN
        RAISE EXCEPTION 'coal basin landing requires its same-transaction sacrifice' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_coal_basin_quest_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_coal_basin_arrivals r
        WHERE r.recipient=NEW.character_id AND r.move_transaction_id=NEW.cause_id
          AND r.quest_changed AND r.created_xact_id=pg_current_xact_id()
          AND NOT EXISTS (
            SELECT 1 FROM unnest(NEW.track_keys,NEW.values_before,NEW.values_after) AS t(key,before,after)
            WHERE after<>1 OR CASE key
                WHEN 'oteryn:quest-progress/quest/u7_4/the_ancient_tombs/default_start' THEN before=1
                WHEN 'oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diphtrahs_treasure' THEN r.source_uid<>9035 OR before>0
                WHEN 'oteryn:quest-track/ancient-tombs/diprath-treasure' THEN r.source_uid<>9035 OR before>0
                WHEN 'oteryn:quest-progress/quest/u7_4/the_ancient_tombs/mahrdis_treasure' THEN r.source_uid<>9036 OR before>0
                ELSE true
            END
          )
    ) OR NEW.transition_key<>'oteryn:quest-callback/ancient-tombs-coal-basin'
      OR NEW.quest_key<>'oteryn:quest.the_ancient_tombs_quest' OR NEW.completes
    THEN
        RAISE EXCEPTION 'coal basin quest receipt requires its same-transaction sacrifice' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER game_coal_basin_admit BEFORE INSERT ON game_coal_basin_arrivals
    FOR EACH ROW EXECUTE FUNCTION game_coal_basin_admit();
CREATE CONSTRAINT TRIGGER game_coal_basin_consistent AFTER INSERT ON game_coal_basin_arrivals
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_coal_basin_consistent();
CREATE CONSTRAINT TRIGGER game_coal_basin_arrival_proven AFTER INSERT OR UPDATE ON game_character_pending_arrivals
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_coal_basin_arrival_proven();
CREATE CONSTRAINT TRIGGER game_coal_basin_quest_proven AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.cause_kind='source_item_arrival')
    EXECUTE FUNCTION game_coal_basin_quest_proven();
CREATE TRIGGER game_coal_basin_immutable BEFORE UPDATE OR DELETE ON game_coal_basin_arrivals
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_coal_basin_no_truncate BEFORE TRUNCATE ON game_coal_basin_arrivals
    EXECUTE FUNCTION game_item_reject_truncate();

DO $migration$
DECLARE body text; anchor text; arm text; at integer; signature text;
BEGIN
    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    anchor:='        UNION ALL SELECT 1 FROM game_dreamer_ring_offerings WHERE transaction_id=NEW.last_transaction_id';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'coal basin item guard classification predecessor missing';
    END IF;
    body:=replace(body,anchor,anchor||E'\n        UNION ALL SELECT 1 FROM game_coal_basin_arrivals WHERE transaction_id=NEW.last_transaction_id');
    anchor:='    IF TG_OP=''UPDATE'' AND EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'coal basin item guard retirement predecessor missing';
    END IF;
    arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_coal_basin_arrivals r
        WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id
          AND r.created_xact_id=pg_current_xact_id() AND r.before_item=v_instance_guard_old_json) THEN
        IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR
           (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
           (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
            RAISE EXCEPTION 'coal basin retires only its exact coin stack' USING ERRCODE='23514';
        END IF;
        NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;
$arm$;
    EXECUTE replace(body,anchor,arm||anchor);

    body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
    at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at=0 THEN RAISE EXCEPTION 'coal basin item proof predecessor missing'; END IF;
    arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_coal_basin_arrivals r WHERE r.transaction_id=NEW.last_transaction_id
        AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
        PERFORM game_coal_basin_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);

    body:=pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure);
    at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at=0 THEN RAISE EXCEPTION 'coal basin ground proof predecessor missing'; END IF;
    arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_coal_basin_arrivals r WHERE r.item_instance_id=OLD.item_instance_id
        AND r.created_xact_id=pg_current_xact_id() AND r.before_ground=to_jsonb(OLD)) THEN
        PERFORM game_coal_basin_complete((SELECT transaction_id FROM game_coal_basin_arrivals
            WHERE item_instance_id=OLD.item_instance_id AND created_xact_id=pg_current_xact_id()));
        RETURN NULL;
    END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);

    FOREACH signature IN ARRAY ARRAY['game_coal_basin_admit()','game_coal_basin_complete(uuid)',
        'game_coal_basin_consistent()','game_coal_basin_arrival_proven()','game_coal_basin_quest_proven()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
    END LOOP;
END $migration$;

REVOKE ALL ON game_coal_basin_arrivals FROM PUBLIC;
GRANT SELECT,INSERT ON game_coal_basin_arrivals TO oteryn_game_runtime;
GRANT SELECT ON game_coal_basin_arrivals TO oteryn_game_control;
