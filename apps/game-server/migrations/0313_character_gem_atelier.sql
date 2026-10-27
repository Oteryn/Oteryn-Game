-- PROG-ADV-1: durable Gem Atelier / Fragment Workshop character state.
CREATE TABLE game_character_atelier_state (
    character_id UUID PRIMARY KEY,
    atelier_revision BIGINT NOT NULL CHECK (atelier_revision >= 0),
    gold BIGINT NOT NULL CHECK (gold >= 0),
    fragments_basic BIGINT NOT NULL CHECK (fragments_basic >= 0),
    fragments_supreme BIGINT NOT NULL CHECK (fragments_supreme >= 0),
    state_json JSONB NOT NULL,
    CONSTRAINT game_character_atelier_state_character_fk
        FOREIGN KEY (character_id) REFERENCES game_character_roots(character_id) ON DELETE CASCADE
);

CREATE TABLE game_character_atelier_receipts (
    character_id UUID NOT NULL,
    occurrence UUID NOT NULL,
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256) = 32),
    before_revision BIGINT NOT NULL,
    after_revision BIGINT NOT NULL CHECK (after_revision = before_revision + 1),
    outcome_json JSONB NOT NULL,
    PRIMARY KEY (character_id, occurrence),
    CONSTRAINT game_character_atelier_receipts_character_fk
        FOREIGN KEY (character_id) REFERENCES game_character_roots(character_id) ON DELETE CASCADE
);
