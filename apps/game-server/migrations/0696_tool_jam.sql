-- M08: the squeezing gear of girlpower jams for a minute on a 1% roll. Crystal
-- data/scripts/actions/tools/squeezing_gear_of_girlpower.lua:16-17,34-40 and items.xml:25842-25847
-- (jammed form 9597, duration 60, decayTo 9596). The jam is a durable absolute deadline on the
-- tool instance; the database clock starts it, so a restart or reconnect cannot shorten it.
CREATE TABLE game_tool_jams (
    tool_item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    character_id UUID NOT NULL,
    jammed_until_unix_ms BIGINT NOT NULL CHECK (jammed_until_unix_ms > 0)
);

CREATE TABLE game_tool_jam_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL,
    tool_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    intent BYTEA NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 529),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (game_session_id, command_id)
);
CREATE INDEX game_tool_jam_receipts_tool ON game_tool_jam_receipts (tool_item_instance_id);

REVOKE ALL ON game_tool_jams FROM PUBLIC;
REVOKE ALL ON game_tool_jam_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_tool_jams TO oteryn_game_runtime;
GRANT SELECT ON game_tool_jams TO oteryn_game_control;
GRANT SELECT, INSERT ON game_tool_jam_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_tool_jam_receipts TO oteryn_game_control;
