-- Execute each original floor-transform deadline once, retaining every cycle.
CREATE TABLE game_floor_transform_expiry_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 source_transaction_id UUID NOT NULL UNIQUE REFERENCES game_floor_transform_receipts,
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 before_item JSONB NOT NULL CHECK(jsonb_typeof(before_item)='object' AND octet_length(before_item::text)<=16384),
 custody JSONB NOT NULL CHECK(jsonb_typeof(custody)='object' AND custody ? 'kind' AND jsonb_typeof(custody->'kind')='number' AND (custody->>'kind')::integer BETWEEN 1 AND 3 AND custody ? 'location' AND jsonb_typeof(custody->'location')='object' AND octet_length(custody::text)<=16384),
 character_id UUID REFERENCES game_character_roots,
 game_session_id UUID,
 character_lease_generation NUMERIC(20,0),
 connection_generation NUMERIC(20,0),
 equipment_before JSONB,
 occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(((custody->>'kind')::integer=1 AND num_nonnulls(character_id,game_session_id,character_lease_generation,connection_generation,equipment_before)=0)
  OR ((custody->>'kind')::integer IN(2,3) AND num_nonnulls(character_id,game_session_id,character_lease_generation,connection_generation)=4
      AND character_lease_generation>0 AND connection_generation>0
      AND ((custody->>'kind')::integer=3)=(equipment_before IS NOT NULL)))
);
CREATE INDEX game_floor_transform_expiry_character_idx ON game_floor_transform_expiry_receipts(character_id);
CREATE TABLE game_floor_transform_expiry_audit (
 event_id UUID PRIMARY KEY REFERENCES game_floor_transform_expiry_receipts(event_id),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_floor_transform_expiry_receipts,
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 32768),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE OR REPLACE VIEW game_floor_transform_deadlines AS
 SELECT (r.target_before->'item'->>'item_instance_id')::uuid AS item_instance_id,
 r.transaction_id AS source_transaction_id,p.duration_millis,r.expires_at_unix_ms,
 p.decay_key AS target_production_key,r.content_digest
 FROM game_floor_transform_receipts r JOIN game_floor_transform_recipes p USING(recipe)
 WHERE NOT EXISTS(SELECT 1 FROM game_floor_transform_expiry_receipts done WHERE done.source_transaction_id=r.transaction_id);

CREATE FUNCTION game_floor_transform_custody_matches(p_receipt game_floor_transform_expiry_receipts) RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE item UUID:=(p_receipt.before_item->>'item_instance_id')::uuid; k INTEGER:=(p_receipt.custody->>'kind')::integer;
BEGIN
 IF k=1 THEN
  RETURN EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=item
    AND g.world_id=p_receipt.world_id AND g.channel_id=p_receipt.channel_id AND g.runtime_scope_ownership_generation=p_receipt.ownership_generation
    AND to_jsonb(g)=p_receipt.custody->'location');
 ELSIF k=2 AND COALESCE((p_receipt.custody->>'nested')::boolean,false) THEN
  RETURN EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
    WHERE n.item_instance_id=item AND n.world_id=p_receipt.world_id AND owner.owner_character=p_receipt.character_id
      AND owner.depth BETWEEN 0 AND 7 AND to_jsonb(n)=p_receipt.custody->'location');
 ELSIF k=2 THEN
  RETURN EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=item AND e.world_id=p_receipt.world_id
    AND e.character_id=p_receipt.character_id AND to_jsonb(e)=p_receipt.custody->'location');
 ELSE
  RETURN EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=item AND e.world_id=p_receipt.world_id
    AND e.character_id=p_receipt.character_id AND to_jsonb(e)=p_receipt.custody->'location');
 END IF;
END $$;

