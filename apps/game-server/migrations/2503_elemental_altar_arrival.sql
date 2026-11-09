-- M49: an ordinary item arrival, complete corpse retirement, audit and pending
-- boss latch share one physical transaction. Existing custody guards stay intact.
CREATE TABLE game_elemental_altar_rules (
 rule TEXT PRIMARY KEY, corpse_key TEXT NOT NULL UNIQUE, creature_key TEXT NOT NULL UNIQUE,
 corpse_duration_ms BIGINT NOT NULL CHECK(corpse_duration_ms>0)
);
INSERT INTO game_elemental_altar_rules VALUES
 ('oteryn:item-arrival/elemental_spheres/energy_overlord','oteryn:item.tibia.i8139','oteryn:creature.energy_overlord',500000),
 ('oteryn:item-arrival/elemental_spheres/fire_overlord','oteryn:item.tibia.i8141','oteryn:creature.fire_overlord',500000),
 ('oteryn:item-arrival/elemental_spheres/ice_overlord','oteryn:item.tibia.i8140','oteryn:creature.ice_overlord',500000),
 ('oteryn:item-arrival/elemental_spheres/earth_overlord','oteryn:item.tibia.i8106','oteryn:creature.earth_overlord',300000);
CREATE TABLE game_elemental_altar_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts,
 rule TEXT NOT NULL REFERENCES game_elemental_altar_rules,
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 readset JSONB NOT NULL CHECK(jsonb_typeof(readset)='array' AND jsonb_array_length(readset)=8),
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs) BETWEEN 1 AND 500),
 inputs_intent BYTEA NOT NULL CHECK(octet_length(inputs_intent) BETWEEN 1 AND 2097152
   AND convert_from(inputs_intent,'UTF8')::jsonb=inputs),
 inputs_sha256 BYTEA NOT NULL CHECK(inputs_sha256=sha256(inputs_intent)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_elemental_altar_receipt_rule_idx ON game_elemental_altar_receipts(rule);
CREATE TABLE game_elemental_boss_latches (
 world_id UUID NOT NULL, channel_id UUID NOT NULL, rule TEXT NOT NULL REFERENCES game_elemental_altar_rules,
 reset_epoch BIGINT NOT NULL CHECK(reset_epoch>=0),
 arrival_transaction_id UUID NOT NULL UNIQUE REFERENCES game_elemental_altar_receipts,
 PRIMARY KEY(world_id,channel_id,rule,reset_epoch)
);

CREATE INDEX game_elemental_boss_latch_rule_idx ON game_elemental_boss_latches(rule);

-- Only the four source-qualified corpse roots may use this added ground lane.
-- D134 stays intact: even an emptied/re-dropped corpse cannot enter inventory.
CREATE FUNCTION game_elemental_corpse_ground_source(p_item UUID) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_mint_receipts m JOIN game_item_instances i USING(item_instance_id)
   JOIN game_item_ground_locations g USING(item_instance_id)
   JOIN game_elemental_altar_rules p ON p.corpse_key=i.definition_production_key
   WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
     AND i.minted_transaction_id=m.transaction_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
     AND m.draw_ordinal=0 AND m.materialized_at IS NOT NULL
     AND m.materialized_at+p.corpse_duration_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
     AND g.scope_kind=1 AND g.instance_id IS NULL AND g.world_id=i.world_id
     AND m.death_world_id=g.world_id AND m.death_channel_id=g.channel_id
     AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation
     AND EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=m.transaction_id
       AND a.event_id=m.event_id AND a.item_instance_id=m.item_instance_id AND a.envelope_sha256=m.envelope_sha256)
     AND EXISTS(SELECT 1 FROM game_item_mint_reservations v WHERE v.transaction_id=m.transaction_id
       AND v.item_instance_id=m.item_instance_id AND v.event_id=m.event_id AND v.intent_binding=m.intent_binding
       AND sha256(v.envelope)=m.envelope_sha256))
