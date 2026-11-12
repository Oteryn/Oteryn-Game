-- ENCX: Crystal 201189f, data-global/monster/quests/primal_ordeal_quest/
-- the_primal_menace.lua:79,94 has a physical corpse and personal boss rewards.
-- Keep the native death's Instance namespace through its ordinary corpse/loot
-- descendants. Runtime kind 2 is Instance; its assignment key uses tag 3
-- (0247), while legacy Channel assignment bytes keep tag 1.
CREATE FUNCTION game_item_death_scope_key(
    p_world UUID, p_kind SMALLINT, p_channel UUID, p_instance UUID
) RETURNS BYTEA LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT CASE
      WHEN p_kind=1 AND p_channel IS NOT NULL AND p_instance IS NULL
        THEN '\x01'::BYTEA || uuid_send(p_world) || uuid_send(p_channel)
      WHEN p_kind=2 AND p_channel IS NULL AND p_instance IS NOT NULL
        THEN game_activity_scope_key(p_world,p_instance)
      ELSE NULL END
$$;
REVOKE ALL ON FUNCTION game_item_death_scope_key(UUID,SMALLINT,UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_death_scope_key(UUID,SMALLINT,UUID,UUID)
    TO oteryn_game_runtime, oteryn_game_control;

CREATE FUNCTION game_item_mint_death_scope_capture() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE expected BYTEA;
BEGIN
  --1100 writers omit this additive compatibility projection. An explicitly
  -- supplied kind is still validated against exact nullable source UUIDs.
  IF NEW.death_scope_kind IS NULL THEN
    NEW.death_scope_kind:=CASE WHEN NEW.death_channel_id IS NOT NULL THEN 1 ELSE 2 END;
  END IF;
  expected:=game_item_death_scope_key(NEW.death_world_id,NEW.death_scope_kind,
    NEW.death_channel_id,NEW.death_instance_id);
  IF expected IS NULL OR (NEW.death_scope_key IS NOT NULL
      AND NEW.death_scope_key IS DISTINCT FROM expected) THEN
    RAISE EXCEPTION 'native death requires one exact Channel or Instance scope'
      USING ERRCODE='23514';
  END IF;
  NEW.death_scope_key:=expected;
  RETURN NEW;
END
$$;
REVOKE ALL ON FUNCTION game_item_mint_death_scope_capture() FROM PUBLIC;

DO $migration$
DECLARE t TEXT;
BEGIN
  FOREACH t IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
    IF EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid=t::regclass
        AND attname='death_scope_id' AND NOT attisdropped) THEN
    --SRVENC1100 is already the complete exact typed cause owner. Preserve its
    --17-byte scope-id PK and every installed proof. Add only equivalent
    --33-byte assignment-scope projections for ENCX readers/components.
    EXECUTE format('ALTER TABLE %I ADD COLUMN death_scope_kind SMALLINT
      GENERATED ALWAYS AS (CASE WHEN death_channel_id IS NOT NULL THEN 1 ELSE 2 END) STORED NOT NULL',t);
    EXECUTE format('ALTER TABLE %I ALTER COLUMN death_scope_kind DROP EXPRESSION',t);
    EXECUTE format('ALTER TABLE %I ADD COLUMN death_scope_key BYTEA
      GENERATED ALWAYS AS (game_item_death_scope_key(death_world_id,death_scope_kind,
        death_channel_id,death_instance_id)) STORED NOT NULL',t);
    EXECUTE format('ALTER TABLE %I ALTER COLUMN death_scope_key DROP EXPRESSION',t);
    EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (
      (death_scope_kind=1 AND death_channel_id IS NOT NULL AND death_instance_id IS NULL)
      OR (death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id IS NOT NULL))',t,t||'_death_scope');
    EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK
      (death_scope_key=game_item_death_scope_key(death_world_id,death_scope_kind,
       death_channel_id,death_instance_id))',t,t||'_scope_key_exact');
    EXECUTE format('CREATE TRIGGER game_item_mint_death_scope_capture BEFORE INSERT ON %I
      FOR EACH ROW EXECUTE FUNCTION game_item_mint_death_scope_capture()',t);
    EXECUTE format('CREATE UNIQUE INDEX %I ON %I
      (death_world_id,death_scope_key,death_scope_ownership_generation,
       death_actor_local_id,death_actor_local_generation,loot_table_family,
       loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal)',t||'_scope_key_cause',t);
    ELSE
    EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',t,t||'_pkey');
    EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',t,t||'_death_channel_id_check');
    EXECUTE format('ALTER TABLE %I ALTER COLUMN death_channel_id DROP NOT NULL',t);
    EXECUTE format('ALTER TABLE %I ADD COLUMN death_scope_kind SMALLINT NOT NULL DEFAULT 1,
      ADD COLUMN death_instance_id UUID,
      ADD CONSTRAINT %I CHECK (
        (death_scope_kind=1 AND death_channel_id IS NOT NULL
          AND game_character_is_uuid_v7(death_channel_id) AND death_instance_id IS NULL)
        OR (death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id IS NOT NULL
          AND game_character_is_uuid_v7(death_instance_id)))',t,t||'_death_scope');
    EXECUTE format('ALTER TABLE %I ADD COLUMN death_scope_key BYTEA
      GENERATED ALWAYS AS (game_item_death_scope_key(death_world_id,death_scope_kind,
        death_channel_id,death_instance_id)) STORED NOT NULL',t);
    -- DDL backfills existing immutable rows without an UPDATE. An ordinary
    -- stored key then remains visible in BEFORE UPDATE, so the installed exact
    -- row guards need no exception for generated-column evaluation order.
    EXECUTE format('ALTER TABLE %I ALTER COLUMN death_scope_key DROP EXPRESSION',t);
    EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK
      (death_scope_key=game_item_death_scope_key(death_world_id,death_scope_kind,
       death_channel_id,death_instance_id))',t,t||'_scope_key_exact');
    EXECUTE format('CREATE TRIGGER game_item_mint_death_scope_capture BEFORE INSERT ON %I
      FOR EACH ROW EXECUTE FUNCTION game_item_mint_death_scope_capture()',t);
    EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I PRIMARY KEY
      (death_world_id,death_scope_key,death_scope_ownership_generation,
       death_actor_local_id,death_actor_local_generation,loot_table_family,
       loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal)',t,t||'_pkey');
    END IF;
    --A missing discriminator is derived only in the INSERT trigger; DEFAULT1
    --would misclassify the old1100 writer's legitimate Instance insertion.
    EXECUTE format('ALTER TABLE %I ALTER COLUMN death_scope_kind DROP DEFAULT',t);
  END LOOP;
END
$migration$;

-- Extend the installed native MINT arms, retaining every later accepted owner
-- and all immutable receipt, audit, location, generation and capacity checks.
-- Stage-timer admission remains its separate owner; this migration changes only
-- the native MINT provenance comparisons and its corpse materialization stamp.
DO $migration$
DECLARE signature TEXT; f REGPROCEDURE; body TEXT; original TEXT;
BEGIN
  FOREACH signature IN ARRAY ARRAY[
    'game_item_mint_receipt_materialize_corpse()',
    'game_item_mint_consistency_guard()',
    'game_corpse_source_race_admit()',
    'game_kill_nested_loot_mint_proven(uuid)'
  ] LOOP
    f:=to_regprocedure(signature);
    IF f IS NULL THEN RAISE EXCEPTION 'native MINT proof missing: %',signature; END IF;
    IF EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='game_item_mint_receipts'::regclass
        AND attname='death_scope_id' AND NOT attisdropped) THEN
      --1100 owns full cause equality/materialization and exact source race.
      --No text replacement may reinterpret its already typed Ground arms.
      CONTINUE;
    END IF;
    body:=pg_get_functiondef(f); original:=body;
    body:=replace(body,'g.channel_id = r.death_channel_id',
      'game_item_death_scope_key(g.world_id,g.scope_kind,g.channel_id,g.instance_id) = r.death_scope_key');
    body:=replace(body,'g.channel_id=r.death_channel_id',
      'game_item_death_scope_key(g.world_id,g.scope_kind,g.channel_id,g.instance_id)=r.death_scope_key');
    body:=replace(body,'death_channel_id','death_scope_key');
    IF body=original THEN
      RAISE EXCEPTION 'native MINT provenance changed in %; review Instance extension',signature;
    END IF;
    EXECUTE body;
  END LOOP;
END
$migration$;

-- W02's Instance Ground gate intentionally admitted only its existing player
-- death primitive. Add the native corpse cause with the same complete frozen
-- reservation, immutable death tuple, current assignment and atomic audit proof.
CREATE FUNCTION game_item_native_instance_corpse_proven(p_item UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_item_mint_receipts r
  JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  JOIN game_item_mint_reservations v ON
   (v.death_world_id,v.death_scope_key,v.death_scope_ownership_generation,
    v.death_actor_local_id,v.death_actor_local_generation,v.loot_table_family,
    v.loot_table_production_key,v.loot_table_revision_ref,v.loot_purpose_key,v.draw_ordinal,
    v.intent_binding,v.transaction_id,v.event_id,v.item_instance_id,v.occurred_at)
   =(r.death_world_id,r.death_scope_key,r.death_scope_ownership_generation,
    r.death_actor_local_id,r.death_actor_local_generation,r.loot_table_family,
    r.loot_table_production_key,r.loot_table_revision_ref,r.loot_purpose_key,r.draw_ordinal,
    r.intent_binding,r.transaction_id,r.event_id,r.item_instance_id,r.occurred_at)
  JOIN game_item_audit_outbox a ON a.event_id=r.event_id
  JOIN game_runtime_scope_assignments owner ON owner.scope_key=r.death_scope_key
  WHERE r.item_instance_id=p_item AND r.death_scope_kind=2
   AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
   AND r.destination_parent_item_instance_id IS NULL AND r.created_xact_id=pg_current_xact_id()
   AND i.world_id=r.death_world_id AND i.minted_transaction_id=r.transaction_id
   AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
      =(r.loot_table_family,r.loot_table_production_key,r.loot_table_revision_ref)
   AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=r.death_instance_id
   AND g.world_id=r.death_world_id
   AND g.runtime_scope_ownership_generation=r.death_scope_ownership_generation
   AND a.transaction_id=r.transaction_id AND a.item_instance_id=i.item_instance_id
   AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
   AND a.envelope=v.envelope AND sha256(v.envelope)=r.envelope_sha256
   AND a.created_xact_id=pg_current_xact_id()
   AND owner.scope_kind=3 AND owner.state=1
   AND owner.world_id=r.death_world_id AND owner.activity_instance_id=r.death_instance_id
   AND owner.ownership_generation=r.death_scope_ownership_generation
   AND owner.holder_node_id=v.fence_holder_node_id
   AND owner.holder_registration_revision=v.fence_holder_registration_revision
 )
$$;
REVOKE ALL ON FUNCTION game_item_native_instance_corpse_proven(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_native_instance_corpse_proven(UUID) TO oteryn_game_runtime;
DO $migration$
DECLARE body TEXT; gate TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_native_instance_corpse_proven(UUID) SET search_path = %I, pg_temp',current_schema());
 IF EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='game_item_mint_receipts'::regclass
     AND attname='death_scope_id' AND NOT attisdropped) THEN
   --1101 is independently qualified and stricter about fresh state/exclusive
   --custody. Do not append a redundant alternative admission arm.
   RETURN;
 END IF;
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 gate:='RAISE EXCEPTION ''Instance Ground requires its exact accepted placement cause''';
 IF position(gate IN body)=0 THEN RAISE EXCEPTION 'Instance Ground cause guard changed; review native corpse arm'; END IF;
 body:=replace(body,gate,
  'IF game_item_native_instance_corpse_proven(NEW.item_instance_id) THEN RETURN NULL; END IF; '||gate);
 EXECUTE body;
END
$migration$;

-- Preserve the existing cycle/depth and relink checks while taking the same
-- key-33 lock as the actual Ground root's SpellLanePermit in either namespace.
DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 IF EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='game_item_mint_receipts'::regclass
     AND attname='death_scope_id' AND NOT attisdropped) THEN
   body:=pg_get_functiondef('game_spell_corpse_entry_owner_lock()'::regprocedure);
   IF position('decode(''03'',''hex'')||uuid_send(w)||uuid_send(i)' IN body)=0
      OR position('k=2 AND c IS NULL AND i IS NOT NULL' IN body)=0 THEN
     RAISE EXCEPTION '1103 exact typed corpse owner lock changed; review integration';
   END IF;
   RETURN;
 END IF;
 body:=pg_get_functiondef('game_spell_corpse_entry_owner_lock()'::regprocedure);
 amended:=replace(body,'DECLARE p UUID; w UUID; c UUID;',
   'DECLARE p UUID; w UUID; c UUID; k SMALLINT; inst UUID;');
 IF amended=body THEN RAISE EXCEPTION 'corpse entry root declaration changed'; END IF;
 body:=amended;
 amended:=replace(body,'SELECT g.world_id,g.channel_id INTO w,c',
   'SELECT g.world_id,g.channel_id,g.scope_kind,g.instance_id INTO w,c,k,inst');
 IF amended=body THEN RAISE EXCEPTION 'corpse entry physical root selection changed'; END IF;
 body:=amended;
 amended:=replace(body,'IF w IS NULL OR c IS NULL THEN',
   'IF game_item_death_scope_key(w,k,c,inst) IS NULL THEN');
 IF amended=body THEN RAISE EXCEPTION 'corpse entry physical root refusal changed'; END IF;
 body:=amended;
 amended:=replace(body,
   'encode(uuid_send(w),''hex'')||encode(uuid_send(c),''hex'')',
   '(CASE WHEN k=1 THEN encode(uuid_send(w),''hex'')||encode(uuid_send(c),''hex'') ELSE encode(game_item_death_scope_key(w,k,c,inst),''hex'') END)');
 IF amended=body THEN RAISE EXCEPTION 'corpse entry key-33 lock changed'; END IF;
 EXECUTE amended;
END
$migration$;

--Pure scope projections and the INSERT capture resolve only this owned schema.
DO $scope_helpers_search_path$
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_death_scope_key(UUID,SMALLINT,UUID,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_mint_death_scope_capture() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $scope_helpers_search_path$;
