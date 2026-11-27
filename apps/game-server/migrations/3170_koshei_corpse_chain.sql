-- Crystal 201189f8 items.xml:20777–20802 and 84143–84158. Koshei’s final remains do not decay.
DO $koshei_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i7535';
    arbaziloth CONSTANT TEXT := 'oteryn:item.tibia.i50029';
    chain_anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    duration_anchor CONSTANT TEXT := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL THEN';
    body TEXT;
BEGIN
    IF game_summer_corpse_chain(root) = ARRAY[7535,7537,7538,7539,7540]
        AND game_source_corpse_durations_v1(root) = ARRAY[10000,10000,300000,300000,0]::BIGINT[]
        AND game_summer_corpse_chain(arbaziloth) = ARRAY[50029,50028,50027]
        AND game_source_corpse_durations_v1(arbaziloth) = ARRAY[60000,60000,60000]::BIGINT[] THEN
        RETURN;
    END IF;
    IF game_summer_corpse_chain(root) IS NOT NULL
        OR game_source_corpse_durations_v1(root) IS NOT NULL
        OR game_summer_corpse_chain(arbaziloth) IS NOT NULL
        OR game_source_corpse_durations_v1(arbaziloth) IS NOT NULL THEN
        RAISE EXCEPTION 'Koshei or Arbaziloth corpse root already has a source owner';
    END IF;

    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    IF (length(body)-length(replace(body,chain_anchor,'')))/length(chain_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body, chain_anchor,
        'WHEN ''oteryn:item.tibia.i7535'' THEN ARRAY[7535,7537,7538,7539,7540] '
        || 'WHEN ''oteryn:item.tibia.i50029'' THEN ARRAY[50029,50028,50027] ' || chain_anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    IF (length(body)-length(replace(body,duration_anchor,'')))/length(duration_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse duration registry anchor changed';
    END IF;
    EXECUTE replace(body, duration_anchor,
        'WHEN p_key=''oteryn:item.tibia.i7535'' THEN ARRAY[10000,10000,300000,300000,0]::BIGINT[] '
        || 'WHEN p_key=''oteryn:item.tibia.i50029'' THEN ARRAY[60000,60000,60000]::BIGINT[] ' || duration_anchor);

    IF game_summer_corpse_chain(root) IS DISTINCT FROM ARRAY[7535,7537,7538,7539,7540]
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,10000,300000,300000,0]::BIGINT[]
        OR game_summer_corpse_chain(arbaziloth) IS DISTINCT FROM ARRAY[50029,50028,50027]
        OR game_source_corpse_durations_v1(arbaziloth) IS DISTINCT FROM ARRAY[60000,60000,60000]::BIGINT[] THEN
        RAISE EXCEPTION 'Koshei or Arbaziloth corpse chain diverges from its source policy';
    END IF;
END $koshei_corpse_chain$;

-- Keep the newest inert Koshei remains on Ground. Superseded remains use
-- the existing retirement owner (including any remaining corpse entries).
-- Their eligibility is derived even while an older corpse's Ground row is
-- being removed by that owner's deferred consistency guard.
CREATE OR REPLACE FUNCTION game_corpse_retirement_deadline(p_item UUID) RETURNS BIGINT
LANGUAGE sql STABLE STRICT AS $$
 SELECT CASE
  WHEN m.loot_table_production_key='oteryn:item.tibia.i7535' THEN
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
   ) THEN m.materialized_at+(SELECT sum(duration) FROM unnest(original.d) AS source(duration)) END
  WHEN original.d[array_length(original.d,1)]=0 THEN NULL
  ELSE m.materialized_at+COALESCE((
    SELECT sum(duration) FROM unnest(original.d) AS source(duration)),60000)
 END
 FROM game_item_mint_receipts m CROSS JOIN LATERAL (
  SELECT game_source_corpse_durations_v1(m.loot_table_production_key) d
 ) original
 WHERE m.item_instance_id=p_item AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
$$;

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
    EXECUTE format('ALTER FUNCTION game_corpse_retirement_deadline(uuid) SET search_path=%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_fresh_corpse(uuid,text) SET search_path=%I,pg_temp', current_schema());
END $koshei_search_path$;
