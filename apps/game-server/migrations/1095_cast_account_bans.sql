-- SRV11: Game-owned caster-account bans. Platform account identifiers are
-- opaque authenticated UUID bytes; Game does not own their account lifecycle.
CREATE TABLE game_cast_account_bans (
    owner_account BYTEA NOT NULL CHECK (octet_length(owner_account) = 16 AND owner_account <> decode(repeat('00',16),'hex')),
    subject_account BYTEA NOT NULL CHECK (octet_length(subject_account) = 16 AND subject_account <> decode(repeat('00',16),'hex')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (owner_account, subject_account),
    CHECK (owner_account <> subject_account)
);
-- No foreign keys to Platform or character rows. The composite primary key
-- bounds owner-scoped counts and answers admission/renewal EXISTS queries.
REVOKE ALL ON game_cast_account_bans FROM PUBLIC;
GRANT SELECT, INSERT ON game_cast_account_bans TO oteryn_game_runtime;
GRANT SELECT ON game_cast_account_bans TO oteryn_game_control;
