-- DROMECUBE. Absolute timestamps make the one-hour effect and individual 24-hour cooldown
-- authoritative across logout and restart. Item retirement remains owned by game_item_use_records.
-- Rollback: disable concoction routing in a successor; retain effects and immutable receipts.
CREATE TABLE game_character_concoction_effects (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 concoction SMALLINT NOT NULL CHECK(concoction BETWEEN 0 AND 19),
 active_until TIMESTAMPTZ NOT NULL,
 ready_at TIMESTAMPTZ NOT NULL,
 activated_transaction_id UUID NOT NULL REFERENCES game_item_use_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 CHECK(active_until <= ready_at),
 PRIMARY KEY(character_id, concoction)
);

CREATE TABLE game_character_concoction_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 concoction SMALLINT NOT NULL CHECK(concoction BETWEEN 0 AND 19),
 active_until TIMESTAMPTZ NOT NULL,
 ready_at TIMESTAMPTZ NOT NULL,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(active_until <= ready_at),
 UNIQUE(game_session_id, command_id)
);

CREATE TRIGGER game_character_concoction_receipts_immutable
 BEFORE UPDATE OR DELETE ON game_character_concoction_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_concoction_receipts_no_truncate
 BEFORE TRUNCATE ON game_character_concoction_receipts
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_concoction_receipts_stamp
 BEFORE INSERT ON game_character_concoction_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

REVOKE ALL ON game_character_concoction_effects, game_character_concoction_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_concoction_effects TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_concoction_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_concoction_effects, game_character_concoction_receipts TO oteryn_game_control;
