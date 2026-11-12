-- SRX-2 P01: exact Creature identity follows the atomic corpse MINT.
-- Shared corpse Item ids are presentation, never race authority.
CREATE TABLE game_corpse_source_races (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 mint_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_mint_receipts(transaction_id),
 race_key TEXT NOT NULL CHECK(octet_length(race_key) BETWEEN 1 AND 128),
 race_definition_revision TEXT NOT NULL CHECK(octet_length(race_definition_revision) BETWEEN 1 AND 128),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_corpse_source_race_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_mint_receipts r
   JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
   JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
   WHERE r.transaction_id=NEW.mint_transaction_id AND r.item_instance_id=NEW.item_instance_id
    AND r.created_xact_id=pg_current_xact_id() AND r.loot_purpose_key='CORPSE_MATERIALIZATION'
    AND r.draw_ordinal=0 AND r.destination_parent_item_instance_id IS NULL
    AND i.minted_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.quantity=1
    AND i.world_id=r.death_world_id AND g.world_id=r.death_world_id
    AND g.channel_id=r.death_channel_id
    AND g.runtime_scope_ownership_generation=r.death_scope_ownership_generation
    AND i.definition_family=r.loot_table_family
    AND i.definition_production_key=r.loot_table_production_key
    AND i.definition_revision_ref=r.loot_table_revision_ref)
 THEN RAISE EXCEPTION 'corpse race requires this transaction exact corpse mint' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_corpse_source_race_admit BEFORE INSERT ON game_corpse_source_races
 FOR EACH ROW EXECUTE FUNCTION game_corpse_source_race_admit();
