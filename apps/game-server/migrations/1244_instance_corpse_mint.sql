-- Extend the existing loot MINT owner to tagged physical Instance deaths.
-- Existing Channel causes, semantic bindings, audit bytes and receipts remain valid.
DO $migration$
DECLARE t TEXT;
BEGIN
 -- pc7 already installed the exact typed cause in 1100/1101 and the
 -- assignment-key projection in 1152. Keep those stricter proof chains;
 -- do not install a second representation or duplicate their columns.
 IF EXISTS(SELECT 1 FROM pg_attribute
   WHERE attrelid='game_item_mint_receipts'::regclass
    AND attname='death_scope_id' AND NOT attisdropped) THEN
  IF NOT EXISTS(SELECT 1 FROM pg_attribute
    WHERE attrelid='game_item_mint_receipts'::regclass
     AND attname='death_scope_key' AND NOT attisdropped) THEN
   RAISE EXCEPTION 'existing typed native MINT is missing its assignment-key projection';
  END IF;
  RETURN;
 END IF;
 FOREACH t IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
  EXECUTE format('ALTER TABLE %1$I DROP CONSTRAINT %1$s_pkey,
   ALTER COLUMN death_channel_id DROP NOT NULL,
   DROP CONSTRAINT %1$s_death_channel_id_check,
   ADD COLUMN death_scope_kind SMALLINT NOT NULL DEFAULT 1,
   ADD COLUMN death_instance_id UUID,
   ADD CONSTRAINT %1$s_typed_death_scope CHECK(
    (death_scope_kind=1 AND death_channel_id IS NOT NULL AND game_character_is_uuid_v7(death_channel_id) AND death_instance_id IS NULL)
    OR (death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id IS NOT NULL AND game_character_is_uuid_v7(death_instance_id))),
   ADD COLUMN death_runtime_scope_key BYTEA GENERATED ALWAYS AS (
    (CASE WHEN death_scope_kind=1 THEN decode(''01'',''hex'') ELSE decode(''03'',''hex'') END)
    || uuid_send(death_world_id) || uuid_send(COALESCE(death_channel_id,death_instance_id))) STORED,
   ADD PRIMARY KEY(death_runtime_scope_key,death_scope_ownership_generation,
    death_actor_local_id,death_actor_local_generation,loot_table_family,
    loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal)',t);
 END LOOP;
END $migration$;

-- Extend installed proof chains rather than copying historical definitions and
-- losing subsequently admitted item owners. Tagged keys are nonnullable and
-- include World and scope namespace as well as the local UUID.
DO $migration$
DECLARE n TEXT; body TEXT; amended TEXT;
BEGIN
 -- pc7 already installed the exact typed cause in 1100/1101 and the
 -- assignment-key projection in 1152. Keep those stricter proof chains;
 -- do not install a second representation or duplicate their columns.
 IF EXISTS(SELECT 1 FROM pg_attribute
   WHERE attrelid='game_item_mint_receipts'::regclass
    AND attname='death_scope_id' AND NOT attisdropped) THEN
  IF NOT EXISTS(SELECT 1 FROM pg_attribute
    WHERE attrelid='game_item_mint_receipts'::regclass
     AND attname='death_scope_key' AND NOT attisdropped) THEN
   RAISE EXCEPTION 'existing typed native MINT is missing its assignment-key projection';
  END IF;
  RETURN;
 END IF;
 -- Stored generated columns are not populated in NEW during a BEFORE
 -- trigger. Compare every writable source field as before, excluding only
 -- the database-derived key that callers cannot assign or mutate.
 FOREACH n IN ARRAY ARRAY['game_item_mint_reservation_guard()',
   'game_item_mint_receipt_guard()'] LOOP
  body:=pg_get_functiondef(n::regprocedure);
  amended:=replace(body,'to_jsonb(NEW)',
   '(to_jsonb(NEW) - ''death_runtime_scope_key'')');
  amended:=replace(amended,'to_jsonb(OLD)',
   '(to_jsonb(OLD) - ''death_runtime_scope_key'')');
  IF amended=body THEN RAISE EXCEPTION 'missing immutable loot MINT source comparison in %',n; END IF;
  EXECUTE amended;
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_item_mint_consistency_guard()',
   'game_kill_nested_loot_mint_proven(uuid)'] LOOP
  body:=pg_get_functiondef(n::regprocedure);
  amended:=regexp_replace(body,'g\.channel_id\s*=\s*r\.death_channel_id',
   '(g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.death_scope_kind,r.death_channel_id,r.death_instance_id)','g');
  -- The Ground expression above must retain its typed columns. Replace only
  -- remaining full-cause comparisons, which use the nonnullable generated key.
  amended:=replace(amended,'v.death_channel_id','v.death_runtime_scope_key');
  amended:=replace(amended,'r.death_channel_id, r.death_scope_ownership_generation','r.death_runtime_scope_key, r.death_scope_ownership_generation');
  amended:=replace(amended,'cr.death_channel_id = r.death_channel_id','cr.death_runtime_scope_key = r.death_runtime_scope_key');
  amended:=replace(amended,'m.death_channel_id','m.death_runtime_scope_key');
  amended:=replace(amended,'child.death_channel_id','child.death_runtime_scope_key');
  IF amended=body THEN RAISE EXCEPTION 'missing loot MINT scope proof in %',n; END IF;
  EXECUTE amended;
 END LOOP;
 body:=pg_get_functiondef('game_item_mint_receipt_materialize_corpse()'::regprocedure);
 amended:=replace(body,'death_channel_id = NEW.death_channel_id',
  'death_runtime_scope_key = NEW.death_runtime_scope_key');
 IF amended=body THEN RAISE EXCEPTION 'missing corpse materialization cause proof'; END IF;
 EXECUTE amended;

 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 amended:=replace(body,'IF NEW.scope_kind=2 THEN', $arm$
 IF NEW.scope_kind=2 THEN
  IF EXISTS(SELECT 1 FROM game_item_mint_receipts r
    JOIN game_item_instances i USING(item_instance_id)
    WHERE r.item_instance_id=NEW.item_instance_id
     AND r.death_scope_kind=2 AND r.death_world_id=NEW.world_id
     AND r.death_instance_id=NEW.instance_id
     AND r.death_scope_ownership_generation=NEW.runtime_scope_ownership_generation
     AND r.destination_parent_item_instance_id IS NULL
     AND r.transaction_id=i.minted_transaction_id
     AND r.created_xact_id=pg_current_xact_id()) THEN
   -- The existing deferred ItemInstance consistency guard also proves the
   -- complete reservation, exact event bytes and transaction membership.
   RETURN NULL;
  END IF;
 $arm$);
 IF amended=body THEN RAISE EXCEPTION 'missing Instance Ground placement proof chain'; END IF;
 EXECUTE amended;
END $migration$;
