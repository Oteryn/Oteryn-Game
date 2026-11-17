-- Rathleton Quest election state (TibiaWiki Rathleton_Quest/Spoiler oldid 1141625).
-- One ballot per confirmed Marvin cast command; the World tally per fortnight period
-- and option is counted in the same NPC transaction. Winner application is separate.
CREATE TABLE game_rathleton_election_ballots (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 world_id UUID NOT NULL,
 period INTEGER NOT NULL CHECK(period>=0),
 option_code SMALLINT NOT NULL CHECK(option_code IN(11,12,13,21,22,23,31,32,33,41,42,43)),
 cast_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 PRIMARY KEY(character_id,game_session_id,command_id)
);
CREATE TABLE game_rathleton_election_tallies (
 world_id UUID NOT NULL,
 period INTEGER NOT NULL CHECK(period>=0),
 option_code SMALLINT NOT NULL CHECK(option_code IN(11,12,13,21,22,23,31,32,33,41,42,43)),
 votes BIGINT NOT NULL CHECK(votes>0),
 PRIMARY KEY(world_id,period,option_code)
);
REVOKE ALL ON game_rathleton_election_ballots, game_rathleton_election_tallies FROM PUBLIC;
GRANT SELECT, INSERT ON game_rathleton_election_ballots TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_rathleton_election_tallies TO oteryn_game_runtime;
GRANT SELECT ON game_rathleton_election_ballots, game_rathleton_election_tallies TO oteryn_game_control;