CREATE TRIGGER game_corpse_source_race_immutable BEFORE UPDATE OR DELETE ON game_corpse_source_races
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_corpse_source_race_no_truncate BEFORE TRUNCATE ON game_corpse_source_races
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_corpse_source_races FROM PUBLIC;
REVOKE ALL ON FUNCTION game_corpse_source_race_admit() FROM PUBLIC;
GRANT SELECT,INSERT ON game_corpse_source_races TO oteryn_game_runtime;
GRANT SELECT ON game_corpse_source_races TO oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_corpse_source_race_admit() SET search_path = %I, pg_temp',current_schema());
END $$;
CREATE TABLE game_corpse_skinning_records (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_craft_records(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 corpse_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 before_item JSONB NOT NULL CHECK(jsonb_typeof(before_item)='object'),
 spent_key TEXT NOT NULL CHECK(octet_length(spent_key) BETWEEN 1 AND 512),
 spent_revision TEXT NOT NULL CHECK(octet_length(spent_revision) BETWEEN 1 AND 512),
 channel_id UUID NOT NULL CHECK(game_character_is_uuid_v7(channel_id)),
 scope_ownership_generation NUMERIC(20,0) NOT NULL CHECK(scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_corpse_skinning_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i
   JOIN game_item_ground_locations g USING(item_instance_id)
   JOIN game_corpse_source_races s USING(item_instance_id)
   WHERE i.item_instance_id=NEW.corpse_item_instance_id AND to_jsonb(i)=NEW.before_item
    AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
    AND s.mint_transaction_id=i.minted_transaction_id
    AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)
    AND g.world_id=i.world_id AND g.channel_id=NEW.channel_id
    AND g.runtime_scope_ownership_generation=NEW.scope_ownership_generation
    AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id))
 THEN RAISE EXCEPTION 'skinning requires exact source corpse in current ground scope' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE FUNCTION game_corpse_skinning_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_craft_records c
   JOIN game_item_instances i ON i.item_instance_id=NEW.corpse_item_instance_id
   JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
   WHERE c.transaction_id=NEW.transaction_id AND c.created_xact_id=pg_current_xact_id()
    AND NOT c.consume_source AND c.recipe='oteryn:skinning.crystal'
    AND c.world_id=i.world_id AND g.world_id=c.world_id AND g.channel_id=NEW.channel_id
    AND g.runtime_scope_ownership_generation=NEW.scope_ownership_generation
    AND jsonb_array_length(c.outputs) BETWEEN 0 AND 1
    AND c.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')
    AND i.definition_production_key=NEW.spent_key AND i.definition_revision_ref=NEW.spent_revision
    AND i.state_revision=(NEW.before_item->>'state_revision')::numeric+1
    AND i.last_transaction_id=NEW.transaction_id
    AND (to_jsonb(i)-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
       =(NEW.before_item-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id']))
 THEN RAISE EXCEPTION 'incomplete atomic corpse skinning' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_corpse_skinning_admit BEFORE INSERT ON game_corpse_skinning_records
 FOR EACH ROW EXECUTE FUNCTION game_corpse_skinning_admit();
CREATE CONSTRAINT TRIGGER game_corpse_skinning_complete AFTER INSERT ON game_corpse_skinning_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_corpse_skinning_complete();
CREATE TRIGGER game_corpse_skinning_immutable BEFORE UPDATE OR DELETE ON game_corpse_skinning_records
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_corpse_skinning_no_truncate BEFORE TRUNCATE ON game_corpse_skinning_records
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Preserve every existing owner's constraint/proof body and add one named recipe.
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT; arm TEXT; n TEXT; begin_at INT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
 IF body IS NULL THEN RAISE EXCEPTION 'missing craft outputs constraint'; END IF;
 EXECUTE 'ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check';
 EXECUTE 'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK (('
  ||substring(body FROM 7 FOR length(body)-6)||') OR (recipe=''oteryn:skinning.crystal'' AND NOT consume_source AND jsonb_typeof(outputs)=''array'' AND jsonb_array_length(outputs) BETWEEN 0 AND 1))';
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 before:='IF NOT NEW.consume_source OR NEW.recipe=''oteryn:gathering.crystal'' THEN';
 after:='IF (NOT NEW.consume_source OR NEW.recipe=''oteryn:gathering.crystal'') AND NEW.recipe<>''oteryn:skinning.crystal'' THEN';
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected gathering admission guard'; END IF;
 body:=replace(body,before,after);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 arm:=$arm$
 IF NEW.recipe='oteryn:skinning.crystal' AND (NEW.consume_source OR jsonb_array_length(NEW.outputs)>1
  OR NOT EXISTS(SELECT 1 FROM game_corpse_skinning_records s
   WHERE s.transaction_id=NEW.transaction_id AND s.created_xact_id=pg_current_xact_id()
    AND (s.before_item->>'world_id')::uuid=NEW.world_id
    AND NEW.before_item->>'definition_production_key' IN ('oteryn:item.tibia.i5908','oteryn:item.tibia.i5942')))
 THEN RAISE EXCEPTION 'skinning requires exact corpse and unchanged tool proof' USING ERRCODE='23514'; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
 before:='(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.consume_source THEN 1 ELSE 0 END)';
 after:='(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.consume_source OR r.recipe=''oteryn:skinning.crystal'' THEN 1 ELSE 0 END)';
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected craft change count'; END IF;
 EXECUTE replace(body,before,after);
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
 IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_corpse_skinning_records s
   JOIN game_item_craft_records c USING(transaction_id)
   WHERE s.corpse_item_instance_id=OLD.item_instance_id AND s.before_item=to_jsonb(OLD)
    AND s.transaction_id=NEW.last_transaction_id AND s.created_xact_id=pg_current_xact_id()
    AND c.recipe='oteryn:skinning.crystal' AND NOT c.consume_source
    AND NEW.definition_production_key=s.spent_key AND NEW.definition_revision_ref=s.spent_revision
    AND NEW.state_revision=OLD.state_revision+1
    AND (to_jsonb(NEW)-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
       =(to_jsonb(OLD)-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id']))
 THEN RETURN NEW; END IF;
$arm$;
  ELSE
   arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_corpse_skinning_records s
   JOIN game_item_craft_records c USING(transaction_id)
   WHERE s.corpse_item_instance_id=NEW.item_instance_id AND s.transaction_id=NEW.last_transaction_id
    AND s.created_xact_id=pg_current_xact_id() AND c.recipe='oteryn:skinning.crystal' AND NOT c.consume_source)
 THEN RETURN NULL; END IF;
$arm$;
  END IF;
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected item proof body %',n; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
REVOKE ALL ON game_corpse_skinning_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_corpse_skinning_admit(),game_corpse_skinning_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_corpse_skinning_records TO oteryn_game_runtime;
GRANT SELECT ON game_corpse_skinning_records TO oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_corpse_skinning_admit() SET search_path = %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_corpse_skinning_complete() SET search_path = %I, pg_temp',current_schema());
END $$;
