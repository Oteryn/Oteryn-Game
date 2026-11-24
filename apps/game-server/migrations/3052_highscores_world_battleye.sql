-- BattlEye is display metadata of the same immutable, control-published
-- World/revision policy that supplies the catalogue's PvP type.
ALTER TABLE game_spell_field_world_policies
    ADD COLUMN battleye_status SMALLINT CHECK (battleye_status IN (1, 2, 3));
