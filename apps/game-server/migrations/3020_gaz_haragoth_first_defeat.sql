-- An acknowledged hill-raid death permanently unlocks the prison raid in this World.
CREATE TABLE game_world_gaz_haragoth_first_defeats (
    world_id UUID PRIMARY KEY,
    firing_id UUID NOT NULL REFERENCES game_world_raid_firings(firing_id)
);

CREATE INDEX game_world_gaz_haragoth_first_defeats_firing
    ON game_world_gaz_haragoth_first_defeats (firing_id);

REVOKE ALL ON game_world_gaz_haragoth_first_defeats FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_gaz_haragoth_first_defeats TO oteryn_game_runtime;
GRANT SELECT ON game_world_gaz_haragoth_first_defeats TO oteryn_game_control;
