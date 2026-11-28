-- The insert statement decides freshness after the sampler owns both custody locks.
-- Older immutable samples have no recorded observation time.
ALTER TABLE game_quest_corpse_samples ADD COLUMN sampled_ms BIGINT;

CREATE OR REPLACE FUNCTION game_quest_fresh_corpse_at(p_item UUID, p_definition TEXT, sampled_ms BIGINT) RETURNS BOOLEAN
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
      AND m.materialized_at<=sampled_ms
      AND s.expires_at_unix_ms>sampled_ms
 )
$$;
CREATE OR REPLACE FUNCTION game_quest_fresh_corpse(p_item UUID, p_definition TEXT) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT game_quest_fresh_corpse_at(p_item,p_definition,floor(extract(epoch FROM statement_timestamp())*1000)::bigint)
$$;

CREATE FUNCTION game_quest_sample_fresh(p_item UUID, appearance BIGINT, age_ms INTEGER, sampled_ms BIGINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
 SELECT CASE WHEN appearance=7535 THEN game_quest_fresh_corpse_at(p_item,'oteryn:item.tibia.i7535',sampled_ms)
 ELSE EXISTS(SELECT 1 FROM game_item_mint_receipts r JOIN game_item_instances i
              ON i.minted_transaction_id=r.transaction_id AND i.item_instance_id=r.item_instance_id
             WHERE i.item_instance_id=p_item AND r.loot_purpose_key='CORPSE_MATERIALIZATION'
               AND sampled_ms-r.materialized_at BETWEEN 0 AND age_ms-1) END
$$;

CREATE FUNCTION game_quest_stamp_corpse_sample() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.sampled_ms := floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 IF game_quest_sample_fresh(NEW.corpse_item,NEW.appearance,NEW.maximum_age_ms,NEW.sampled_ms) THEN
  RETURN NEW;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_quest_stamp_corpse_sample BEFORE INSERT ON game_quest_corpse_samples
 FOR EACH ROW EXECUTE FUNCTION game_quest_stamp_corpse_sample();

DO $migration$
DECLARE
 body TEXT := pg_get_functiondef('game_quest_corpse_sample_complete()'::regprocedure);
 legacy CONSTANT TEXT := '(CASE WHEN sample_row.appearance=7535 THEN game_quest_fresh_corpse(i.item_instance_id,i.definition_production_key) ELSE floor(extract(epoch FROM transaction_timestamp())*1000)::BIGINT-r.occurred_at BETWEEN 0 AND sample_row.maximum_age_ms-1 END)';
 sig TEXT;
BEGIN
 IF (length(body)-length(replace(body,legacy,'')))/length(legacy) <> 1 THEN
  RAISE EXCEPTION 'Quest corpse sample admission predicate changed';
 END IF;
 EXECUTE replace(body,legacy,'(CASE WHEN sample_row.sampled_ms IS NULL THEN ' || legacy ||
   ' ELSE game_quest_sample_fresh(i.item_instance_id,sample_row.appearance,sample_row.maximum_age_ms,sample_row.sampled_ms) END)');
 FOREACH sig IN ARRAY ARRAY['game_quest_fresh_corpse(uuid,text)','game_quest_fresh_corpse_at(uuid,text,bigint)',
  'game_quest_sample_fresh(uuid,bigint,integer,bigint)','game_quest_stamp_corpse_sample()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',sig,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_quest_fresh_corpse_at(UUID,TEXT,BIGINT),
 game_quest_sample_fresh(UUID,BIGINT,INTEGER,BIGINT),game_quest_stamp_corpse_sample() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_fresh_corpse_at(UUID,TEXT,BIGINT),
 game_quest_sample_fresh(UUID,BIGINT,INTEGER,BIGINT) TO oteryn_game_runtime;
