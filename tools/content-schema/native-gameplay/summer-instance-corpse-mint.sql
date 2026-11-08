-- W16: existing ordinary creature MINT owner, exact tagged Summer Instance death.
-- Channel histories retain their full cause tuple and original event bytes.
CREATE FUNCTION game_summer_death_scope_key(p_world UUID,p_channel UUID,p_instance UUID)
RETURNS BYTEA LANGUAGE sql IMMUTABLE AS $$
 SELECT CASE WHEN p_channel IS NOT NULL AND p_instance IS NULL THEN '\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel)
             WHEN p_channel IS NULL AND p_instance IS NOT NULL THEN '\x03'::bytea||uuid_send(p_world)||uuid_send(p_instance) END
$$;
DO $scope_columns$
DECLARE relation TEXT;
BEGIN
 FOREACH relation IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
  EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',relation,relation||'_pkey');
  EXECUTE format('ALTER TABLE %I ALTER COLUMN death_channel_id DROP NOT NULL',relation);
  EXECUTE format('ALTER TABLE %I ADD COLUMN death_instance_id UUID CHECK(death_instance_id IS NULL OR game_character_is_uuid_v7(death_instance_id))',relation);
  EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK((death_channel_id IS NOT NULL AND death_instance_id IS NULL) OR (death_channel_id IS NULL AND death_instance_id IS NOT NULL))',relation,relation||'_death_scope_shape');
  EXECUTE format('ALTER TABLE %I ADD COLUMN death_scope_key BYTEA GENERATED ALWAYS AS(game_summer_death_scope_key(death_world_id,death_channel_id,death_instance_id)) STORED',relation);
  EXECUTE format('ALTER TABLE %I ADD PRIMARY KEY(death_world_id,death_scope_key,death_scope_ownership_generation,death_actor_local_id,death_actor_local_generation,loot_table_family,loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal)',relation);
  EXECUTE format($constraint$ALTER TABLE %I ADD CONSTRAINT %I CHECK(death_instance_id IS NULL OR
   (loot_table_revision_ref='definition-r1' AND
    ((loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0 AND loot_table_family='Item' AND loot_table_production_key IN
      ('oteryn:item.tibia.i43758','oteryn:item.tibia.i54346','oteryn:item.tibia.i54352','oteryn:item.tibia.i54433','oteryn:item.tibia.i54670'))
     OR (loot_purpose_key<>'CORPSE_MATERIALIZATION' AND loot_table_family='Loot' AND loot_table_production_key IN
      ('oteryn:loot.creature.rakesh_moonfang','oteryn:loot.creature.maior_domus','oteryn:loot.creature.maior_domus_final','oteryn:loot.creature.mimar_haffar','oteryn:loot.creature.mimar_haffar_final','oteryn:loot.creature.phosphorus','oteryn:loot.creature.phosphorus_final','oteryn:loot.creature.the_moonsnow_magnolia')))))$constraint$,relation,relation||'_summer_instance_source');
 END LOOP;
END $scope_columns$;

-- Stored generated values are computed after BEFORE triggers. Compare every
-- immutable input, while omitting only this DB-derived scope key (NEW is NULL
-- here). World/Channel/Instance and every other full-cause field remain fixed.
CREATE OR REPLACE FUNCTION game_item_mint_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.work_units_used=OLD.work_units_used+1
 AND (to_jsonb(NEW)-'work_units_used'-'death_scope_key')
     =(to_jsonb(OLD)-'work_units_used'-'death_scope_key') THEN
  RETURN NEW;
 END IF;
 RAISE EXCEPTION 'DUR-03 item MINT reservation is immutable except its work-unit charge'
  USING ERRCODE='23514';
END $$;
DO $mint_guard_path$
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_mint_reservation_guard() SET search_path=%I,pg_temp',current_schema());
END $mint_guard_path$;

