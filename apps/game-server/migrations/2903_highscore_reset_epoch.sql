-- Legacy cuts have no proven reset epoch: computed_at_ms can postdate the
-- repeatable-read snapshot. Keep those cuts readable and let the scheduled job
-- publish any active epoch newer than the default.
ALTER TABLE game_world_highscore_heads ADD COLUMN published_reset_epoch NUMERIC(20,0)
    NOT NULL DEFAULT 0 CHECK (published_reset_epoch BETWEEN 0 AND 18446744073709551615);
