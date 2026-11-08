-- W18: extend the existing House consume/transform owner. Source rules remain
-- generated in 1161; no Channel item or parallel Item writer is introduced.
-- Crystal wall_mirror.lua:17-24; magic_tree.lua:18-37; carpets.lua:185-201.
CREATE FUNCTION game_house_decoration_pair(p_source TEXT,p_target TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT FALSE $$;

CREATE TABLE game_house_decoration_cooldowns (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 cooldown_key TEXT NOT NULL CHECK(length(cooldown_key) BETWEEN 1 AND 64),
 until_ms BIGINT NOT NULL CHECK(until_ms>0),
 transaction_id UUID NOT NULL REFERENCES game_item_consume_transform_records,
 PRIMARY KEY(character_id,cooldown_key)
);
CREATE INDEX game_house_decoration_cooldowns_transaction ON game_house_decoration_cooldowns(transaction_id);
REVOKE ALL ON game_house_decoration_cooldowns FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_decoration_cooldowns TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_decoration_cooldown_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; c JSONB;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'decoration cooldown history cannot be deleted' USING ERRCODE='23514'; END IF;
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=NEW.transaction_id;
 c:=r.effects->'decoration_cooldown';
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR r.source_kind<>5
  OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration'
  OR NEW.character_id<>r.character_id OR NEW.cooldown_key IS DISTINCT FROM c->>'key'
  OR NEW.until_ms IS DISTINCT FROM (c->>'until')::bigint
  OR (TG_OP='INSERT' AND c->'before' IS DISTINCT FROM 'null'::jsonb)
 THEN RAISE EXCEPTION 'decoration cooldown requires its exact current receipt' USING ERRCODE='23514'; END IF;
 IF TG_OP='UPDATE' AND (NEW.character_id<>OLD.character_id OR NEW.cooldown_key<>OLD.cooldown_key
  OR OLD.until_ms IS DISTINCT FROM (c->>'before')::bigint)
 THEN RAISE EXCEPTION 'decoration cooldown predecessor differs' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_decoration_cooldown_guard BEFORE INSERT OR UPDATE OR DELETE
 ON game_house_decoration_cooldowns FOR EACH ROW EXECUTE FUNCTION game_house_decoration_cooldown_guard();

CREATE FUNCTION game_house_decoration_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; remove_source BOOLEAN; mutate_source BOOLEAN; c JSONB; previous BIGINT;
BEGIN
 PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
  (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
  (r.audit_context->>'scope_generation')::numeric,(r.source_location->>'node')::uuid,
  (r.source_location->>'node_revision')::numeric,r.world_id,r.source_location->'house'->>'house_key');
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 remove_source:=(r.effects->>'house_remove')::boolean;
 mutate_source:=(r.effects->>'house_mutate')::boolean;
 IF r.source_kind<>5 OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration'
  OR r.recipe NOT LIKE 'oteryn:house-decoration.%' OR i.item_instance_id IS NULL
  OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item' OR i.world_id<>r.world_id
  OR to_jsonb(i) IS DISTINCT FROM r.before_item OR r.before_secondary IS DISTINCT FROM r.before_item
  OR r.parent_item_instance_id IS NOT NULL OR r.secondary_item_instance_id<>r.source_item_instance_id
  OR r.outputs<>'[]'::jsonb OR r.extra_inputs<>'[]'::jsonb OR r.content_digest IS NULL
  OR remove_source IS NULL OR mutate_source IS NULL OR (remove_source AND NOT mutate_source)
  OR r.consume_source IS DISTINCT FROM remove_source OR r.source_count<>(CASE WHEN remove_source THEN 1 ELSE 0 END)
  OR (mutate_source AND i.state_revision>=18446744073709551615)
  OR r.effects->'house_after'->>'revision' IS DISTINCT FROM i.definition_revision_ref
  OR game_house_decoration_pair(i.definition_production_key,r.effects->'house_after'->>'key') IS DISTINCT FROM TRUE
  OR game_item_action_house_custody(r) IS DISTINCT FROM TRUE
  OR (NOT mutate_source AND r.effects->'house_after'->>'key' IS DISTINCT FROM i.definition_production_key)
  OR (remove_source AND r.effects->>'decoration_family' NOT IN('carpet','bed_kit'))
  OR (remove_source AND r.source_location->>'custody_kind' NOT IN('house','container','nested'))
 THEN RAISE EXCEPTION 'House decoration requires exact native recipe, source and current custody' USING ERRCODE='23514'; END IF;
 c:=r.effects->'decoration_cooldown';
 IF c IS NOT NULL AND c<>'null'::jsonb THEN
  SELECT until_ms INTO previous FROM game_house_decoration_cooldowns
   WHERE character_id=r.character_id AND cooldown_key=c->>'key' FOR UPDATE;
  IF to_jsonb(previous) IS DISTINCT FROM NULLIF(c->'before','null'::jsonb)
   OR (c->>'until')::bigint<=0 OR r.effects->>'decoration_family' NOT IN('mirror','magic_tree')
   OR (r.effects->>'decoration_family'='mirror' AND c->>'key'<>'wall-mirror')
   OR (r.effects->>'decoration_family'='magic_tree' AND c->>'key' !~ '^tree\.[0-9]+$')
  THEN RAISE EXCEPTION 'House decoration cooldown state differs' USING ERRCODE='23514'; END IF;
 END IF;
END $$;

CREATE FUNCTION game_house_decoration_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; i game_item_instances%ROWTYPE; expected JSONB; removed BOOLEAN; changed BOOLEAN; c JSONB;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 removed:=(r.effects->>'house_remove')::boolean; changed:=(r.effects->>'house_mutate')::boolean;
 expected:=r.before_item;
 IF changed THEN expected:=expected||jsonb_build_object('definition_production_key',r.effects->'house_after'->>'key',
  'last_transaction_id',r.transaction_id,'state_revision',(r.before_item->>'state_revision')::numeric+1); END IF;
 IF removed THEN expected:=expected||jsonb_build_object('quantity',0,'lifecycle',2); END IF;
 IF r.transaction_id IS NULL OR r.source_kind<>5 OR r.created_xact_id<>pg_current_xact_id()
  OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration' OR to_jsonb(i) IS DISTINCT FROM expected
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN changed THEN 1 ELSE 0 END)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id AND event_id=r.event_id
    AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
    AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
    AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'House decoration source/audit completion differs' USING ERRCODE='23514'; END IF;
 IF removed THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=r.source_item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.source_item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.source_item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=r.source_item_instance_id)
  THEN RAISE EXCEPTION 'House decoration retirement retained custody' USING ERRCODE='23514'; END IF;
 ELSIF game_item_action_house_custody(r) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'House decoration changed source custody' USING ERRCODE='23514';
 END IF;
 c:=r.effects->'decoration_cooldown';
 IF c IS NOT NULL AND c<>'null'::jsonb AND NOT EXISTS(SELECT 1 FROM game_house_decoration_cooldowns
  WHERE character_id=r.character_id AND cooldown_key=c->>'key' AND until_ms=(c->>'until')::bigint AND transaction_id=r.transaction_id)
 THEN RAISE EXCEPTION 'House decoration cooldown missing' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_house_decoration_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; c JSONB;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'House decoration requires current receipt' USING ERRCODE='23514'; END IF;
 PERFORM game_house_decoration_admit(r);
 IF r.effects->'base_changes' IS NOT NULL AND r.effects->'base_changes'<>'null'::jsonb THEN
  PERFORM game_house_base_apply_changes(r.transaction_id,r.effects->'base_changes');
 END IF;
 IF (r.effects->>'house_mutate')::boolean THEN
  UPDATE game_item_instances SET definition_production_key=r.effects->'house_after'->>'key',
   quantity=CASE WHEN r.consume_source THEN 0 ELSE quantity END,lifecycle=CASE WHEN r.consume_source THEN 2 ELSE lifecycle END,
   state_revision=state_revision+1,last_transaction_id=r.transaction_id WHERE item_instance_id=r.source_item_instance_id;
 END IF;
 IF r.effects->'timed_decoration' IS NOT NULL AND r.effects->'timed_decoration'<>'null'::jsonb THEN
  PERFORM game_house_decoration_timed_apply(r.transaction_id);
 END IF;
 IF r.consume_source THEN
  CASE r.source_location->>'custody_kind'
   WHEN 'house' THEN
    DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.source_item_instance_id;
    DELETE FROM game_item_house_interior_locations WHERE item_instance_id=r.source_item_instance_id;
   WHEN 'container' THEN DELETE FROM game_item_container_entries WHERE item_instance_id=r.source_item_instance_id;
   WHEN 'nested' THEN DELETE FROM game_item_nested_entries WHERE item_instance_id=r.source_item_instance_id;
   ELSE RAISE EXCEPTION 'House decoration cannot retire this custody' USING ERRCODE='23514';
  END CASE;
 END IF;
 c:=r.effects->'decoration_cooldown';
 IF c IS NOT NULL AND c<>'null'::jsonb THEN
  IF c->'before'='null'::jsonb THEN
   INSERT INTO game_house_decoration_cooldowns VALUES(r.character_id,c->>'key',(c->>'until')::bigint,r.transaction_id);
  ELSE
   UPDATE game_house_decoration_cooldowns SET until_ms=(c->>'until')::bigint,transaction_id=r.transaction_id
    WHERE character_id=r.character_id AND cooldown_key=c->>'key' AND until_ms=(c->>'before')::bigint;
   IF NOT FOUND THEN RAISE EXCEPTION 'House decoration cooldown predecessor disappeared' USING ERRCODE='23514'; END IF;
  END IF;
 END IF;