-- The deferred materializer's one-way mark has the same generated-key
-- ordering as reservation charging. Only the derived key is omitted; every
-- writable receipt/cause/scope/physical body field remains immutable.
CREATE OR REPLACE FUNCTION game_item_mint_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.loot_purpose_key='CORPSE_MATERIALIZATION'
 AND OLD.materialized_at IS NULL AND NEW.materialized_at IS NOT NULL
 AND (to_jsonb(NEW)-'materialized_at'-'death_scope_key')
     =(to_jsonb(OLD)-'materialized_at'-'death_scope_key') THEN
  RETURN NEW;
 END IF;
 RAISE EXCEPTION 'DUR-03 item MINT receipt is immutable except its one-way materialized_at mark'
  USING ERRCODE='23514';
END $$;
DO $mint_receipt_guard_path$
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_mint_receipt_guard() SET search_path=%I,pg_temp',current_schema());
END $mint_receipt_guard_path$;


-- This context is neither client input nor a writable GUC. Only possession of
-- the actual registered Node secret can arm its current Instance assignment.
CREATE TABLE game_summer_instance_mint_context(
 backend_pid INTEGER NOT NULL,transaction_xid XID8 NOT NULL,scope_key BYTEA NOT NULL CHECK(octet_length(scope_key)=33),
 generation NUMERIC NOT NULL,node_id UUID NOT NULL,registration NUMERIC NOT NULL,
 PRIMARY KEY(backend_pid,transaction_xid));
REVOKE ALL ON game_summer_instance_mint_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
-- Current mature activity identity, not an arbitrary Node-owned Instance.
CREATE FUNCTION game_summer_instance_activity_source(p_scope BYTEA,p_generation NUMERIC,p_family TEXT,p_key TEXT,p_purpose TEXT)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g
 JOIN game_boss_room_activities a ON a.admission=g.handoff_id AND a.world_id=g.world_id AND a.instance_id=g.instance_id
 JOIN (VALUES
 ('oteryn:creature.rakesh_moonfang','oteryn:item.tibia.i43758','oteryn:loot.creature.rakesh_moonfang'),
 ('oteryn:creature.maior_domus','oteryn:item.tibia.i54433','oteryn:loot.creature.maior_domus'),
 ('oteryn:creature.maior_domus','oteryn:item.tibia.i54433','oteryn:loot.creature.maior_domus_final'),
 ('oteryn:creature.maior_domus_final','oteryn:item.tibia.i54433','oteryn:loot.creature.maior_domus_final'),
 ('oteryn:creature.mimar_haffar','oteryn:item.tibia.i54346','oteryn:loot.creature.mimar_haffar'),
 ('oteryn:creature.mimar_haffar','oteryn:item.tibia.i54346','oteryn:loot.creature.mimar_haffar_final'),
 ('oteryn:creature.mimar_haffar_final','oteryn:item.tibia.i54346','oteryn:loot.creature.mimar_haffar_final'),
 ('oteryn:creature.phosphorus','oteryn:item.tibia.i54352','oteryn:loot.creature.phosphorus'),
 ('oteryn:creature.phosphorus','oteryn:item.tibia.i54352','oteryn:loot.creature.phosphorus_final'),
 ('oteryn:creature.phosphorus_final','oteryn:item.tibia.i54352','oteryn:loot.creature.phosphorus_final'),
 ('oteryn:creature.the_moonsnow_magnolia','oteryn:item.tibia.i54670','oteryn:loot.creature.the_moonsnow_magnolia')
 ) source(creature,body,loot) ON source.creature=a.boss_creature_key
 WHERE ('\x03'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id))=p_scope
 AND g.ownership_generation=p_generation AND g.state=2 AND a.state IN(1,2,3)
 AND a.boss_key=g.activity_key AND a.content_revision=g.content_revision
 AND ((p_purpose='CORPSE_MATERIALIZATION' AND p_family='Item' AND p_key=source.body)
   OR(p_purpose<>'CORPSE_MATERIALIZATION' AND p_family='Loot' AND p_key=source.loot)))
