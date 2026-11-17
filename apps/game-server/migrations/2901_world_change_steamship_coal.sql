-- Keep coal burned in Dronk's pumps separate from coal donated to Junkar.
ALTER TABLE game_world_change_tallies
    DROP CONSTRAINT game_world_change_tallies_resource_check,
    ADD CHECK (resource IN ('coal', 'medicine', 'steamship_coal', 'votes'));

ALTER TABLE game_world_change_contributions
    DROP CONSTRAINT game_world_change_contributions_resource_check,
    ADD CHECK (resource IN ('coal', 'medicine', 'steamship_coal', 'votes'));

REVOKE ALL ON game_world_change_tallies, game_world_change_contributions FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_world_change_tallies TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_world_change_contributions TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_tallies, game_world_change_contributions TO oteryn_game_control;

-- Allow these exact, fenced coal debit recipes to produce no items.
DO $$
DECLARE previous text;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT previous
    FROM pg_constraint
    WHERE conrelid='game_item_craft_records'::regclass
      AND conname='game_item_craft_records_outputs_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE format(
        'ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (consume_source AND recipe IN (''oteryn:world-change.awash-coal'', ''oteryn:world-change.steamship-coal'') AND outputs = ''[]''::jsonb AND before_item->>''definition_production_key'' = ''oteryn:item.tibia.i12600''))',
        previous
    );
END $$;
