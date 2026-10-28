-- NEWS-HIST-AUTO-SPELLS-1 (news 8675): durable automatically acquired spells.
CREATE TABLE game_character_known_spells (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    spell_key text NOT NULL CHECK (
        spell_key <> ''
        AND spell_key = btrim(spell_key)
    ),
    learned_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    PRIMARY KEY (character_id, spell_key)
);

-- Runtime reconciliation only appends identities; existing grants remain immutable.
REVOKE ALL ON game_character_known_spells FROM PUBLIC, oteryn_game_control;
GRANT SELECT, INSERT ON game_character_known_spells TO oteryn_game_runtime;
