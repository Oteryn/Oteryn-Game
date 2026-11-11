-- N009/N014 (PLATFORM_SANCTION_LOYALTY_EXPORT_V1, owner decision 2026-11-11): Game pulls
-- Platform's sanction and loyalty streams over the private /internal/v1 mTLS route and projects
-- them itself. Platform never calls Game SQL (OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT: no
-- direct or shared SQL), so the 2800 projection moves from the control role to the Game runtime
-- puller, and each stream keeps one durable cursor.
CREATE TABLE game_platform_stream_cursors (
    -- 1 sanctions (support/sanctions/stream/read), 2 loyalty (loyalty-snapshots/stream).
    stream SMALLINT PRIMARY KEY CHECK (stream IN (1, 2)),
    -- Platform's opaque resume cursor; NULL before the first page.
    stream_cursor TEXT
        CHECK (stream_cursor IS NULL
               OR (octet_length(stream_cursor) BETWEEN 1 AND 256 AND stream_cursor ~ '^[!-~]+$')),
    pages BIGINT NOT NULL DEFAULT 0 CHECK (pages >= 0),
    advanced_at TIMESTAMPTZ
);
INSERT INTO game_platform_stream_cursors(stream) VALUES (1), (2);

REVOKE ALL ON game_platform_stream_cursors FROM PUBLIC;
GRANT SELECT, UPDATE ON game_platform_stream_cursors TO oteryn_game_runtime;
GRANT SELECT ON game_platform_stream_cursors TO oteryn_game_control;

REVOKE ALL ON FUNCTION game_account_sanction_project(UUID, BIGINT, UUID, UUID, SMALLINT, INTEGER,
    TEXT, BIGINT, BIGINT, BOOLEAN) FROM oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_account_sanction_project(UUID, BIGINT, UUID, UUID, SMALLINT,
    INTEGER, TEXT, BIGINT, BIGINT, BOOLEAN) TO oteryn_game_runtime;