$$;
REVOKE ALL ON FUNCTION game_summer_instance_activity_source(bytea,numeric,text,text,text) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_summer_instance_mint_arm(p_node UUID,p_registration NUMERIC,p_secret BYTEA,p_scope BYTEA,p_generation NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
 OR octet_length(p_scope) IS DISTINCT FROM 33 OR get_byte(p_scope,0) IS DISTINCT FROM 3
 OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.scope_key=p_scope AND a.scope_kind=3 AND a.state=1
  AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration) THEN
  RAISE EXCEPTION 'Summer Instance MINT requires its actual current Node/scope generation' USING ERRCODE='23514';
 END IF;
 PERFORM 1 FROM game_activity_handoff_groups g JOIN game_boss_room_activities a ON a.admission=g.handoff_id
 WHERE ('\x03'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id))=p_scope
 AND g.ownership_generation=p_generation AND g.state=2 AND a.state IN(1,2,3)
 AND a.world_id=g.world_id AND a.instance_id=g.instance_id AND a.boss_key=g.activity_key AND a.content_revision=g.content_revision
 AND a.boss_creature_key IN('oteryn:creature.rakesh_moonfang','oteryn:creature.maior_domus','oteryn:creature.maior_domus_final',
 'oteryn:creature.mimar_haffar','oteryn:creature.mimar_haffar_final','oteryn:creature.phosphorus','oteryn:creature.phosphorus_final','oteryn:creature.the_moonsnow_magnolia') FOR SHARE OF g,a;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer MINT requires its actual qualified encounter activity' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_summer_instance_mint_context c WHERE c.backend_pid=pg_backend_pid()AND c.transaction_xid=pg_current_xact_id()
  AND (c.scope_key,c.generation,c.node_id,c.registration) IS DISTINCT FROM(p_scope,p_generation,p_node,p_registration))THEN
  RAISE EXCEPTION 'Summer MINT context cannot switch Instance or owner' USING ERRCODE='23514';
 END IF;
 DELETE FROM game_summer_instance_mint_context c WHERE (c.backend_pid=pg_backend_pid()AND c.transaction_xid<>pg_current_xact_id())OR NOT EXISTS(SELECT 1 FROM pg_stat_activity s WHERE s.pid=c.backend_pid);
 INSERT INTO game_summer_instance_mint_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_scope,p_generation,p_node,p_registration)ON CONFLICT DO NOTHING;
END $$;
CREATE FUNCTION game_summer_instance_mint_admit()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.death_instance_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_summer_instance_mint_context c JOIN game_runtime_scope_assignments a ON a.scope_key=c.scope_key
  JOIN game_node_registrations n ON n.node_id=c.node_id AND n.registration_revision=c.registration
  WHERE c.backend_pid=pg_backend_pid()AND c.transaction_xid=pg_current_xact_id()
   AND c.scope_key=game_summer_death_scope_key(NEW.death_world_id,NEW.death_channel_id,NEW.death_instance_id)
   AND c.generation=NEW.death_scope_ownership_generation AND a.scope_kind=3 AND a.state=1
   AND a.ownership_generation=c.generation AND a.holder_node_id=c.node_id AND a.holder_registration_revision=c.registration
   AND n.state=1 AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings e WHERE e.node_id=c.node_id))THEN
  RAISE EXCEPTION 'Summer Instance MINT lacks same-transaction private owner proof' USING ERRCODE='23514';
 END IF;
 IF NEW.death_instance_id IS NOT NULL AND game_summer_instance_activity_source(game_summer_death_scope_key(NEW.death_world_id,NEW.death_channel_id,NEW.death_instance_id),NEW.death_scope_ownership_generation,
 NEW.loot_table_family,NEW.loot_table_production_key,NEW.loot_purpose_key) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'Summer MINT body/loot must match its actual encounter identity' USING ERRCODE='23514';END IF;
 IF TG_TABLE_NAME='game_item_mint_receipts' AND NEW.death_instance_id IS NOT NULL
 AND NEW.loot_purpose_key='CORPSE_MATERIALIZATION' AND NOT EXISTS(SELECT 1 FROM game_item_instances i
  WHERE i.item_instance_id=NEW.item_instance_id AND i.world_id=NEW.death_world_id AND i.quantity=1 AND i.lifecycle=1
   AND i.definition_family=NEW.loot_table_family AND i.definition_production_key=NEW.loot_table_production_key AND i.definition_revision_ref=NEW.loot_table_revision_ref)THEN
  RAISE EXCEPTION 'Summer Instance corpse must retain the exact native physical body' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_summer_instance_reservation_admit BEFORE INSERT ON game_item_mint_reservations FOR EACH ROW EXECUTE FUNCTION game_summer_instance_mint_admit();
