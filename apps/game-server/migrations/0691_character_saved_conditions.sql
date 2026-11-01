-- C08: the Character's persistent conditions across logout and a fresh login (Crystal 201189f8
-- `src/io/functions/iologindata_save_player.cpp:264` saves every condition whose
-- `Condition::isPersistent()` holds, `iologindata_load_player.cpp:287` loads them and
-- `player.cpp:11565` re-adds them at login).
--
-- Scope of this migration only:
--   * `conditions`: the saved conditions of the last-position row, a JSON array of at most 64
--     entries in at most 16 KiB. It rides in the same row and the same ordered upsert as the
--     cell and the vitals (migrations 0080 and 0084), so it shares their order key and session
--     fence. Only the final logout write stores it; every other write of the row (a periodic
--     write, a respawn placement, a bed or travel placement) stores NULL, so a row whose key is
--     newer than the logout capture never carries conditions of an earlier capture;
--   * NULL: nothing to restore (a row written before this migration, or by any non-final write);
--   * grants: unchanged in substance and restated here: the runtime role may SELECT, INSERT and
--     UPDATE, never DELETE; the control role may SELECT.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- column and its constraint. Only the fresh-login restore reads it.

ALTER TABLE game_character_last_positions
    ADD COLUMN conditions JSONB NULL,
    ADD CONSTRAINT game_character_last_positions_conditions CHECK (
        conditions IS NULL
        OR (
            jsonb_typeof(conditions) = 'array'
            AND jsonb_array_length(conditions) BETWEEN 1 AND 64
            AND octet_length(conditions::text) <= 16384
        )
    );

REVOKE ALL ON game_character_last_positions FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_last_positions TO oteryn_game_runtime;
GRANT SELECT ON game_character_last_positions TO oteryn_game_control;
