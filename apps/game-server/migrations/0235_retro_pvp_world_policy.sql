-- SPAWNGLOBAL: the control-owned World policy accepts Retro PvP as value 4.
ALTER TABLE game_spell_field_world_policies
    DROP CONSTRAINT game_spell_field_world_policies_world_type_check;
ALTER TABLE game_spell_field_world_policies
    ADD CONSTRAINT game_spell_field_world_policies_world_type_check
    CHECK (world_type IN (1, 2, 3, 4));
