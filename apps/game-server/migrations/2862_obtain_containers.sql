-- Obtain is independent of loot settings and uses the same five categories; zero is default.
CREATE TABLE game_character_obtain_containers (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 category SMALLINT NOT NULL CHECK(category BETWEEN 0 AND 5),
 container_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 PRIMARY KEY(character_id,category)
);
CREATE INDEX game_character_obtain_containers_container ON game_character_obtain_containers(container_id);
CREATE TABLE game_obtain_container_receipts (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 request BYTEA NOT NULL CHECK(octet_length(request)<=32),
 result BYTEA NOT NULL CHECK(octet_length(result)<=32768),
 PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_obtain_container_receipts_character ON game_obtain_container_receipts(character_id);
CREATE TRIGGER game_obtain_container_receipts_immutable BEFORE UPDATE OR DELETE ON game_obtain_container_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_obtain_container_receipts_no_truncate BEFORE TRUNCATE ON game_obtain_container_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_character_obtain_containers,game_obtain_container_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_character_obtain_containers TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_obtain_container_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_obtain_containers,game_obtain_container_receipts TO oteryn_game_control;
