-- Main 1107 owns lifetime 14/current-rotation 17; main 2727 also owns
-- Phosphorus 16. Preserve all three when upgrading a populated main database
-- past the former branch category-13 interpretation.
-- Public 13 remains main's inert historical tombstone; no data is deleted.
LOCK TABLE public.game_public_highscore_outbox, public.game_world_highscore_heads,
    public.game_world_highscore_snapshots, public.game_world_highscore_rows,
    public.game_public_highscores IN ACCESS EXCLUSIVE MODE;

ALTER TABLE public.game_world_highscore_snapshots
    DROP CONSTRAINT game_world_highscore_snapshots_category_check,
    ADD CONSTRAINT game_world_highscore_snapshots_category_check
        CHECK (category BETWEEN 1 AND 12 OR category IN (14, 16, 17));
ALTER TABLE public.game_world_highscore_heads
    DROP CONSTRAINT game_world_highscore_heads_category_check,
    ADD CONSTRAINT game_world_highscore_heads_category_check
        CHECK (category BETWEEN 1 AND 12 OR category IN (14, 16, 17));
ALTER TABLE public.game_public_highscores
    DROP CONSTRAINT game_public_highscores_category_check,
    ADD CONSTRAINT game_public_highscores_category_check
        CHECK (category BETWEEN 1 AND 14 OR category IN (16, 17));

-- The canonical job reads game_tibiadrome_scores, already granted by main.
REVOKE SELECT ON public.game_character_tibiadrome FROM oteryn_game_highscore_job;
