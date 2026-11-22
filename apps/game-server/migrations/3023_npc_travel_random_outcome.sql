-- The selected cell survives consumption of the existing pending arrival.
ALTER TABLE game_newhaven_travel_receipts
 ADD COLUMN random_draw SMALLINT,
 ADD COLUMN random_destination INTEGER[],
 ADD CONSTRAINT game_npc_travel_random_outcome_shape CHECK (
   (random_draw IS NULL AND random_destination IS NULL)
   OR (request_binding IS NOT NULL
       AND random_draw IS NOT NULL AND random_draw BETWEEN 1 AND 10
       AND random_destination IS NOT NULL
       AND array_ndims(random_destination) = 1 AND array_lower(random_destination, 1) = 1
       AND cardinality(random_destination) = 3
       AND random_destination[1] IS NOT NULL AND random_destination[1] BETWEEN 0 AND 65535
       AND random_destination[2] IS NOT NULL AND random_destination[2] BETWEEN 0 AND 65535
       AND random_destination[3] IS NOT NULL AND random_destination[3] BETWEEN 0 AND 15));

REVOKE ALL ON game_newhaven_travel_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE(arrived_at) ON game_newhaven_travel_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_newhaven_travel_receipts TO oteryn_game_control;

-- Commands expose their ordinal and session ID on the wire. Keep the draw root server-side.
CREATE TABLE game_receipt_random_root (
 singleton BOOLEAN PRIMARY KEY CHECK (singleton),
 secret BYTEA NOT NULL CHECK (octet_length(secret) = 32)
);
INSERT INTO game_receipt_random_root
 VALUES (TRUE, sha256(uuid_send(gen_random_uuid()) || uuid_send(gen_random_uuid())));
CREATE TRIGGER game_receipt_random_root_immutable BEFORE UPDATE OR DELETE
 ON game_receipt_random_root FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_receipt_random_root_no_truncate BEFORE TRUNCATE
 ON game_receipt_random_root EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_receipt_random_root FROM PUBLIC;
GRANT SELECT ON game_receipt_random_root TO oteryn_game_runtime;
GRANT SELECT ON game_receipt_random_root TO oteryn_game_control;
