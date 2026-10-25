-- PVP-RT-1: immutable per-swing/effect receipts and current aggression.
-- Character/World state and death consequences remain owned by 0081/0142.
CREATE TABLE game_character_pvp_harm (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 65535),
 victim UUID NOT NULL REFERENCES game_character_roots(character_id),
 attacker UUID NOT NULL REFERENCES game_character_roots(character_id),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 damage INTEGER NOT NULL CHECK(damage>=0),
 unjustified BOOLEAN NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(game_session_id,command_id,ordinal,victim), CHECK(victim<>attacker)
);
CREATE INDEX game_character_pvp_harm_victim ON game_character_pvp_harm(victim,attacker,created_at DESC);
CREATE TABLE game_character_pvp_aggression (
 attacker UUID NOT NULL REFERENCES game_character_roots(character_id),
 target UUID NOT NULL REFERENCES game_character_roots(character_id),
 until_at TIMESTAMPTZ NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
 attacker_placement BYTEA NOT NULL CHECK(octet_length(attacker_placement)=16),
 target_placement BYTEA NOT NULL CHECK(octet_length(target_placement)=16),
 PRIMARY KEY(attacker,target), CHECK(attacker<>target)
);
CREATE INDEX game_character_pvp_aggression_expiry ON game_character_pvp_aggression(until_at);
CREATE TRIGGER game_character_pvp_harm_immutable BEFORE UPDATE OR DELETE ON game_character_pvp_harm
 FOR EACH ROW EXECUTE FUNCTION game_world_party_receipt_immutable();
REVOKE ALL ON game_character_pvp_harm,game_character_pvp_aggression FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_pvp_harm TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_character_pvp_aggression TO oteryn_game_runtime;
GRANT SELECT ON game_character_pvp_harm,game_character_pvp_aggression TO oteryn_game_control;
