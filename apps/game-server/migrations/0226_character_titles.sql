-- CHARINFO: durable earned-title facts and the selected display title.
-- Eligibility is re-evaluated from native owners; this table stores no copied predicate facts.
CREATE TABLE game_character_titles (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    title SMALLINT NOT NULL CHECK (title BETWEEN 1 AND 4),
    earned_at_unix_ms BIGINT NOT NULL CHECK (earned_at_unix_ms >= 0),
    expires_at_unix_ms BIGINT CHECK (
        expires_at_unix_ms IS NULL OR expires_at_unix_ms > earned_at_unix_ms
    ),
    PRIMARY KEY (character_id, title)
);

CREATE TABLE game_character_title_selection (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    title SMALLINT,
    revision BIGINT NOT NULL CHECK (revision > 0),
    CHECK (title IS NULL OR title BETWEEN 1 AND 4),
    FOREIGN KEY (character_id, title)
        REFERENCES game_character_titles(character_id, title)
);

CREATE INDEX game_character_titles_expiry
    ON game_character_titles(expires_at_unix_ms, character_id)
    WHERE expires_at_unix_ms IS NOT NULL;

REVOKE ALL ON game_character_titles, game_character_title_selection FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_titles TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_title_selection TO oteryn_game_runtime;
GRANT SELECT ON game_character_titles, game_character_title_selection TO oteryn_game_control;