END $$;

DO $migration$
DECLARE body TEXT; n TEXT; arm TEXT; begin_at INTEGER; definition TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO definition FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF definition IS NULL OR left(definition,6)<>'CHECK ' THEN RAISE EXCEPTION 'missing item-action custody shape'; END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
  ||substring(definition FROM 7)||' OR (source_kind=5 AND parent_item_instance_id IS NULL AND outputs=''[]''::jsonb'
  ||' AND extra_inputs=''[]''::jsonb AND secondary_item_instance_id=source_item_instance_id'
  ||' AND effects->>''kind''=''house_decoration'' AND source_count IN(0,1) AND consume_source=(source_count=1)))';
 FOREACH n IN ARRAY ARRAY['game_item_action_house_admit','game_item_action_house_complete','game_item_action_house_apply',
  'game_item_instance_guard','game_item_instance_change_proven','game_house_location_proven',
  'game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_action_house_admit' THEN '(game_item_consume_transform_records)'
   WHEN n IN('game_item_action_house_complete','game_item_action_house_apply') THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing decoration extension boundary: %',n; END IF;
  IF n='game_item_action_house_admit' THEN arm:=$arm$
   IF r.effects->>'kind'='house_decoration' THEN PERFORM game_house_decoration_admit(r); RETURN; END IF;
$arm$;
  ELSIF n='game_item_action_house_complete' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=p_transaction AND effects->>'kind'='house_decoration') THEN
    PERFORM game_house_decoration_complete(p_transaction); RETURN; END IF;