CREATE TRIGGER game_summer_instance_receipt_admit BEFORE INSERT ON game_item_mint_receipts FOR EACH ROW EXECUTE FUNCTION game_summer_instance_mint_admit();

-- The existing Instance gate predates qualified Summer corpse MINT. Admit
-- that exact same-XID cause through the gate, then continue through every
-- original mature MINT/exclusivity check below it. Never RETURN early here.
DO $summer_ground_cause$
DECLARE body TEXT;needle TEXT:='IF NEW.scope_kind=2 THEN';replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'Summer Ground exact Instance gate changed';
 END IF;
 replacement:=$accepted$
IF NEW.scope_kind=2 AND NOT EXISTS(
 SELECT 1 FROM game_item_mint_receipts r
 JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
 JOIN game_summer_instance_mint_context c ON c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 JOIN game_runtime_scope_assignments ownership ON ownership.scope_key=c.scope_key
 JOIN game_node_registrations n ON n.node_id=c.node_id AND n.registration_revision=c.registration
 WHERE r.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
 AND r.death_world_id=NEW.world_id AND r.death_channel_id IS NULL AND r.death_instance_id=NEW.instance_id
 AND r.death_scope_ownership_generation=NEW.runtime_scope_ownership_generation
 AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.loot_table_family='Item'
 AND i.world_id=NEW.world_id AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1
 AND i.last_transaction_id IS NULL AND i.minted_transaction_id=r.transaction_id
 AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
     =(r.loot_table_family,r.loot_table_production_key,r.loot_table_revision_ref)
 AND c.scope_key=game_summer_death_scope_key(NEW.world_id,NULL,NEW.instance_id)
 AND c.generation=NEW.runtime_scope_ownership_generation
 AND ownership.scope_kind=3 AND ownership.state=1 AND ownership.ownership_generation=c.generation
 AND ownership.holder_node_id=c.node_id AND ownership.holder_registration_revision=c.registration
 AND n.state=1 AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings ending WHERE ending.node_id=c.node_id)
 AND game_summer_instance_activity_source(c.scope_key,c.generation,r.loot_table_family,r.loot_table_production_key,r.loot_purpose_key)
) THEN
$accepted$;
 EXECUTE replace(body,needle,replacement);
END $summer_ground_cause$;

-- Extend only native Source MINT INSERT locking. This is not a mutation
-- authorization: every existing deferred child-MINT proof still runs. Other
-- Instance operations (including future TAKE/DELETE) retain their old refusal.
DO $summer_corpse_entry_lock$
DECLARE body TEXT;needle TEXT;replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_corpse_entry_owner_lock()'::regprocedure);
 needle:='DECLARE p UUID; w UUID; c UUID;';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'Summer corpse lock declaration changed';END IF;
 body:=replace(body,needle,'DECLARE p UUID; w UUID; c UUID; source_instance UUID; source_root UUID; source_kind SMALLINT; source_generation NUMERIC;');
 needle:='SELECT g.world_id,g.channel_id INTO w,c FROM ancestors a JOIN game_item_ground_locations g ON g.item_instance_id=a.id;';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'Summer corpse lock physical root lookup changed';END IF;
 body:=replace(body,needle,'SELECT g.world_id,g.channel_id,g.instance_id,g.item_instance_id,g.scope_kind,g.runtime_scope_ownership_generation INTO w,c,source_instance,source_root,source_kind,source_generation FROM ancestors a JOIN game_item_ground_locations g ON g.item_instance_id=a.id;');
 needle:='IF w IS NULL OR c IS NULL THEN';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'Summer corpse lock Channel boundary changed';END IF;
 replacement:=$source_lock$
