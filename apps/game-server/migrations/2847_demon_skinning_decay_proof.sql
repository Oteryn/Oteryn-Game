-- Skinning may advance the pinned fresh demon corpse to its existing spent stage.
-- Keep the original immutable absolute decay chain; no timer is removed or extended.
CREATE FUNCTION game_corpse_skinning_temporal_allowed(p_item UUID, p_spent TEXT, p_revision TEXT)
RETURNS BOOLEAN LANGUAGE sql AS $$
 SELECT NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=p_item)
 OR EXISTS(
  SELECT 1 FROM game_item_instances i
  JOIN game_item_mint_receipts m ON m.transaction_id=i.minted_transaction_id AND m.item_instance_id=i.item_instance_id
  JOIN game_corpse_source_races r ON r.item_instance_id=i.item_instance_id AND r.mint_transaction_id=m.transaction_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
   AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i5995' AND i.definition_revision_ref='definition-r1'
   AND (m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)=('Item','oteryn:item.tibia.i5995','definition-r1')
   AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
   AND r.race_key='oteryn:creature.demon' AND r.race_definition_revision='definition-r1'
   AND p_spent='oteryn:item.tibia.i4098' AND p_revision='definition-r1'
   AND m.materialized_at+10000>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
   AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts t WHERE t.item_instance_id=i.item_instance_id)
   AND (SELECT count(*)=3 AND bool_and(
    t.source_transaction_id=m.transaction_id AND t.source_ordinal=0 AND t.source_stage BETWEEN 1 AND 3
    AND t.definition_family='Item' AND t.definition_revision='definition-r1'
    AND t.definition_production_key='oteryn:item.tibia.i'||(ARRAY[5995,4097,4098])[t.source_stage]::text
    AND (t.target_family,t.target_production_key,t.target_revision) IS NOT DISTINCT FROM
        ('Item','oteryn:item.tibia.i'||(ARRAY[4097,4098,4099])[t.source_stage]::text,'definition-r1')
    AND t.duration_millis=CASE WHEN t.source_stage=1 THEN 10000 ELSE 300000 END
    AND t.expires_at_unix_ms=m.materialized_at+(ARRAY[10000,310000,610000])[t.source_stage]
    AND t.content_digest=sha256(convert_to(g.content_revision,'UTF8'))
    AND NOT t.blocks_movement AND NOT t.blocks_projectile AND NOT t.immovable_block_solid)
    FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)
 );
$$;
REVOKE ALL ON FUNCTION game_corpse_skinning_temporal_allowed(UUID,TEXT,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_corpse_skinning_temporal_allowed(UUID,TEXT,TEXT) TO oteryn_game_runtime,oteryn_game_control;
DO $$
DECLARE body TEXT; anchor TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_corpse_skinning_temporal_allowed(UUID,TEXT,TEXT) SET search_path = %I, pg_temp',current_schema());
 body:=pg_get_functiondef('game_corpse_skinning_admit()'::regprocedure);
 anchor:='NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected corpse skinning temporal guard';
 END IF;
 EXECUTE replace(body,anchor,'game_corpse_skinning_temporal_allowed(i.item_instance_id,NEW.spent_key,NEW.spent_revision)');
END $$;

-- A later stage-three decay follows the exact skinning receipt instead of a
-- stage-two decay receipt. All existing due/custody/audit checks still run.
DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
 anchor:=' THEN RAISE EXCEPTION ''field stage requires exact preceding receipt''';
 replacement:=$guard$ AND NOT (
  r.source_stage=3 AND p.definition_family='Item' AND p.definition_production_key='oteryn:item.tibia.i4098'
  AND p.definition_revision='definition-r1'
  AND (p.target_family,p.target_production_key,p.target_revision) IS NOT DISTINCT FROM ('Item','oteryn:item.tibia.i4099','definition-r1')
  AND EXISTS(SELECT 1 FROM game_corpse_skinning_records s
   JOIN game_item_craft_records c USING(transaction_id)
   JOIN game_corpse_source_races race ON race.item_instance_id=s.corpse_item_instance_id
   WHERE s.corpse_item_instance_id=OLD.item_instance_id AND s.transaction_id=OLD.last_transaction_id
    AND s.spent_key='oteryn:item.tibia.i4098' AND s.spent_revision='definition-r1'
    AND s.before_item->>'definition_family'='Item' AND s.before_item->>'definition_production_key'='oteryn:item.tibia.i5995'
    AND s.before_item->>'definition_revision_ref'='definition-r1' AND (s.before_item->>'state_revision')::numeric=1
    AND s.before_item->>'last_transaction_id' IS NULL
    AND race.race_key='oteryn:creature.demon' AND race.race_definition_revision='definition-r1'
    AND race.mint_transaction_id=OLD.minted_transaction_id AND p.source_transaction_id=OLD.minted_transaction_id AND p.source_ordinal=0
    AND c.recipe='oteryn:skinning.crystal' AND NOT c.consume_source AND c.world_id=r.world_id
    AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i5942'
    AND s.created_xact_id=c.created_xact_id AND s.channel_id=r.channel_id AND s.scope_ownership_generation=r.ownership_generation
    AND OLD.state_revision=2
    AND (to_jsonb(OLD)-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
        =(s.before_item-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id']))
 ) THEN RAISE EXCEPTION 'field stage requires exact preceding receipt'$guard$;
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected field temporal predecessor guard';
 END IF;
 EXECUTE replace(body,anchor,replacement);
END $migration$;

-- The newly admitted timed demon case is stake-only, as in the pinned donor.
DO $migration$
DECLARE body TEXT; anchor TEXT;
BEGIN
 body:=pg_get_functiondef('game_corpse_skinning_complete()'::regprocedure);
 anchor:=$exact$c.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')$exact$;
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected corpse skinning tool guard';
 END IF;
 EXECUTE replace(body,anchor,'('||anchor||$guard$ AND (
  NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)
  OR c.before_item->>'definition_production_key'='oteryn:item.tibia.i5942'))$guard$);
END $migration$;
