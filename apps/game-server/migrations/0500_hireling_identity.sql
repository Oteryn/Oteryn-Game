CREATE TABLE game_hireling_identities (
    world_id uuid NOT NULL,
    owner_character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    hireling_id uuid NOT NULL DEFAULT game_character_uuid_v7(),
    outfit_id integer NOT NULL DEFAULT 1 CHECK (outfit_id > 0),
    purchased_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    PRIMARY KEY(world_id, owner_character_id),
    UNIQUE(hireling_id)
);

ALTER TABLE game_house_hirelings
    ADD COLUMN hireling_id uuid REFERENCES game_hireling_identities(hireling_id);

REVOKE ALL ON game_hireling_identities FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_hireling_identities TO oteryn_game_runtime;
GRANT SELECT ON game_hireling_identities TO oteryn_game_control;
