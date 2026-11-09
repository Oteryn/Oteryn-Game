-- T026: explicit account/world consent, independent of character VIP grants.
CREATE TABLE game_account_social_profiles (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id), world_id UUID NOT NULL,
 main_character UUID REFERENCES game_character_roots(character_id) ON DELETE SET NULL,
 tokens SMALLINT NOT NULL DEFAULT 20 CHECK(tokens BETWEEN 0 AND 20),
 rate_at BIGINT NOT NULL DEFAULT floor(extract(epoch FROM clock_timestamp()))::bigint,
 PRIMARY KEY(account_id,world_id)
);
CREATE INDEX game_account_social_profiles_character ON game_account_social_profiles(main_character);
CREATE TABLE game_account_friends (
 id BIGSERIAL PRIMARY KEY, world_id UUID NOT NULL,
 left_account UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 right_account UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 requester UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 requester_name TEXT NOT NULL CHECK(octet_length(requester_name)<=100), target_name TEXT NOT NULL CHECK(octet_length(target_name)<=100),
 accepted BOOLEAN NOT NULL DEFAULT false,
 CHECK(left_account<right_account), CHECK(requester IN(left_account,right_account)),
 UNIQUE(left_account,right_account,world_id)
);
CREATE INDEX game_account_friends_right ON game_account_friends(right_account,world_id);
CREATE INDEX game_account_friends_requester ON game_account_friends(requester);
CREATE TABLE game_account_friend_handles (
 game_session_id UUID NOT NULL, character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 handle BIGINT NOT NULL CHECK(handle>0), relation_id BIGINT NOT NULL REFERENCES game_account_friends(id) ON DELETE CASCADE,
 PRIMARY KEY(game_session_id,handle), UNIQUE(game_session_id,relation_id)
);
CREATE INDEX game_account_friend_handles_character ON game_account_friend_handles(character_id);
CREATE INDEX game_account_friend_handles_relation ON game_account_friend_handles(relation_id);
CREATE TABLE game_account_friend_receipts (
 game_session_id UUID NOT NULL, command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id), request JSONB NOT NULL, result JSONB NOT NULL,
 PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_account_friend_receipts_character ON game_account_friend_receipts(character_id);
REVOKE ALL ON game_account_social_profiles,game_account_friends,game_account_friend_handles,game_account_friend_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_account_social_profiles TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_account_friends TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_account_friend_handles,game_account_friend_receipts TO oteryn_game_runtime;
REVOKE ALL ON SEQUENCE game_account_friends_id_seq FROM PUBLIC;
GRANT USAGE ON SEQUENCE game_account_friends_id_seq TO oteryn_game_runtime;
GRANT SELECT ON game_account_social_profiles,game_account_friends,game_account_friend_handles,game_account_friend_receipts TO oteryn_game_control;
CREATE TRIGGER game_account_friend_receipt_immutable BEFORE UPDATE OR DELETE ON game_account_friend_receipts FOR EACH ROW EXECUTE FUNCTION game_vip_immutable();

CREATE TABLE game_account_friend_handle_counters(game_session_id UUID PRIMARY KEY,next_handle BIGINT NOT NULL DEFAULT 1 CHECK(next_handle>0));
REVOKE ALL ON game_account_friend_handle_counters FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_account_friend_handle_counters TO oteryn_game_runtime;
GRANT SELECT ON game_account_friend_handle_counters TO oteryn_game_control;
