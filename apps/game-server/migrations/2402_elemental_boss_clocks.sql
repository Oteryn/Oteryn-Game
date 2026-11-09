-- M49 reuses the native boss clock and its corpse-MINT death transaction.
-- Explicit arrivals admit these one-shot clocks; no reset draw admits them.
CREATE INDEX game_elemental_arrival_receipts_altar ON game_elemental_arrival_receipts(altar_id);
CREATE INDEX game_elemental_summon_intents_altar ON game_elemental_summon_intents(altar_id);
CREATE INDEX game_elemental_boss_leases_altar ON game_elemental_boss_leases(altar_id);
CREATE INDEX game_elemental_current_corpse_mints ON game_item_mint_receipts
 (created_xact_id,death_world_id,death_channel_id,death_scope_ownership_generation)
 WHERE loot_purpose_key='CORPSE_MATERIALIZATION';

CREATE FUNCTION game_elemental_intent_clock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO game_boss_spawn_clocks(world_id,channel_id,spawn_key,cycle,state,reset_epoch,due_at,owner_generation)
 VALUES(NEW.world_id,NEW.channel_id,'qn1.elemental:'||replace(NEW.transaction_id::text,'-',''),0,2,NEW.reset_epoch,NULL,NEW.scope_generation);
 RETURN NEW;
END $$;
CREATE TRIGGER game_elemental_intent_clock AFTER INSERT ON game_elemental_summon_intents
 FOR EACH ROW EXECUTE FUNCTION game_elemental_intent_clock();
-- Preserve already committed, unprojected arrivals when upgrading this lane.
-- These are historical expected clock details, not a current owner grant.
INSERT INTO game_boss_spawn_clocks(world_id,channel_id,spawn_key,cycle,state,reset_epoch,due_at,owner_generation)
 SELECT b.world_id,b.channel_id,'qn1.elemental:'||replace(b.transaction_id::text,'-',''),0,2,b.reset_epoch,NULL,b.scope_generation
 FROM game_elemental_summon_intents b JOIN game_elemental_boss_leases l USING(transaction_id)
 ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION game_elemental_lease_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE b game_elemental_summon_intents%ROWTYPE;
BEGIN
 SELECT * INTO STRICT b FROM game_elemental_summon_intents WHERE transaction_id=NEW.transaction_id;
 IF b.created_xact_id<>pg_current_xact_id() OR (NEW.world_id,NEW.channel_id,NEW.reset_epoch,NEW.altar_id) IS DISTINCT FROM
    (b.world_id,b.channel_id,b.reset_epoch,b.altar_id)
    OR NOT EXISTS(SELECT 1 FROM game_boss_spawn_clocks c WHERE c.world_id=b.world_id AND c.channel_id=b.channel_id
      AND c.spawn_key='qn1.elemental:'||replace(b.transaction_id::text,'-','') AND c.state=2 AND c.cycle=0
      AND c.reset_epoch=b.reset_epoch AND c.owner_generation=b.scope_generation)
 THEN RAISE EXCEPTION 'elemental boss lease needs its current intent and native clock' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_elemental_clock_death_proven(p_world UUID,p_channel UUID,p_generation NUMERIC)
RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_mint_receipts m
   WHERE m.created_xact_id=pg_current_xact_id() AND m.death_world_id=p_world AND m.death_channel_id=p_channel
     AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
     AND (p_generation IS NULL OR m.death_scope_ownership_generation=p_generation))
$$;
CREATE FUNCTION game_elemental_lease_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- Only the nested, proven clock-death trigger may release an altar. Direct
 -- deletes, updates and historical corpse receipts never release a lease.
 IF TG_OP='DELETE' AND pg_trigger_depth()>1
   AND game_elemental_clock_death_proven(OLD.world_id,OLD.channel_id,NULL)
   AND EXISTS(SELECT 1 FROM game_boss_spawn_clocks c WHERE c.world_id=OLD.world_id AND c.channel_id=OLD.channel_id
      AND c.spawn_key='qn1.elemental:'||replace(OLD.transaction_id::text,'-','') AND c.state=3 AND c.reset_epoch=OLD.reset_epoch)
 THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'elemental lease release requires its native corpse-MINT clock death' USING ERRCODE='23514';
END $$;
DROP TRIGGER game_elemental_boss_leases_immutable ON game_elemental_boss_leases;
CREATE TRIGGER game_elemental_boss_leases_immutable BEFORE UPDATE OR DELETE ON game_elemental_boss_leases
 FOR EACH ROW EXECUTE FUNCTION game_elemental_lease_mutation();

CREATE FUNCTION game_elemental_clock_closed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE lease game_elemental_boss_leases%ROWTYPE;
BEGIN
 SELECT * INTO lease FROM game_elemental_boss_leases l WHERE l.world_id=OLD.world_id AND l.channel_id=OLD.channel_id
   AND l.reset_epoch=OLD.reset_epoch AND OLD.spawn_key='qn1.elemental:'||replace(l.transaction_id::text,'-','') LIMIT 1;
 IF lease.transaction_id IS NULL THEN RETURN NULL; END IF;
 IF (NEW.world_id,NEW.channel_id,NEW.spawn_key,NEW.reset_epoch) IS DISTINCT FROM
    (OLD.world_id,OLD.channel_id,OLD.spawn_key,OLD.reset_epoch) OR OLD.state<>2
 THEN RAISE EXCEPTION 'elemental clock identity is fixed for its live lease' USING ERRCODE='23514'; END IF;
 IF NEW.state=2 AND NEW.cycle=OLD.cycle AND NEW.owner_generation>=OLD.owner_generation THEN RETURN NULL; END IF;
 IF NEW.state<>3 OR NEW.cycle<>OLD.cycle+1 OR NOT game_elemental_clock_death_proven(OLD.world_id,OLD.channel_id,OLD.owner_generation)
 THEN RAISE EXCEPTION 'elemental clock closes only with its current owner corpse MINT' USING ERRCODE='23514'; END IF;
 DELETE FROM game_elemental_boss_leases WHERE transaction_id=lease.transaction_id;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_elemental_clock_closed AFTER UPDATE ON game_boss_spawn_clocks
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_clock_closed();

DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_elemental_intent_clock()','game_elemental_lease_admit()',
  'game_elemental_clock_death_proven(uuid,uuid,numeric)','game_elemental_lease_mutation()','game_elemental_clock_closed()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
GRANT DELETE ON game_elemental_boss_leases TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_elemental_clock_death_proven(UUID,UUID,NUMERIC) TO oteryn_game_runtime;
