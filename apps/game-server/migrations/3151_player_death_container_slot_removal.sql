-- A death may drop the main backpack (source_slot 9). The runtime role keeps no
-- DELETE on game_item_container_slots; it may remove only the exact slot its own
-- same-transaction death record names. The slot guards and deferred proofs still run.
CREATE OR REPLACE FUNCTION game_player_death_container_slot_remove(p_tx UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 DELETE FROM game_item_container_slots s USING game_player_death_item_records r
  WHERE r.transaction_id=p_tx AND r.source_slot=9 AND r.created_xact_id=pg_current_xact_id()
   AND s.item_instance_id=r.item_instance_id AND s.character_id=r.character_id
   AND s.world_id=r.world_id AND s.placed_transaction_id=r.source_placement;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'death container slot removal lacks its same-transaction record' USING ERRCODE='23514';
 END IF;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_player_death_container_slot_remove(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_player_death_container_slot_remove(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_player_death_container_slot_remove(UUID) TO oteryn_game_runtime;