$$;
DO $migration$
DECLARE body TEXT; original TEXT; at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='game_item_loop_map_mint_source(NEW.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing altar corpse predecessor'; END IF;
 body:=replace(body,original,'('||original||' OR (NEW.to_kind=3 AND g.runtime_scope_ownership_generation=r.scope_generation AND game_elemental_corpse_ground_source(NEW.item_instance_id)))');
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing altar corpse source guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
   IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION') THEN
     RAISE EXCEPTION 'D134 corpse roots remain outside carried custody' USING ERRCODE='23514';
   END IF;
 $arm$ FROM at FOR 5);
 EXECUTE format('ALTER FUNCTION game_elemental_corpse_ground_source(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_elemental_corpse_ground_source(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_elemental_corpse_ground_source(UUID) TO oteryn_game_runtime;

CREATE FUNCTION game_elemental_altar_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE m game_item_loop_move_receipts%ROWTYPE; p game_elemental_altar_rules%ROWTYPE;
 v JSONB; actual JSONB; root UUID; ids UUID[]; reached UUID[];
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=NEW.move_transaction_id;
 SELECT * INTO STRICT p FROM game_elemental_altar_rules WHERE rule=NEW.rule;
 root:=(NEW.inputs->0->'item'->>'item_instance_id')::uuid;
 SELECT array_agg((x->'item'->>'item_instance_id')::uuid ORDER BY x->'item'->>'item_instance_id') INTO ids
   FROM jsonb_array_elements(NEW.inputs) x;
 IF m.created_xact_id<>pg_current_xact_id() OR m.operation NOT IN(1,4)
   OR NOT game_world_reset_map_epoch_current(m.world_id,NEW.reset_epoch)
   OR cardinality(ids)<>(SELECT count(DISTINCT x) FROM unnest(ids) x)
   OR NEW.inputs->0->'item'->>'definition_production_key' IS DISTINCT FROM p.corpse_key
   OR NOT EXISTS(SELECT 1 FROM game_item_mint_receipts WHERE item_instance_id=root AND loot_purpose_key='CORPSE_MATERIALIZATION')
   OR NOT EXISTS(SELECT 1 FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id)
     JOIN game_item_ground_locations g USING(item_instance_id)
     WHERE l.transaction_id=m.transaction_id AND l.created_xact_id=m.created_xact_id
       AND l.item_instance_id=root AND l.to_kind=3
       AND ((m.operation=4 AND l.from_kind IN(1,2)) OR (m.operation=1 AND l.from_kind=3))
       AND l.quantity_before=1 AND l.quantity_after=1 AND i.lifecycle=1 AND i.quantity=1
       AND i.state_revision=l.state_revision_before+1 AND i.last_transaction_id=m.transaction_id
       AND (l.definition_family,l.definition_production_key,l.definition_revision_ref)=('Item',p.corpse_key,i.definition_revision_ref)
       AND l.to_spatial_position=g.spatial_position
       AND g.world_id=m.world_id AND g.channel_id=m.channel_id AND g.scope_kind=1 AND g.instance_id IS NULL
       AND g.runtime_scope_ownership_generation=m.scope_generation AND g.stack_ordinal IS NOT NULL
       AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations other JOIN game_item_instances live USING(item_instance_id)
         WHERE other.world_id=g.world_id AND other.channel_id=g.channel_id AND other.spatial_position=g.spatial_position
           AND live.lifecycle=1 AND other.stack_ordinal>g.stack_ordinal))
   OR (SELECT count(DISTINCT x->>0) FROM jsonb_array_elements(NEW.readset) x)<>8
   OR EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.readset) x WHERE jsonb_typeof(x)<>'array'
      OR jsonb_array_length(x)<>2 OR x->>1 IS NULL OR jsonb_typeof(x->1)<>'number'
      OR (x->>0)::numeric NOT BETWEEN 1 AND 18446744073709551615
      OR (x->>1)::numeric NOT BETWEEN 0 AND 18446744073709551614)
 THEN RAISE EXCEPTION 'altar arrival requires a fresh exact corpse drop and complete readset' USING ERRCODE='23514'; END IF;
 -- UNION, rather than UNION ALL, bounds even a malformed cyclic custody graph.
 WITH RECURSIVE tree(id) AS (
   SELECT root UNION SELECT e.item_instance_id FROM tree t JOIN
    (SELECT item_instance_id,parent_item_instance_id FROM game_item_corpse_container_entries
     UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries) e
    ON e.parent_item_instance_id=t.id
 ) SELECT array_agg(id ORDER BY id::text) INTO reached FROM (SELECT id FROM tree LIMIT 501) bounded;
 IF ids IS DISTINCT FROM reached THEN
   RAISE EXCEPTION 'altar retirement must cover the whole actual corpse tree' USING ERRCODE='23514';
 END IF;
 FOR v IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
   SELECT to_jsonb(i) INTO actual FROM game_item_instances i WHERE item_instance_id=(v->'item'->>'item_instance_id')::uuid FOR UPDATE;
   IF actual IS DISTINCT FROM v->'item' OR actual->>'lifecycle'<>'1' OR (actual->>'world_id')::uuid<>m.world_id
      OR (actual->>'state_revision')::numeric>=18446744073709551615
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=(actual->>'item_instance_id')::uuid)
   THEN RAISE EXCEPTION 'altar input is not the exact live corpse member' USING ERRCODE='23514'; END IF;
   IF (actual->>'item_instance_id')::uuid=root THEN
     SELECT to_jsonb(g) INTO actual FROM game_item_ground_locations g WHERE item_instance_id=root;
   ELSIF v->>'kind'='corpse' THEN
     SELECT to_jsonb(e) INTO actual FROM game_item_corpse_container_entries e WHERE item_instance_id=(actual->>'item_instance_id')::uuid;
   ELSIF v->>'kind'='container' THEN
     SELECT to_jsonb(e) INTO actual FROM game_item_container_entries e WHERE item_instance_id=(actual->>'item_instance_id')::uuid;
   ELSE RAISE EXCEPTION 'altar member custody kind' USING ERRCODE='23514'; END IF;
   IF actual IS NULL OR actual IS DISTINCT FROM v->'custody' THEN
     RAISE EXCEPTION 'altar member custody changed' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;

