-- The native plain bodies i33793 and i33891 are pickupable Items, with
-- CharacterInventory admission in spell-native-profiles.json. The other four
-- plain bodies and every corpse container remain outside carried custody.
-- Preserve the installed exact-item, receipt, fence and Ground-source checks,
-- including loop-drop provenance when a portable body is picked up again.
DO $migration$
DECLARE body TEXT; original TEXT := $old$
   IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
      WHERE item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION') THEN
$old$;
BEGIN
    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed corpse carried-custody guard';
    END IF;
    EXECUTE replace(body,original,$new$
   IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS(SELECT 1 FROM game_item_mint_receipts
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
            AND m.loot_table_revision_ref=NEW.definition_revision_ref)) THEN
$new$);
END $migration$;
