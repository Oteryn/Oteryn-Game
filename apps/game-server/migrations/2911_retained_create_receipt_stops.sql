-- Terminal publication dispositions are append-only and leave receipt bytes immutable.
CREATE TABLE game_character_retained_create_receipt_stops (
    operation_id UUID PRIMARY KEY REFERENCES game_character_command_receipts (operation_id)
);

REVOKE ALL ON game_character_retained_create_receipt_stops FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_retained_create_receipt_stops TO oteryn_game_runtime;
GRANT SELECT ON game_character_retained_create_receipt_stops TO oteryn_game_control;
