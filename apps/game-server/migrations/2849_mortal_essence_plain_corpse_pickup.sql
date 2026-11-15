-- Consolidate portable plain-body pickup with Mortal Essence's exact source proof.
-- Only i33793 and i33891 have CharacterInventory admission. Original Ground
-- bodies retain the complete lethal MINT/scope proof; a later loop drop retains
-- the existing authenticated loop custody checks and the same exact corpse MINT.
-- Every other corpse root remains denied by the single D134 guard.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
 body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original := $guard$IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION')
      AND NOT (NEW.to_kind IN(1,2) AND NEW.quantity_before=1 AND NEW.quantity_after=1
        AND NEW.definition_family='Item' AND NEW.definition_revision_ref='definition-r1'
        AND NEW.definition_production_key IN('oteryn:item.tibia.i33793','oteryn:item.tibia.i33891')
        AND EXISTS(SELECT 1 FROM game_item_mint_receipts m
          JOIN game_item_instances portable_body ON portable_body.item_instance_id=m.item_instance_id
          WHERE m.item_instance_id=NEW.item_instance_id
            AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
            AND m.materialized_at IS NOT NULL AND m.transaction_id=portable_body.minted_transaction_id
            AND m.loot_table_family=NEW.definition_family
            AND m.loot_table_production_key=NEW.definition_production_key
            AND m.loot_table_revision_ref=NEW.definition_revision_ref)) THEN$guard$;
 replacement := $guard$IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION')
      AND NOT COALESCE((NEW.to_kind IN(1,2) AND NEW.quantity_before=1 AND NEW.quantity_after=1
        AND NEW.definition_family='Item' AND NEW.definition_revision_ref='definition-r1'
        AND NEW.definition_production_key IN('oteryn:item.tibia.i33793','oteryn:item.tibia.i33891')
        AND EXISTS(SELECT 1 FROM game_item_mint_receipts m
          JOIN game_item_instances portable_body ON portable_body.item_instance_id=m.item_instance_id
          JOIN game_item_ground_locations ground_body ON ground_body.item_instance_id=portable_body.item_instance_id
          WHERE m.item_instance_id=NEW.item_instance_id
            AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
            AND m.materialized_at IS NOT NULL AND m.transaction_id=portable_body.minted_transaction_id
            AND portable_body.lifecycle=1 AND portable_body.quantity=1
            AND portable_body.world_id=m.death_world_id AND ground_body.world_id=portable_body.world_id
            AND m.loot_table_family=NEW.definition_family
            AND m.loot_table_production_key=NEW.definition_production_key
            AND m.loot_table_revision_ref=NEW.definition_revision_ref
            AND (game_item_plain_corpse_source(NEW.item_instance_id)
                 OR game_item_loop_drop_ref(ground_body.corpse_ref)))),false) THEN$guard$;
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'portable plain corpse pickup D134 predecessor changed';
 END IF;
 -- pg_get_functiondef retains SECURITY DEFINER and the pinned search_path;
 -- CREATE OR REPLACE retains ownership and existing runtime grants.
 EXECUTE replace(body,original,replacement);
END $migration$;
