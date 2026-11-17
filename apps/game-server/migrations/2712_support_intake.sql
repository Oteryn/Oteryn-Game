-- T153/T154: anonymous displayed evidence only; content-derived receipt deduplicates retries.
CREATE TABLE game_support_intake (
    receipt bytea PRIMARY KEY CHECK (octet_length(receipt) = 32),
    kind integer NOT NULL CHECK (kind IN (6144, 6145)),
    preview text NOT NULL CHECK (octet_length(preview) BETWEEN 1 AND 4096)
);
REVOKE ALL ON game_support_intake FROM PUBLIC;
GRANT SELECT, INSERT ON game_support_intake TO oteryn_game_runtime;
GRANT SELECT ON game_support_intake TO oteryn_game_control;
