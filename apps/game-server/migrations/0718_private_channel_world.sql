-- S04b: session-owned private channels across one World. Chat text never enters storage.
-- Crystal 201189f chat.cpp:458-472 reserves channel numbers 100..9999. A never-reused
-- epoch distinguishes later reuse of a number from an earlier encrypted relay line.
CREATE TABLE game_private_chat_participants (
    game_session_id UUID PRIMARY KEY REFERENCES game_durability_reconnect_sessions(game_session_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    enabled BOOLEAN NOT NULL,
    UNIQUE (world_id, game_session_id)
);
CREATE INDEX game_private_chat_participants_character ON game_private_chat_participants(character_id);
CREATE TABLE game_private_chat_channels (
    world_id UUID NOT NULL,
    channel_id INTEGER NOT NULL CHECK (channel_id BETWEEN 100 AND 9999),
    epoch BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE CHECK (epoch > 0),
    owner_session_id UUID NOT NULL,
    PRIMARY KEY (world_id, channel_id),
    UNIQUE (world_id, owner_session_id),
    FOREIGN KEY (world_id, owner_session_id) REFERENCES game_private_chat_participants(world_id, game_session_id)
);
CREATE INDEX game_private_chat_channels_owner ON game_private_chat_channels(owner_session_id);
CREATE TABLE game_private_chat_members (
    world_id UUID NOT NULL,
    channel_id INTEGER NOT NULL,
    game_session_id UUID NOT NULL,
    joined BOOLEAN NOT NULL DEFAULT FALSE,
    joined_at_micros BIGINT NOT NULL DEFAULT 0 CHECK (joined_at_micros >= 0),
    PRIMARY KEY (world_id, channel_id, game_session_id),
    FOREIGN KEY (world_id, channel_id) REFERENCES game_private_chat_channels(world_id, channel_id) ON DELETE CASCADE,
    FOREIGN KEY (world_id, game_session_id) REFERENCES game_private_chat_participants(world_id, game_session_id)
);
CREATE INDEX game_private_chat_members_session ON game_private_chat_members(game_session_id, world_id, channel_id);
CREATE TABLE game_private_chat_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    binding BYTEA NOT NULL CHECK (octet_length(binding) = 32),
    disposition SMALLINT NOT NULL CHECK (disposition BETWEEN 1 AND 11),
    wait_seconds INTEGER NOT NULL DEFAULT 0 CHECK (wait_seconds BETWEEN 0 AND 1280),
    CHECK ((disposition IN (2, 3)) = (wait_seconds > 0)),
    PRIMARY KEY (game_session_id, command_id)
);
REVOKE ALL ON game_private_chat_participants, game_private_chat_channels,
    game_private_chat_members, game_private_chat_receipts FROM PUBLIC;
REVOKE ALL ON SEQUENCE game_private_chat_channels_epoch_seq FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_private_chat_participants,
    game_private_chat_channels, game_private_chat_members TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_private_chat_receipts TO oteryn_game_runtime;
GRANT USAGE ON SEQUENCE game_private_chat_channels_epoch_seq TO oteryn_game_runtime;
