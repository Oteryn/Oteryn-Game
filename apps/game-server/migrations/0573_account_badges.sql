-- PAR-P12b: Cyclopedia badges. Global (TibiaWiki "Social", Badges section) awards badges to the
-- account and keeps them permanently, so a row is only ever inserted: no update, no delete.
-- The catalogue ids are `Badge` in `crates/protocol-oteryn/src/cyclopedia_character.rs`
-- (Crystal `game.cpp:293-316` ids 1-14 kept; Global-only grades appended as 22-27; the
-- sunset tournament badges 15-21 are not admitted).
CREATE TABLE game_account_badges (
    account_id UUID NOT NULL REFERENCES game_character_account_guards (account_id),
    badge SMALLINT NOT NULL CHECK (badge BETWEEN 1 AND 14 OR badge BETWEEN 22 AND 27),
    earned_at_unix_ms BIGINT NOT NULL CHECK (earned_at_unix_ms >= 0),
    PRIMARY KEY (account_id, badge)
);

REVOKE ALL ON game_account_badges FROM PUBLIC;
GRANT SELECT, INSERT ON game_account_badges TO oteryn_game_runtime;
GRANT SELECT ON game_account_badges TO oteryn_game_control;
