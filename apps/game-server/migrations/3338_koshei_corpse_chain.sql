-- Crystal 201189f8 items.xml:20777–20802. Koshei's final remains do not decay.
DO $koshei_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i7535';
    chain_anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    duration_anchor CONSTANT TEXT := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL THEN';
    deadline_anchor CONSTANT TEXT := 'SELECT CASE WHEN original.d[array_length(original.d,1)]=0 THEN NULL';
    body TEXT;
BEGIN
    IF game_summer_corpse_chain(root) = ARRAY[7535,7537,7538,7539,7540]
        AND game_source_corpse_durations_v1(root) = ARRAY[10000,10000,300000,300000,0]::BIGINT[]
        AND position('i7540' IN pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure)) > 0 THEN
        RETURN;
    END IF;
    IF game_summer_corpse_chain(root) IS NOT NULL
        OR game_source_corpse_durations_v1(root) IS NOT NULL THEN
        RAISE EXCEPTION 'Koshei corpse root already has a source owner';
    END IF;

    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    IF (length(body)-length(replace(body,chain_anchor,'')))/length(chain_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body, chain_anchor,
        'WHEN ''oteryn:item.tibia.i7535'' THEN ARRAY[7535,7537,7538,7539,7540] ' || chain_anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    IF (length(body)-length(replace(body,duration_anchor,'')))/length(duration_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse duration registry anchor changed';
    END IF;
    EXECUTE replace(body, duration_anchor,
        'WHEN p_key=''oteryn:item.tibia.i7535'' THEN ARRAY[10000,10000,300000,300000,0]::BIGINT[] ' || duration_anchor);

    IF game_summer_corpse_chain(root) IS DISTINCT FROM ARRAY[7535,7537,7538,7539,7540]
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,10000,300000,300000,0]::BIGINT[] THEN
        RAISE EXCEPTION 'Koshei corpse chain diverges from its source policy';
    END IF;

    -- Corpses minted before the chain was installed keep their 60s deadline.
    -- Only scheduled Koshei corpses can leave inert remains in this scope.
    body := pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure);
    IF (length(body)-length(replace(body,deadline_anchor,'')))/length(deadline_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse retirement deadline anchor changed';
    END IF;
    EXECUTE replace(body, deadline_anchor, $deadline$
 SELECT CASE
  WHEN m.loot_table_production_key='oteryn:item.tibia.i7535' AND NOT EXISTS (
    SELECT 1 FROM game_spell_field_temporal_schedules s
    WHERE s.item_instance_id=m.item_instance_id AND s.source_transaction_id=m.transaction_id
      AND s.source_ordinal=0 AND s.source_stage=1
  ) THEN m.materialized_at+60000
  WHEN m.loot_table_production_key='oteryn:item.tibia.i7535'
    AND original.d[array_length(original.d,1)]=0 THEN
   CASE WHEN EXISTS(
    SELECT 1 FROM game_item_ground_locations g
    JOIN game_item_instances i USING(item_instance_id,world_id)
    JOIN game_item_mint_receipts newer ON newer.transaction_id=i.minted_transaction_id
                                      AND newer.item_instance_id=i.item_instance_id
    WHERE g.world_id=m.death_world_id AND g.scope_kind=m.death_scope_kind
      AND g.channel_id IS NOT DISTINCT FROM m.death_channel_id
      AND g.instance_id IS NOT DISTINCT FROM m.death_instance_id
      AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i7540'
      AND newer.loot_purpose_key='CORPSE_MATERIALIZATION' AND newer.draw_ordinal=0
      AND newer.loot_table_production_key=m.loot_table_production_key
      AND (newer.materialized_at,newer.item_instance_id)>(m.materialized_at,m.item_instance_id)
   ) THEN m.materialized_at+(SELECT sum(duration) FROM pg_catalog.unnest(original.d) AS source(duration)) END
  WHEN original.d[array_length(original.d,1)]=0 THEN NULL$deadline$);
END $koshei_corpse_chain$;

-- Both USE owners check the original mint and its first-stage schedule.
CREATE OR REPLACE FUNCTION game_quest_fresh_corpse(p_item UUID, p_definition TEXT) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(
    SELECT 1 FROM game_item_instances i
    JOIN game_item_ground_locations g USING(item_instance_id,world_id)
    JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
                                 AND m.transaction_id=i.minted_transaction_id
    JOIN game_spell_field_temporal_schedules s ON s.item_instance_id=m.item_instance_id
        AND s.source_transaction_id=m.transaction_id AND s.source_ordinal=0 AND s.source_stage=1
    WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1
      AND i.definition_family='Item' AND i.definition_production_key=p_definition
      AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
          (m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)
      AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
      AND s.definition_production_key=p_definition AND s.created_xact_id=m.created_xact_id
      AND m.death_world_id=g.world_id AND m.death_channel_id=g.channel_id
      AND m.death_scope_kind=g.scope_kind AND g.instance_id IS NULL
      AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation
      AND m.materialized_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
      AND s.expires_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
 )
$$;
REVOKE ALL ON FUNCTION game_quest_fresh_corpse(UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_fresh_corpse(UUID,TEXT) TO oteryn_game_runtime;

DO $koshei_search_path$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_fresh_corpse(uuid,text) SET search_path=%I,pg_temp', current_schema());
END $koshei_search_path$;

-- Preserve the race, kill, adjacency and receipt proofs in the installed sampler.
DO $koshei_sample$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_corpse_sample_complete()'::regprocedure);
    age CONSTANT TEXT := 'floor(extract(epoch FROM transaction_timestamp())*1000)::BIGINT-r.occurred_at BETWEEN 0 AND sample_row.maximum_age_ms-1';
    predicate CONSTANT TEXT := '(CASE WHEN sample_row.appearance=7535 THEN game_quest_fresh_corpse(i.item_instance_id,i.definition_production_key) ELSE ' || age || ' END)';
BEGIN
    IF strpos(body,predicate)>0 THEN RETURN; END IF;
    IF (length(body)-length(replace(body,age,'')))/length(age) <> 1 THEN
        RAISE EXCEPTION 'Quest corpse sample age predicate changed';
    END IF;
    EXECUTE replace(body,age,predicate);
END $koshei_sample$;