CREATE FUNCTION game_elemental_altar_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_elemental_altar_receipts%ROWTYPE; m game_item_loop_move_receipts%ROWTYPE;
 v JSONB; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_elemental_altar_receipts WHERE transaction_id=p_transaction;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=r.move_transaction_id;
 IF r.created_xact_id<>pg_current_xact_id() OR m.created_xact_id<>r.created_xact_id THEN
   RAISE EXCEPTION 'altar retirement must share the physical move transaction' USING ERRCODE='23514'; END IF;
 FOR v IN SELECT value FROM jsonb_array_elements(r.inputs) LOOP
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(v->'item'->>'item_instance_id')::uuid;
   IF i.item_instance_id IS NULL OR i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
     OR i.state_revision<>(v->'item'->>'state_revision')::numeric+1
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
         ((v->'item')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
     OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=i.item_instance_id OR parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id OR parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'altar member did not retire exactly once' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>jsonb_array_length(r.inputs)
   OR NOT EXISTS(SELECT 1 FROM game_elemental_boss_latches l WHERE l.arrival_transaction_id=r.transaction_id
     AND l.world_id=m.world_id AND l.channel_id=m.channel_id AND l.rule=r.rule AND l.reset_epoch=r.reset_epoch)
   OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
   OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
     AND a.item_instance_id=(r.inputs->0->'item'->>'item_instance_id')::uuid AND a.envelope_sha256=r.envelope_sha256
     AND a.created_xact_id=r.created_xact_id AND a.transaction_ordinal=1 AND a.transaction_count=1
     AND a.occurred_at=r.occurred_at AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'altar retirement requires its unique audit and pending boss latch' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_elemental_altar_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_elemental_altar_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE FUNCTION game_elemental_latch_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_elemental_altar_receipts r JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id
    WHERE r.transaction_id=NEW.arrival_transaction_id AND r.created_xact_id=pg_current_xact_id()
      AND (m.world_id,m.channel_id,r.rule,r.reset_epoch)=(NEW.world_id,NEW.channel_id,NEW.rule,NEW.reset_epoch))
 THEN RAISE EXCEPTION 'boss latch requires its current arrival' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_elemental_altar_admit BEFORE INSERT ON game_elemental_altar_receipts
 FOR EACH ROW EXECUTE FUNCTION game_elemental_altar_admit();
CREATE CONSTRAINT TRIGGER game_elemental_altar_consistent AFTER INSERT ON game_elemental_altar_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_altar_consistent();
CREATE TRIGGER game_elemental_latch_admit BEFORE INSERT ON game_elemental_boss_latches
 FOR EACH ROW EXECUTE FUNCTION game_elemental_latch_admit();
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
   'game_item_ground_removal_proven','game_item_corpse_entry_removal_proven','game_item_container_entry_removal_proven'] LOOP
   body:=pg_get_functiondef((n||'()')::regprocedure);
   at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF at=0 THEN RAISE EXCEPTION 'missing altar predecessor guard %',n; END IF;
   IF n='game_item_instance_guard' THEN
     arm:=$arm$
     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
         RAISE EXCEPTION 'altar retires whole exact corpse members' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;
     $arm$;
   ELSIF n='game_item_instance_change_proven' THEN
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_altar_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
   ELSE
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.created_xact_id=pg_current_xact_id() AND v->'custody'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_altar_complete((SELECT r.transaction_id FROM game_elemental_altar_receipts r,
         LATERAL jsonb_array_elements(r.inputs) v WHERE r.created_xact_id=pg_current_xact_id() AND v->'custody'=to_jsonb(OLD)));
       RETURN NULL;
     END IF;
     $arm$;
   END IF;
   EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_elemental_altar_admit()','game_elemental_altar_complete(uuid)',
   'game_elemental_altar_consistent()','game_elemental_latch_admit()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_elemental_altar_rules','game_elemental_altar_receipts','game_elemental_boss_latches'] LOOP
   EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n||'_immutable',n);
   EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_item_reject_truncate()',n||'_no_truncate',n);
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_altar_rules,game_elemental_altar_receipts,game_elemental_boss_latches FROM PUBLIC;
REVOKE ALL ON FUNCTION game_elemental_altar_admit(),game_elemental_altar_complete(UUID),game_elemental_altar_consistent(),game_elemental_latch_admit() FROM PUBLIC;
GRANT SELECT ON game_elemental_altar_rules TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_elemental_altar_receipts,game_elemental_boss_latches TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_altar_receipts,game_elemental_boss_latches TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_elemental_altar_complete(UUID) TO oteryn_game_runtime;