$arm$;
  ELSIF n='game_item_action_house_apply' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records WHERE transaction_id=p_transaction AND effects->>'kind'='house_decoration') THEN
    PERFORM game_house_decoration_apply(p_transaction); RETURN; END IF;
$arm$;
  ELSIF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE r.source_kind=5
    AND r.effects->>'kind'='house_decoration' AND r.created_xact_id=pg_current_xact_id()
    AND r.source_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
    AND (r.effects->>'house_mutate')::boolean AND NEW.last_transaction_id=r.transaction_id
    AND to_jsonb(NEW)=r.before_item||jsonb_build_object('definition_production_key',r.effects->'house_after'->>'key',
     'quantity',CASE WHEN r.consume_source THEN 0 ELSE OLD.quantity END,'lifecycle',CASE WHEN r.consume_source THEN 2 ELSE OLD.lifecycle END,
     'last_transaction_id',r.transaction_id,'state_revision',OLD.state_revision+1)) THEN RETURN NEW; END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE r.source_kind=5 AND r.effects->>'kind'='house_decoration'
    AND r.created_xact_id=pg_current_xact_id() AND r.source_item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id) THEN
    PERFORM game_house_decoration_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF n IN('game_item_container_entry_removal_proven','game_item_nested_entry_proven') THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r JOIN game_item_instances i ON i.item_instance_id=r.source_item_instance_id
    WHERE r.source_kind=5 AND r.effects->>'kind'='house_decoration' AND r.consume_source
     AND r.created_xact_id=pg_current_xact_id() AND r.source_location->'custody'=to_jsonb(OLD)
     AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id) THEN RETURN NULL; END IF;
$arm$;
  ELSE arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r JOIN game_item_instances i ON i.item_instance_id=r.source_item_instance_id
    WHERE r.source_kind=5 AND r.effects->>'kind'='house_decoration' AND r.consume_source
     AND r.created_xact_id=pg_current_xact_id() AND r.source_location->'house'=to_jsonb(OLD)
     AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_house_decoration_pair(TEXT,TEXT)','game_house_decoration_cooldown_guard()',
  'game_house_decoration_admit(game_item_consume_transform_records)','game_house_decoration_complete(UUID)','game_house_decoration_apply(UUID)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION game_house_decoration_pair(TEXT,TEXT),game_house_decoration_complete(UUID),game_house_decoration_apply(UUID) TO oteryn_game_runtime;
