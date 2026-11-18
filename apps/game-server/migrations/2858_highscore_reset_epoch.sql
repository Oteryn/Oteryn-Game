-- The publication head and its reset epoch advance in the same transaction.
ALTER TABLE game_world_highscore_heads ADD COLUMN published_reset_epoch NUMERIC(20,0)
    NOT NULL DEFAULT 0 CHECK (published_reset_epoch BETWEEN 0 AND 18446744073709551615);
