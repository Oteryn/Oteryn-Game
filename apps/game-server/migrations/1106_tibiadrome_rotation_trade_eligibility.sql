-- W17; docs/reference/tibia-manual/combat.md:263: an auction spanning
-- rotation end forfeits that rotation's points/prizes. Later trade events must
-- not rewrite the historical result. Accepted Bazaar clocks are Unix seconds
-- (durability/bazaar/mod.rs:276); immutable per-Character root revisions resolve
-- multiple events recorded in the same second (0995_bazaar_domain_proofs.sql:17).
-- Rollback: forward migration reverting Drome callers, then dropping this reader.
CREATE FUNCTION game_tibiadrome_lifecycle_at_cut(p_character UUID, p_cut BIGINT)
RETURNS SMALLINT LANGUAGE sql STABLE STRICT SECURITY INVOKER AS $$
 SELECT COALESCE((
  SELECT t.new_lifecycle
  FROM game_bazaar_character_transitions t
  JOIN game_bazaar_decisions d USING(issuer,operation_id)
  WHERE t.character_id=p_character AND d.result_code='CHAR_CMD_OK'
    AND d.decided_at<p_cut
  ORDER BY d.decided_at DESC,t.new_revision DESC
  LIMIT 1
 ),1::smallint)
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_tibiadrome_lifecycle_at_cut(UUID,BIGINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_tibiadrome_lifecycle_at_cut(UUID,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_tibiadrome_lifecycle_at_cut(UUID,BIGINT)
 TO oteryn_game_runtime,oteryn_game_control;
