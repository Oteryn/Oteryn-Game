-- BOSS-LEVER-1 staged storage only. No production activity handoff/InstanceRuntime or
-- return/recovery owner exists yet. These rows confer no admission, death or reward authority.
-- Keep runtime access read-only until the complete fenced writer is installed and qualified.
CREATE TABLE game_boss_room_catalogue (
    boss_key TEXT PRIMARY KEY CHECK (boss_key ~ '^[A-Za-z0-9_./-]+:[A-Za-z0-9_.:/-]+$' AND octet_length(boss_key) <= 512),
    content_revision TEXT NOT NULL,
    boss_creature_key TEXT NOT NULL,
    minimum_players SMALLINT NOT NULL CHECK (minimum_players BETWEEN 1 AND 15),
    maximum_players SMALLINT NOT NULL CHECK (maximum_players BETWEEN minimum_players AND 15),
    minimum_level BIGINT NOT NULL CHECK (minimum_level BETWEEN 1 AND 4294967295),
    cooldown_seconds BIGINT NOT NULL CHECK (cooldown_seconds BETWEEN 1 AND 2592000),
    time_limit_seconds BIGINT NOT NULL CHECK (time_limit_seconds BETWEEN 1 AND 86400),
    lever_positions JSONB NOT NULL CHECK (jsonb_typeof(lever_positions)='array'),
    room_template_key TEXT NOT NULL,
    reward_key TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE game_boss_room_activities (
    admission UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    instance_id UUID NOT NULL UNIQUE,
    boss_key TEXT NOT NULL REFERENCES game_boss_room_catalogue(boss_key),
    content_revision TEXT NOT NULL,
    origin_channel_id UUID NOT NULL,
    state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 4), -- live, defeated, timed-out, retired
    opened_at BIGINT NOT NULL CHECK (opened_at >= 0),
    deadline BIGINT NOT NULL CHECK (deadline > opened_at),
    terminal_at BIGINT,
    boss_generation BIGINT NOT NULL CHECK (boss_generation = 1),
    CHECK ((state=1 AND terminal_at IS NULL) OR (state<>1 AND terminal_at IS NOT NULL))
);

CREATE TABLE game_boss_room_participants (
    admission UUID NOT NULL REFERENCES game_boss_room_activities(admission),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    origin_game_session_id UUID NOT NULL,
    origin_lease_generation NUMERIC(20,0) NOT NULL
      CHECK (origin_lease_generation BETWEEN 1 AND 18446744073709551615),
    qualified_reward BOOLEAN,
    returned_at BIGINT,
    PRIMARY KEY (admission, character_id)
);

CREATE TABLE game_boss_room_reward_receipts (
    admission UUID NOT NULL,
    character_id UUID NOT NULL,
    reward_key TEXT NOT NULL,
    granted_at BIGINT NOT NULL CHECK (granted_at >= 0),
    PRIMARY KEY (admission, character_id),
    FOREIGN KEY (admission, character_id)
      REFERENCES game_boss_room_participants(admission, character_id),
    CHECK (reward_key <> '')
);

REVOKE ALL ON game_boss_room_catalogue, game_boss_room_activities,
  game_boss_room_participants, game_boss_room_reward_receipts FROM PUBLIC;
GRANT SELECT ON game_boss_room_catalogue TO oteryn_game_runtime;
GRANT SELECT ON game_boss_room_activities TO oteryn_game_runtime;
GRANT SELECT ON game_boss_room_participants TO oteryn_game_runtime;
GRANT SELECT ON game_boss_room_reward_receipts TO oteryn_game_runtime;
