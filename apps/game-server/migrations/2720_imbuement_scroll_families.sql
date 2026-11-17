-- T174: explicit elemental scroll families 21-25 and Vibrancy 26.
-- Preserve existing category identities and every immutable receipt.
ALTER TABLE game_item_imbuements DROP CONSTRAINT game_item_imbuements_category_check;
ALTER TABLE game_item_imbuements ADD CONSTRAINT game_item_imbuements_category_check CHECK(category BETWEEN 0 AND 25);
ALTER TABLE game_item_imbuement_records DROP CONSTRAINT game_item_imbuement_records_category_check;
ALTER TABLE game_item_imbuement_records ADD CONSTRAINT game_item_imbuement_records_category_check CHECK(category BETWEEN 0 AND 25);
ALTER TABLE game_item_imbuement_item_records DROP CONSTRAINT game_item_imbuement_item_records_category_check;
ALTER TABLE game_item_imbuement_item_records ADD CONSTRAINT game_item_imbuement_item_records_category_check CHECK(category BETWEEN 0 AND 25);
-- At most one elemental conversion, and one paralysis imbuement, per item.
CREATE UNIQUE INDEX game_item_imbuements_exclusion_group ON game_item_imbuements
 (item_instance_id, (CASE WHEN category BETWEEN 20 AND 24 THEN 1 WHEN category=25 THEN 12 ELSE category END));


-- Inscription transforms an existing blank, but 0229 never extended the definition guard.
-- Admit only the immutable receipt in this physical transaction; retain all prior arms.
DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body := pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_OP='UPDATE' AND OLD.definition_family='Item'
     AND OLD.definition_production_key='oteryn:item.tibia.i51442'
     AND OLD.quantity=1 AND OLD.lifecycle=1
     AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
     AND (to_jsonb(NEW)-ARRAY['definition_family','definition_production_key','definition_revision_ref','last_transaction_id','state_revision'])
       = (to_jsonb(OLD)-ARRAY['definition_family','definition_production_key','definition_revision_ref','last_transaction_id','state_revision'])
     AND EXISTS(SELECT 1 FROM game_item_imbuement_records r
       JOIN game_item_container_entries e ON e.item_instance_id=r.item_instance_id
       WHERE r.action=3 AND r.transaction_id=NEW.last_transaction_id
         AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
         AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)
           =(r.output_definition_family,r.output_production_key,r.output_revision_ref)
         AND e.character_id=r.character_id AND e.world_id=r.world_id
         AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=r.item_ordinal
         AND r.created_xact_id=pg_current_xact_id()) THEN
    NEW.state_revision := OLD.state_revision+1;
    RETURN NEW;
  END IF;
$arm$ from begin_at for 5);
 body := pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF EXISTS(SELECT 1 FROM game_item_imbuement_records r
    WHERE r.action=3 AND r.transaction_id=NEW.last_transaction_id
      AND r.item_instance_id=NEW.item_instance_id AND r.world_id=NEW.world_id
      AND NEW.quantity=1 AND NEW.lifecycle=1 AND NEW.state_revision=OLD.state_revision+1
      AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)
        =(r.output_definition_family,r.output_production_key,r.output_revision_ref)
      AND r.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_item_imbuement_complete(NEW.last_transaction_id);
    RETURN NULL;
  END IF;
$arm$ from begin_at for 5);
END $migration$;