CREATE FUNCTION game_floor_transform_expiry_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source game_floor_transform_receipts%ROWTYPE; p game_floor_transform_recipes%ROWTYPE;
 m game_item_loop_move_receipts%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT source FROM game_floor_transform_receipts WHERE transaction_id=NEW.source_transaction_id;
 SELECT * INTO STRICT p FROM game_floor_transform_recipes WHERE recipe=source.recipe;
 SELECT * INTO STRICT m FROM game_item_loop_move_receipts WHERE transaction_id=source.move_transaction_id;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(NEW.before_item->>'item_instance_id')::uuid FOR UPDATE;
 IF i.item_instance_id IS NULL OR i.item_instance_id<>(source.target_before->'item'->>'item_instance_id')::uuid
   OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR i.world_id<>m.world_id OR NEW.world_id<>m.world_id
   OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item' OR i.definition_revision_ref<>'definition-r1'
   OR i.definition_production_key<>p.output_key
   OR NEW.occurred_at_unix_ms<source.expires_at_unix_ms
   OR NEW.occurred_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
   OR NOT game_floor_transform_custody_matches(NEW)
 THEN RAISE EXCEPTION 'floor expiry requires the exact original due Item and current custody' USING ERRCODE='23514'; END IF;
 IF (NEW.custody->>'kind')::integer<>1 AND NOT EXISTS(
  SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards g USING(character_id)
   JOIN game_character_roots c USING(character_id)
   WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.session_state=2
     AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.character_lease_generation
     AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id AND s.runtime_scope_channel_id=NEW.channel_id
     AND s.scope_ownership_generation=NEW.ownership_generation AND g.eligible AND g.holder_game_session_id=s.game_session_id
     AND g.lease_generation=s.character_lease_generation AND c.world_id=NEW.world_id AND c.lifecycle=1) THEN
  RAISE EXCEPTION 'floor expiry inventory requires current Character/session/scope' USING ERRCODE='23514'; END IF;
 IF (NEW.custody->>'kind')::integer=3 AND NOT EXISTS(SELECT 1 FROM game_character_equipment_state es
    WHERE es.character_id=NEW.character_id AND to_jsonb(es)=NEW.equipment_before) THEN
  RAISE EXCEPTION 'floor expiry equipment epoch differs' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_floor_transform_expiry_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_floor_transform_expiry_receipts%ROWTYPE; source game_floor_transform_receipts%ROWTYPE;
 p game_floor_transform_recipes%ROWTYPE; i game_item_instances%ROWTYPE; e game_character_equipment_state%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_floor_transform_expiry_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
 THEN RAISE EXCEPTION 'floor expiry has no current receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT source FROM game_floor_transform_receipts WHERE transaction_id=r.source_transaction_id;
 SELECT * INTO STRICT p FROM game_floor_transform_recipes WHERE recipe=source.recipe;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(r.before_item->>'item_instance_id')::uuid;
 IF i.item_instance_id IS NULL OR i.definition_production_key<>p.decay_key
   OR i.last_transaction_id IS DISTINCT FROM r.transaction_id OR i.state_revision<>(r.before_item->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['definition_production_key','state_revision','last_transaction_id']) IS DISTINCT FROM
      (r.before_item-ARRAY['definition_production_key','state_revision','last_transaction_id'])
   OR NOT game_floor_transform_custody_matches(r)
   OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
   OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
   OR NOT EXISTS(SELECT 1 FROM game_floor_transform_expiry_audit a WHERE a.transaction_id=r.transaction_id AND a.event_id=r.event_id
      AND a.created_xact_id=r.created_xact_id AND convert_from(a.envelope,'UTF8')::jsonb=
          jsonb_build_object('receipt',to_jsonb(r),'source',to_jsonb(source),'recipe',to_jsonb(p)))
 THEN RAISE EXCEPTION 'floor expiry requires exact same identity/custody/successor/audit' USING ERRCODE='23514'; END IF;
 IF (r.custody->>'kind')::integer=3 THEN
  SELECT * INTO e FROM game_character_equipment_state WHERE character_id=r.character_id;
  IF e.character_id IS NULL OR e.revision<>(r.equipment_before->>'revision')::numeric+1 OR e.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(e)-ARRAY['revision','last_transaction_id']) IS DISTINCT FROM (r.equipment_before-ARRAY['revision','last_transaction_id'])
  THEN RAISE EXCEPTION 'floor expiry must advance actual equipment epoch' USING ERRCODE='23514'; END IF;
 END IF;
END $$;
CREATE FUNCTION game_floor_transform_expiry_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_floor_transform_expiry_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_floor_transform_expiry_admit BEFORE INSERT ON game_floor_transform_expiry_receipts
 FOR EACH ROW EXECUTE FUNCTION game_floor_transform_expiry_admit();
CREATE CONSTRAINT TRIGGER game_floor_transform_expiry_consistent AFTER INSERT ON game_floor_transform_expiry_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_floor_transform_expiry_consistent();

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing floor expiry predecessor %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_floor_transform_expiry_receipts r
      WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     IF (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id']) IS DISTINCT FROM
        (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id'])
     THEN RAISE EXCEPTION 'floor expiry changed unrelated Item state' USING ERRCODE='23514'; END IF;
     NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;
   $arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_floor_transform_expiry_receipts r WHERE r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     PERFORM game_floor_transform_expiry_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
   $arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 END LOOP;
 -- A later recharge needs the prior cycle's immutable expiry receipt.
 body:=pg_get_functiondef('game_floor_transform_admit()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing floor cycle predecessor'; END IF;
 arm:=$arm$
  IF EXISTS(SELECT 1 FROM game_floor_transform_receipts prior
    WHERE prior.target_before->'item'->>'item_instance_id'=NEW.target_before->'item'->>'item_instance_id'
     AND NOT EXISTS(SELECT 1 FROM game_floor_transform_expiry_receipts done WHERE done.source_transaction_id=prior.transaction_id)) THEN
   RAISE EXCEPTION 'floor transform already has an unexpired original cycle' USING ERRCODE='23514';
  END IF;
 $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 FOREACH n IN ARRAY ARRAY['game_floor_transform_expiry_receipts','game_floor_transform_expiry_audit'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n||'_immutable',n);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_item_reject_truncate()',n||'_no_truncate',n);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',n);
  EXECUTE format('GRANT SELECT,INSERT ON %I TO oteryn_game_runtime',n);
  EXECUTE format('GRANT SELECT ON %I TO oteryn_game_control',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_floor_transform_custody_matches(game_floor_transform_expiry_receipts)',
   'game_floor_transform_expiry_admit()','game_floor_transform_expiry_complete(uuid)','game_floor_transform_expiry_consistent()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $migration$;

DO $migration$
DECLARE body TEXT; arm TEXT; at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing floor expiry equipment predecessor'; END IF;
 arm:=$arm$
  IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_floor_transform_expiry_receipts r
    WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
     AND (r.custody->>'kind')::integer=3 AND r.equipment_before=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
   PERFORM game_floor_transform_expiry_complete(NEW.last_transaction_id); RETURN NULL;
  END IF;
 $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
END $migration$;
