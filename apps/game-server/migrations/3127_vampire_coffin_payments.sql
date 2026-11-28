-- The garlic payment is recoverable until the fenced owner delivers its encounter.
CREATE TABLE game_pending_vampire_coffin_payments (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id)
);

REVOKE ALL ON game_pending_vampire_coffin_payments FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_pending_vampire_coffin_payments TO oteryn_game_runtime;
