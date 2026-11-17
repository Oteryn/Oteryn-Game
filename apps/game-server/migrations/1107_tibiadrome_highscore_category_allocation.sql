-- HIGHSCORES0 §3 reserves13 Loyalty,14 Drome lifetime,15 Taint,16 Phosphorus.
-- docs/reference/tibia-manual/combat.md:252,262 requires a separate current
-- rotation category;17 is its next-free allocation.1105's erroneous13/14
-- snapshots are derived caches, not gameplay scores or Character authority.
-- Retire their tokens without rewriting immutable headers/rows or resetting
-- public14's revision floor.13 public metadata remains an inert, unserved
-- tombstone: the Game public encoder never exported13, and no new13 body can
-- pass the snapshot/head bounds or the corrected source category admission.
LOCK TABLE game_public_highscore_outbox, game_world_highscore_heads,
 game_world_highscore_snapshots, game_world_highscore_rows,
 game_public_highscores IN ACCESS EXCLUSIVE MODE;

-- Foreign-key ordering: withdraw queued old interpretations, then heads, then
-- headers.0129's normal header cascade is the only permitted row deletion.
DELETE FROM game_public_highscore_outbox WHERE category IN(13,14);
DELETE FROM game_world_highscore_heads WHERE category IN(13,14);
DELETE FROM game_world_highscore_snapshots WHERE category IN(13,14);

ALTER TABLE game_world_highscore_snapshots
 DROP CONSTRAINT game_world_highscore_snapshots_category_check,
 ADD CONSTRAINT game_world_highscore_snapshots_category_check
 CHECK(category BETWEEN 1 AND 12 OR category IN(14,17));
ALTER TABLE game_world_highscore_heads
 DROP CONSTRAINT game_world_highscore_heads_category_check,
 ADD CONSTRAINT game_world_highscore_heads_category_check
 CHECK(category BETWEEN 1 AND 12 OR category IN(14,17));
ALTER TABLE game_public_highscores
 DROP CONSTRAINT game_public_highscores_category_check,
 ADD CONSTRAINT game_public_highscores_category_check
 CHECK(category BETWEEN 1 AND 14 OR category=17);
-- No public revision/epoch or immutable guard changes. The next genuine14/17
-- source publication rebuilds bodies from unchanged game_tibiadrome_scores.

--0977 wraps0091's complete resync. Preserve that wrapper and every unrelated
-- branch; exclude only the retired13 metadata tombstone from both source arms.
-- Otherwise a later resync would recreate an unsupported queue head and stall
-- the ordinary publisher (public_projections.rs:HIGHSCORES_HEAD).
DO $retired_category_resync$
DECLARE body TEXT; metadata_anchor TEXT := $metadata$SELECT world_id, category FROM game_public_highscores';$metadata$;
 snapshot_anchor TEXT := $snapshots$ FROM game_world_highscore_snapshots';$snapshots$;
BEGIN
 SELECT pg_get_functiondef('game_public_projection_resync_before_world_totals(text,boolean)'::regprocedure) INTO STRICT body;
 IF (length(body)-length(replace(body,metadata_anchor,'')))/length(metadata_anchor)<>1
  OR (length(body)-length(replace(body,snapshot_anchor,'')))/length(snapshot_anchor)<>1 THEN
  RAISE EXCEPTION '1107 retired-category resync anchors changed';
 END IF;
 body:=replace(body,metadata_anchor,$metadata$SELECT world_id, category FROM game_public_highscores WHERE category <> 13';$metadata$);
 body:=replace(body,snapshot_anchor,$snapshots$ FROM game_world_highscore_snapshots WHERE category <> 13';$snapshots$);
 EXECUTE body;
END $retired_category_resync$;
