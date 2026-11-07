-- Native creature loot retains its full death cause in both runtime namespaces.
-- Existing Channel keys and event bytes remain unchanged. Instance identities
-- never borrow an origin Channel. Ground and corpse-container owners, their
-- capacity locks, and all installed non-creature proof arms remain in place.
DO $migration$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I', t, t || '_pkey');
        EXECUTE format('ALTER TABLE %I ALTER COLUMN death_channel_id DROP NOT NULL,
            ADD COLUMN death_scope_kind SMALLINT NOT NULL DEFAULT 1,
            ADD COLUMN death_instance_id UUID NULL,
            ADD COLUMN death_scope_id UUID GENERATED ALWAYS AS
                (COALESCE(death_channel_id,death_instance_id)) STORED', t);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (
            (death_scope_kind=1 AND death_channel_id IS NOT NULL AND death_instance_id IS NULL)
            OR (death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id IS NOT NULL
                AND game_character_is_uuid_v7(death_instance_id))),
            ADD PRIMARY KEY (death_world_id,death_scope_kind,death_scope_id,
                death_scope_ownership_generation,death_actor_local_id,death_actor_local_generation,
                loot_table_family,loot_table_production_key,loot_table_revision_ref,
                loot_purpose_key,draw_ordinal)', t, t || '_death_scope');
    END LOOP;
END
$migration$;

-- Stored generated values are recomputed after BEFORE triggers. Exclude only
-- that derived value; raw kind, Channel, Instance and all other columns retain
-- their existing immutability protection.
DO $migration$
DECLARE n TEXT; body TEXT; amended TEXT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_mint_reservation_guard','game_item_mint_receipt_guard'] LOOP
        body := pg_get_functiondef((n || '()')::regprocedure);
        amended := replace(replace(body, 'to_jsonb(NEW)', '(to_jsonb(NEW) - ''death_scope_id'')'),
            'to_jsonb(OLD)', '(to_jsonb(OLD) - ''death_scope_id'')');
        IF amended=body THEN RAISE EXCEPTION 'missing MINT immutability body: %', n; END IF;
        EXECUTE amended;
    END LOOP;
    body := pg_get_functiondef('game_item_mint_receipt_materialize_corpse()'::regprocedure);
    amended := replace(body, 'death_channel_id = NEW.death_channel_id',
        'death_scope_kind = NEW.death_scope_kind AND death_scope_id = NEW.death_scope_id');
    IF amended=body THEN RAISE EXCEPTION 'missing corpse materialization death key'; END IF;
    EXECUTE amended;
END
$migration$;

-- Amend the installed creature proof clauses instead of replacing historical
-- function bodies and losing subsequently admitted reward/craft/spell owners.
DO $migration$
DECLARE n TEXT; body TEXT; amended TEXT; a TEXT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_mint_consistency_guard','game_kill_nested_loot_mint_proven'] LOOP
        body := pg_get_functiondef((n || CASE WHEN n='game_kill_nested_loot_mint_proven'
            THEN '(uuid)' ELSE '()' END)::regprocedure);
        amended := body;
        FOREACH a IN ARRAY ARRAY['v','r','m','child'] LOOP
            amended := replace(amended, a || '.death_world_id, ' || a || '.death_channel_id,',
                a || '.death_world_id, ' || a || '.death_scope_kind, ' || a || '.death_scope_id,');
        END LOOP;
        amended := replace(amended, 'cr.death_channel_id = r.death_channel_id',
            '(cr.death_scope_kind,cr.death_scope_id) = (r.death_scope_kind,r.death_scope_id)');
        amended := replace(amended, 'g.channel_id = r.death_channel_id',
            '(g.scope_kind,COALESCE(g.channel_id,g.instance_id)) = (r.death_scope_kind,r.death_scope_id)');
        IF amended=body THEN RAISE EXCEPTION 'missing creature MINT proof body: %', n; END IF;
        EXECUTE amended;
    END LOOP;
END
$migration$;

-- 0630's Instance gate originally admitted only player-death item placement.
-- A creature MINT may now continue into the existing single-custody/fresh-MINT
-- guard only when its stamped receipt names this exact Instance Ground. It
-- does not return early: the installed ordinary MINT proof still runs.
DO $migration$
DECLARE body TEXT; amended TEXT; refusal TEXT;
BEGIN
    body := pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
    refusal := 'RAISE EXCEPTION ''Instance Ground requires its exact accepted placement cause'' USING ERRCODE=''23514'';';
    amended := replace(body, refusal, $arm$
    IF NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.item_instance_id=NEW.item_instance_id
           AND r.created_xact_id=pg_current_xact_id()
           AND r.death_world_id=NEW.world_id
           AND r.death_scope_kind=2 AND r.death_instance_id=NEW.instance_id
           AND r.death_scope_ownership_generation=NEW.runtime_scope_ownership_generation
           AND r.destination_parent_item_instance_id IS NULL
    ) THEN
        RAISE EXCEPTION 'Instance Ground requires its exact accepted placement cause' USING ERRCODE='23514';
    END IF;
$arm$);
    IF amended=body THEN RAISE EXCEPTION 'missing Instance Ground admission gate'; END IF;
    EXECUTE amended;
END
$migration$;