IF TG_OP='INSERT' AND source_kind=2 AND w IS NOT NULL AND c IS NULL AND source_instance IS NOT NULL
 AND EXISTS(
 SELECT 1 FROM game_summer_instance_mint_context context
 JOIN game_runtime_scope_assignments ownership ON ownership.scope_key=context.scope_key
 JOIN game_node_registrations node ON node.node_id=context.node_id AND node.registration_revision=context.registration
 JOIN game_item_mint_receipts receipt ON receipt.item_instance_id=source_root
 JOIN game_item_instances item ON item.item_instance_id=receipt.item_instance_id
 WHERE context.backend_pid=pg_backend_pid() AND context.transaction_xid=pg_current_xact_id()
 AND context.scope_key=game_summer_death_scope_key(w,NULL,source_instance) AND context.generation=source_generation
 AND ownership.scope_kind=3 AND ownership.state=1 AND ownership.ownership_generation=context.generation
 AND ownership.holder_node_id=context.node_id AND ownership.holder_registration_revision=context.registration
 AND node.state=1 AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings ending WHERE ending.node_id=node.node_id)
 AND receipt.death_world_id=w AND receipt.death_channel_id IS NULL AND receipt.death_instance_id=source_instance
 AND receipt.death_scope_ownership_generation=source_generation
 AND receipt.created_xact_id=pg_current_xact_id()
 AND receipt.loot_purpose_key='CORPSE_MATERIALIZATION' AND receipt.loot_table_family='Item'
 AND item.world_id=w AND item.lifecycle=1 AND item.quantity=1 AND item.state_revision=1
 AND item.last_transaction_id IS NULL AND item.minted_transaction_id=receipt.transaction_id
 AND (item.definition_family,item.definition_production_key,item.definition_revision_ref)
     =(receipt.loot_table_family,receipt.loot_table_production_key,receipt.loot_table_revision_ref)
 AND game_summer_instance_activity_source(context.scope_key,context.generation,receipt.loot_table_family,receipt.loot_table_production_key,receipt.loot_purpose_key)
 ) THEN
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(game_summer_death_scope_key(w,NULL,source_instance),'hex'),33));
 RETURN NEW;
END IF;
IF w IS NULL OR c IS NULL THEN
$source_lock$;
 EXECUTE replace(body,needle,replacement);
END $summer_corpse_entry_lock$;
-- The closed branch reads private proof; its pinned pg_catalog/schema/pg_temp
-- path is retained from0042. Definer privilege grants no mutation authorization.
ALTER FUNCTION game_spell_corpse_entry_owner_lock() SECURITY DEFINER;

-- Extend the existing conservation, materialization and nested-loot functions
-- in place. All unrelated source-specific branches remain byte-for-byte intact.
DO $owner_scope$
DECLARE signature TEXT;body TEXT;previous TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_item_mint_consistency_guard()','game_kill_nested_loot_mint_proven(uuid)','game_item_mint_receipt_materialize_corpse()'] LOOP
  body:=pg_get_functiondef(signature::regprocedure);previous:=body;
  body:=replace(body,'g.channel_id = r.death_channel_id','g.channel_id IS NOT DISTINCT FROM r.death_channel_id AND g.instance_id IS NOT DISTINCT FROM r.death_instance_id');
  body:=replace(body,'v.death_world_id, v.death_channel_id','v.death_world_id, v.death_scope_key');
  body:=replace(body,'r.death_world_id, r.death_channel_id','r.death_world_id, r.death_scope_key');
  body:=replace(body,'m.death_world_id, m.death_channel_id','m.death_world_id, m.death_scope_key');
  body:=replace(body,'child.death_world_id, child.death_channel_id','child.death_world_id, child.death_scope_key');
  body:=replace(body,'cr.death_channel_id = r.death_channel_id','cr.death_scope_key = r.death_scope_key');
  body:=replace(body,'death_channel_id = NEW.death_channel_id','death_scope_key = NEW.death_scope_key');
  IF body=previous THEN RAISE EXCEPTION 'Summer MINT existing owner scope boundary changed: %',signature;END IF;
  EXECUTE body;
 END LOOP;
END $owner_scope$;
DO $source_privileges$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_summer_instance_activity_source(bytea,numeric,text,text,text)','game_summer_instance_mint_arm(uuid,numeric,bytea,bytea,numeric)','game_summer_instance_mint_admit()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  IF signature<>'game_summer_instance_activity_source(bytea,numeric,text,text,text)' THEN EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);END IF;
 END LOOP;
END $source_privileges$;
