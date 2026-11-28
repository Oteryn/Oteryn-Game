-- Keep the victim's respawn rule with the immutable death. An expired skull
-- or a later reconnect cannot change the outcome. Existing receipts use full vitals.
ALTER TABLE game_character_death_receipts
    ADD COLUMN black_skull BOOLEAN NOT NULL DEFAULT FALSE;
