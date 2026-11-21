-- M49: retain exact boss realizations across owner passes. A real corpse MINT
-- releases only its own arrival latch, in the death's physical transaction.
CREATE TABLE game_elemental_boss_realizations (
 arrival_transaction_id UUID NOT NULL REFERENCES game_elemental_altar_receipts,
 world_id UUID NOT NULL, channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 actor_local_id BIGINT NOT NULL CHECK(actor_local_id BETWEEN 0 AND 4294967295),
 actor_local_generation NUMERIC(20,0) NOT NULL CHECK(actor_local_generation BETWEEN 1 AND 18446744073709551615),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(arrival_transaction_id,ownership_generation),
 UNIQUE(world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation)
);
CREATE FUNCTION game_elemental_realization_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT EXISTS(SELECT 1 FROM game_elemental_boss_latches l
   JOIN game_elemental_altar_receipts r ON r.transaction_id=l.arrival_transaction_id
   JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id
   JOIN game_runtime_scope_assignments a ON a.world_id=l.world_id AND a.channel_id=l.channel_id
     AND a.scope_kind=1 AND a.state=1 AND a.ownership_generation=NEW.ownership_generation
   WHERE l.arrival_transaction_id=NEW.arrival_transaction_id
     AND l.world_id=NEW.world_id AND l.channel_id=NEW.channel_id
     AND game_world_reset_map_epoch_current(l.world_id,l.reset_epoch)
     AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=l.world_id AND c.channel_id=l.channel_id
       AND c.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=l.world_id AND channel_id=l.channel_id)
       AND c.server_artifact_digest=m.content_digest))
 THEN RAISE EXCEPTION 'boss realization requires its pending arrival and current scope/content' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_elemental_realization_admit BEFORE INSERT ON game_elemental_boss_realizations
 FOR EACH ROW EXECUTE FUNCTION game_elemental_realization_admit();
CREATE TRIGGER game_elemental_realization_immutable BEFORE UPDATE OR DELETE ON game_elemental_boss_realizations
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_elemental_realization_no_truncate BEFORE TRUNCATE ON game_elemental_boss_realizations
 EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_elemental_latch_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_elemental_boss_realizations a
   JOIN game_item_mint_receipts m ON m.death_world_id=a.world_id AND m.death_channel_id=a.channel_id
     AND m.death_scope_ownership_generation=a.ownership_generation AND m.death_actor_local_id=a.actor_local_id
     AND m.death_actor_local_generation=a.actor_local_generation
   WHERE a.arrival_transaction_id=OLD.arrival_transaction_id AND a.world_id=OLD.world_id AND a.channel_id=OLD.channel_id
     AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND m.created_xact_id=pg_current_xact_id())
 THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'boss latch releases only with its exact realized actor corpse receipt' USING ERRCODE='23514';
END $$;
DROP TRIGGER game_elemental_boss_latches_immutable ON game_elemental_boss_latches;
CREATE TRIGGER game_elemental_boss_latches_immutable BEFORE UPDATE OR DELETE ON game_elemental_boss_latches
 FOR EACH ROW EXECUTE FUNCTION game_elemental_latch_mutation();
CREATE FUNCTION game_elemental_boss_death() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.loot_purpose_key='CORPSE_MATERIALIZATION' AND NEW.draw_ordinal=0 THEN
   DELETE FROM game_elemental_boss_latches l USING game_elemental_boss_realizations a
    WHERE l.arrival_transaction_id=a.arrival_transaction_id AND l.world_id=a.world_id AND l.channel_id=a.channel_id
      AND a.world_id=NEW.death_world_id AND a.channel_id=NEW.death_channel_id
      AND a.ownership_generation=NEW.death_scope_ownership_generation
      AND a.actor_local_id=NEW.death_actor_local_id AND a.actor_local_generation=NEW.death_actor_local_generation;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_elemental_boss_death AFTER INSERT ON game_item_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_elemental_boss_death();
DO $migration$
DECLARE n TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_elemental_realization_admit()','game_elemental_latch_mutation()','game_elemental_boss_death()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON game_elemental_boss_realizations FROM PUBLIC;
REVOKE ALL ON FUNCTION game_elemental_realization_admit(),game_elemental_latch_mutation(),game_elemental_boss_death() FROM PUBLIC;
GRANT SELECT,INSERT ON game_elemental_boss_realizations TO oteryn_game_runtime;
GRANT SELECT ON game_elemental_boss_realizations TO oteryn_game_control;
