-- Quest rewards use the active Forge policy inherited from earlier migrations.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM (VALUES ('oteryn:item.tibia.i7774'), ('oteryn:item.tibia.i25973')) reward(key)
        WHERE NOT EXISTS (
            SELECT 1 FROM game_forge_admitted_profile(reward.key, 'definition-r1')
            WHERE classification = 1 AND max_tier = 1 AND equipment_slot = 'hand'
        )
    ) THEN
        RAISE EXCEPTION 'Missing quest reward Forge profile';
    END IF;
END $$;
