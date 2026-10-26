-- GUILD-WAR-1: World-scoped pending/active/ended wars and immutable command receipts.
CREATE TABLE game_guild_wars (
 war_id UUID PRIMARY KEY,
 world_id UUID NOT NULL,
 declaring_guild UUID NOT NULL REFERENCES game_guilds,
 opposing_guild UUID NOT NULL REFERENCES game_guilds,
 state SMALLINT NOT NULL CHECK(state BETWEEN 1 AND 3),
 frag_limit INTEGER NOT NULL CHECK(frag_limit BETWEEN 10 AND 1000),
 duration_days INTEGER NOT NULL CHECK(duration_days BETWEEN 7 AND 180),
 declared_at BIGINT NOT NULL,
 started_at BIGINT,
 ends_at BIGINT,
 declaring_frags INTEGER NOT NULL DEFAULT 0 CHECK(declaring_frags>=0),
 opposing_frags INTEGER NOT NULL DEFAULT 0 CHECK(opposing_frags>=0),
 ended_reason SMALLINT CHECK(ended_reason IN(1,2)),
 CHECK(declaring_guild<>opposing_guild),
 CHECK((state=1 AND started_at IS NULL AND ends_at IS NULL) OR (state=2 AND started_at IS NOT NULL AND ends_at IS NOT NULL) OR state=3)
);
CREATE UNIQUE INDEX game_guild_wars_live_pair ON game_guild_wars(world_id,LEAST(declaring_guild,opposing_guild),GREATEST(declaring_guild,opposing_guild)) WHERE state<3;
CREATE TABLE game_guild_war_commands (
 game_session_id UUID NOT NULL, command_id BIGINT NOT NULL, wire_sha256 BYTEA NOT NULL CHECK(octet_length(wire_sha256)=32),
 war_id UUID NOT NULL REFERENCES game_guild_wars, result SMALLINT NOT NULL, PRIMARY KEY(game_session_id,command_id)
);
CREATE FUNCTION game_guild_war_rows_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'guild war receipts are immutable'; END $$;
CREATE TRIGGER game_guild_war_commands_immutable BEFORE UPDATE OR DELETE ON game_guild_war_commands FOR EACH ROW EXECUTE FUNCTION game_guild_war_rows_immutable();
GRANT SELECT,INSERT,UPDATE ON game_guild_wars TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_guild_war_commands TO oteryn_game_runtime;
GRANT SELECT ON game_guild_wars,game_guild_war_commands TO oteryn_game_control;
