-- The publication head and its reset epoch advance in the same transaction.
ALTER TABLE game_world_highscore_heads ADD COLUMN published_reset_epoch NUMERIC(20,0)
    NOT NULL DEFAULT 0 CHECK (published_reset_epoch BETWEEN 0 AND 18446744073709551615);

-- A retained legacy cut from after activation already includes that epoch.
UPDATE game_world_highscore_heads h SET published_reset_epoch=a.reset_epoch
FROM game_world_reset_active a
JOIN game_world_reset_epochs e USING (world_id, reset_epoch),
game_world_highscore_snapshots s
WHERE h.world_id=a.world_id AND h.current_snapshot_id=s.snapshot_id
  AND s.computed_at_ms>=floor(extract(epoch FROM e.activated_at)*1000);
