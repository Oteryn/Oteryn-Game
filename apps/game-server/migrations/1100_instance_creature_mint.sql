-- W02: Crystal src/creatures/creature.cpp:711-721,777-782 creates the
-- physical corpse in the killed creature's actual scope, including an Instance.
-- Extend the existing DUR-03 cause, never reinterpret an Instance as a Channel.
DO $migration$
DECLARE relation_name TEXT;
BEGIN
 FOREACH relation_name IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
  EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',relation_name,relation_name||'_pkey');
  EXECUTE format('ALTER TABLE %I ALTER COLUMN death_channel_id DROP NOT NULL',relation_name);
  EXECUTE format('ALTER TABLE %I ADD COLUMN death_instance_id UUID CHECK (game_character_is_uuid_v7(death_instance_id))',relation_name);
  EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK ((death_channel_id IS NOT NULL) <> (death_instance_id IS NOT NULL))',relation_name,relation_name||'_scope_variant');
  EXECUTE format($ddl$ALTER TABLE %I ADD COLUMN death_scope_id BYTEA GENERATED ALWAYS AS
    (CASE WHEN death_channel_id IS NOT NULL THEN decode('01','hex')||uuid_send(death_channel_id)
          ELSE decode('02','hex')||uuid_send(death_instance_id) END) STORED NOT NULL$ddl$,relation_name);
  EXECUTE format('ALTER TABLE %I ADD PRIMARY KEY (death_world_id,death_scope_id,death_scope_ownership_generation,death_actor_local_id,death_actor_local_generation,loot_table_family,loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal)',relation_name);
 END LOOP;
END $migration$;

-- Preserve all independently installed proof arms. Only the creature-MINT
-- comparisons change: each compares the full tagged scope, not nullable IDs.
DO $migration$
DECLARE function_name TEXT; definition TEXT;
BEGIN
 FOREACH function_name IN ARRAY ARRAY['game_item_mint_consistency_guard()',
  'game_kill_nested_loot_mint_proven(uuid)','game_item_mint_receipt_materialize_corpse()'] LOOP
  definition:=pg_get_functiondef(function_name::regprocedure);
  IF strpos(definition,'death_channel_id')=0 THEN
   RAISE EXCEPTION 'creature MINT scope proof changed: %',function_name;
  END IF;
  definition:=replace(definition,'g.channel_id = r.death_channel_id',
   '(g.channel_id IS NOT DISTINCT FROM r.death_channel_id AND g.instance_id IS NOT DISTINCT FROM r.death_instance_id)');
  -- Protect this Ground comparison before converting the receipt-only tuples.
  definition:=replace(definition,'r.death_channel_id AND g.instance_id','r.__ground_channel_id AND g.instance_id');
  definition:=replace(definition,'v.death_channel_id','v.death_scope_id');
  definition:=replace(definition,'cr.death_channel_id','cr.death_scope_id');
  definition:=replace(definition,'r.death_channel_id','r.death_scope_id');
  definition:=replace(definition,'m.death_channel_id','m.death_scope_id');
  definition:=replace(definition,'child.death_channel_id','child.death_scope_id');
  definition:=replace(definition,'r.__ground_channel_id','r.death_channel_id');
  definition:=replace(definition,'death_channel_id = NEW.death_channel_id','death_scope_id = NEW.death_scope_id');
  EXECUTE definition;
 END LOOP;
END $migration$;

-- A stored generated column is filled after BEFORE UPDATE triggers. Exclude
-- only that derived projection from the existing exact immutability comparisons;
-- both source UUID columns stay immutable, and SQL owns the projection itself.
DO $migration$
DECLARE function_name TEXT; definition TEXT; field_name TEXT;
BEGIN
 FOREACH function_name IN ARRAY ARRAY['game_item_mint_reservation_guard()',
  'game_item_mint_receipt_guard()'] LOOP
  field_name:=CASE WHEN function_name='game_item_mint_reservation_guard()'
    THEN 'work_units_used' ELSE 'materialized_at' END;
  definition:=pg_get_functiondef(function_name::regprocedure);
  IF strpos(definition,format('to_jsonb(NEW) - %L',field_name))=0 THEN
   RAISE EXCEPTION 'creature MINT immutability proof changed: %',function_name;
  END IF;
  definition:=replace(definition,format('to_jsonb(NEW) - %L',field_name),
   format('to_jsonb(NEW) - %L - ''death_scope_id''',field_name));
  definition:=replace(definition,format('to_jsonb(OLD) - %L',field_name),
   format('to_jsonb(OLD) - %L - ''death_scope_id''',field_name));
  EXECUTE definition;
 END LOOP;
END $migration$;

-- Preserve exact source race custody as well as physical placement. A shared
-- corpse Item definition is never authority for the killed Creature's race.
DO $migration$
DECLARE definition TEXT; before TEXT := 'g.channel_id=r.death_channel_id';
BEGIN
 definition:=pg_get_functiondef('game_corpse_source_race_admit()'::regprocedure);
 IF strpos(definition,before)=0 THEN
  RAISE EXCEPTION 'corpse source race scope proof changed';
 END IF;
 EXECUTE replace(definition,before,
  '(g.channel_id IS NOT DISTINCT FROM r.death_channel_id AND g.instance_id IS NOT DISTINCT FROM r.death_instance_id)');
END $migration$;
