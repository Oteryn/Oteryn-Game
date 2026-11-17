-- T027: visibility preferences never update or revoke permanent earned badges.
CREATE TABLE game_account_badge_visibility (
    account_id UUID NOT NULL,
    badge SMALLINT NOT NULL,
    is_public BOOLEAN NOT NULL,
    PRIMARY KEY (account_id, badge),
    FOREIGN KEY (account_id, badge) REFERENCES game_account_badges (account_id, badge)
);
CREATE TABLE game_account_badge_visibility_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id > 0),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    request BYTEA NOT NULL CHECK (octet_length(request) = 4),
    result BYTEA NOT NULL CHECK (octet_length(result) BETWEEN 3 AND 43),
    PRIMARY KEY (game_session_id, command_id)
);
CREATE INDEX game_account_badge_visibility_receipts_character ON game_account_badge_visibility_receipts(character_id);
REVOKE ALL ON game_account_badge_visibility, game_account_badge_visibility_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_account_badge_visibility TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_account_badge_visibility_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_account_badge_visibility, game_account_badge_visibility_receipts TO oteryn_game_control;
CREATE FUNCTION game_badge_visibility_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'badge visibility receipt is immutable' USING ERRCODE='23514'; END;
$$;
REVOKE ALL ON FUNCTION game_badge_visibility_receipt_immutable() FROM PUBLIC;
CREATE TRIGGER game_badge_visibility_receipt_immutable BEFORE UPDATE OR DELETE ON game_account_badge_visibility_receipts
FOR EACH ROW EXECUTE FUNCTION game_badge_visibility_receipt_immutable();
