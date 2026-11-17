-- P1-a, owner approval 2026-11-13. Reuse the cursor transaction for the third stream.
ALTER TABLE game_platform_stream_cursors DROP CONSTRAINT game_platform_stream_cursors_stream_check;
ALTER TABLE game_platform_stream_cursors ADD CONSTRAINT game_platform_stream_cursors_stream_check
    CHECK (stream IN (1,2,3));
INSERT INTO game_platform_stream_cursors(stream) VALUES (3);
