-- Poor Soul's i33891 Mortal Essence is a movable plain Item, not a loot
-- container. Keep D134 for every other corpse root and require the original
-- lethal MINT/Ground proof even for this one exact content definition.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
 body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original := $guard$IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION') THEN$guard$;
 replacement := $guard$IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION')
      AND NOT COALESCE((NEW.to_kind IN (1,2)
        AND NEW.definition_family='Item'
        AND NEW.definition_production_key='oteryn:item.tibia.i33891'
        AND NEW.definition_revision_ref='definition-r1'
        AND NEW.quantity_before=1 AND NEW.quantity_after=1
        AND game_item_plain_corpse_source(NEW.item_instance_id)),false) THEN$guard$;
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'Mortal Essence pickup D134 predecessor changed';
 END IF;
 -- pg_get_functiondef retains SECURITY DEFINER and the pinned search_path;
 -- CREATE OR REPLACE retains ownership and existing runtime grants.
 EXECUTE replace(body,original,replacement);
END $migration$;
