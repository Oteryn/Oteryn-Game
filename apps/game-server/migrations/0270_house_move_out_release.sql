-- HOUSE-1b final ordinary release.  Custody, its HouseOperation event, and the
-- owner/slot/ACL release share one physical transaction: a restart observes
-- either the owned frozen house or the completely vacant successor.
CREATE FUNCTION game_house_move_out_release(op UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_house_dispositions%ROWTYPE; p game_house_properties%ROWTYPE; now_ms BIGINT;
BEGIN
 SELECT * INTO d FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF NOT FOUND OR d.state<>3 OR d.released_at IS NOT NULL THEN RETURN FALSE; END IF;
 IF d.cause<>'MOVE_OUT' OR d.owner_guild_id IS NOT NULL THEN
  RAISE EXCEPTION 'ordinary release requires a personal move-out' USING ERRCODE='23514';
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=d.world_id AND house_key=d.house_key FOR UPDATE;
 IF NOT FOUND OR p.owner_character_id IS DISTINCT FROM d.owner_character_id
  OR p.ownership_generation<>d.ownership_revision
  OR NOT EXISTS(SELECT 1 FROM game_account_housing_slots s WHERE s.account_id=p.owner_account_id
   AND s.world_id=d.world_id AND s.house_key=d.house_key)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations l WHERE l.world_id=d.world_id AND l.house_key=d.house_key)
  OR EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.world_id=d.world_id
   AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=d.house_key AND s.session_state IN(1,2))
  OR EXISTS(SELECT 1 FROM game_house_scope_handoffs h WHERE h.world_id=d.world_id
   AND h.house_key=d.house_key AND h.state IN(1,2))
  OR NOT EXISTS(SELECT 1 FROM game_house_disposition_audit_outbox a
   WHERE a.operation_id=op AND a.written_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'house move-out release lacks current custody proof' USING ERRCODE='23514';
 END IF;
 DELETE FROM game_house_acl_entries WHERE world_id=d.world_id AND house_key=d.house_key;
 DELETE FROM game_house_acl WHERE world_id=d.world_id AND house_key=d.house_key;
 DELETE FROM game_house_editors WHERE world_id=d.world_id AND house_key=d.house_key;
 DELETE FROM game_account_housing_slots WHERE account_id=p.owner_account_id AND world_id=d.world_id;
 UPDATE game_house_properties SET state=1,owner_character_id=NULL,owner_account_id=NULL,
  paid_until=NULL,content_fence_operation=NULL,ownership_generation=ownership_generation+1,
  revision=revision+1,acl_revision=acl_revision+1
  WHERE world_id=d.world_id AND house_key=d.house_key;
 UPDATE game_house_ownership SET owner_character_id=NULL,owner_guild_id=NULL,
  ownership_revision=ownership_revision+1 WHERE world_id=d.world_id AND house_key=d.house_key;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 UPDATE game_house_dispositions SET released_at=greatest(now_ms,d.drained_at)
  WHERE operation_id=op;
 RETURN TRUE;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_move_out_release(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_move_out_release(UUID) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_move_out_release(UUID) TO oteryn_game_runtime;
