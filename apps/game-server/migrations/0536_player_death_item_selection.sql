-- C20b / DEATH-COMPLETE0 §4.2: immutable selection precedes one-item operations.
CREATE FUNCTION game_character_is_death_item_set(value UUID[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT value IS NOT NULL AND cardinality(value)<=10
    AND (cardinality(value)=0 OR (array_ndims(value)=1 AND array_lower(value,1)=1))
    AND NOT EXISTS(SELECT 1 FROM unnest(value) v WHERE v IS NULL OR NOT game_character_is_uuid_v7(v))
    AND cardinality(value)=(SELECT count(DISTINCT v) FROM unnest(value) v)
$$;
ALTER TABLE game_character_death_receipts
 DROP CONSTRAINT game_character_death_receipts_no_amulet_until_death3,
 DROP CONSTRAINT game_character_death_receipts_no_lost_items_until_death3,
 ADD COLUMN item_selection_root BYTEA CHECK(octet_length(item_selection_root)=32),
 ADD CONSTRAINT game_character_death_receipts_item_selection CHECK(
    game_character_is_death_item_set(lost_item_ids)
    AND (amulet_of_loss_item_id IS NULL OR cardinality(lost_item_ids)=0)
    AND (item_selection_root IS NOT NULL OR (amulet_of_loss_item_id IS NULL AND cardinality(lost_item_ids)=0))
 );

-- A record is terminal only after its transaction and deferred item/audit guards commit.
CREATE FUNCTION game_player_death_items_complete(occurrence UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 SELECT NOT EXISTS(
    SELECT 1 FROM game_character_death_receipts d,
      LATERAL unnest(d.lost_item_ids || CASE WHEN d.amulet_of_loss_item_id IS NULL THEN ARRAY[]::uuid[] ELSE ARRAY[d.amulet_of_loss_item_id] END) selected(item)
    WHERE d.death_occurrence_id=occurrence AND NOT EXISTS(
      SELECT 1 FROM game_player_death_item_records r
       WHERE r.death_occurrence_id=d.death_occurrence_id AND r.item_instance_id=selected.item
         AND r.character_id=d.character_id
         AND r.operation=CASE WHEN d.amulet_of_loss_item_id=selected.item THEN 2 ELSE 1 END))
$$;
CREATE FUNCTION game_character_respawn_items_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_player_death_items_complete(OLD.death_occurrence_id) THEN
   RAISE EXCEPTION 'death item workflow is incomplete' USING ERRCODE='23514';
 END IF;
 RETURN OLD;
END $$;
CREATE TRIGGER game_character_respawn_items_guard BEFORE DELETE ON game_character_pending_respawns
 FOR EACH ROW EXECUTE FUNCTION game_character_respawn_items_guard();
REVOKE ALL ON FUNCTION game_character_is_death_item_set(UUID[]), game_player_death_items_complete(UUID), game_character_respawn_items_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_is_death_item_set(UUID[]), game_player_death_items_complete(UUID), game_character_respawn_items_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_character_is_death_item_set(UUID[]), game_player_death_items_complete(UUID) TO oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_character_is_death_item_set(UUID[]) SET search_path = %I, pg_temp', current_schema());
 EXECUTE format('ALTER FUNCTION game_player_death_items_complete(UUID) SET search_path = %I, pg_temp', current_schema());
 EXECUTE format('ALTER FUNCTION game_character_respawn_items_guard() SET search_path = %I, pg_temp', current_schema());
END $$;
