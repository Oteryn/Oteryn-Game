-- XDECAY. Follow at most seven nested parents to a live Channel Ground root.
-- Root ownership comes from the current Channel assignment, including after
-- takeover of a container placed by an older generation.
CREATE FUNCTION game_item_decay_ground_path(
 p_parent UUID, p_world UUID, p_channel UUID, p_generation NUMERIC
) RETURNS UUID[] LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
 WITH RECURSIVE ancestry(item_id, path) AS (
  SELECT p_parent, ARRAY[p_parent]
  UNION ALL
  SELECT n.parent_item_instance_id, a.path || n.parent_item_instance_id
  FROM ancestry a JOIN game_item_nested_entries n ON n.item_instance_id=a.item_id
  JOIN game_item_instances i ON i.item_instance_id=a.item_id
  WHERE n.world_id=p_world AND i.world_id=p_world AND i.lifecycle=1
   AND cardinality(a.path)<8 AND NOT n.parent_item_instance_id=ANY(a.path)
 )
 SELECT a.path FROM ancestry a
 JOIN game_item_ground_locations g ON g.item_instance_id=a.item_id
 JOIN game_item_instances i ON i.item_instance_id=a.item_id
 JOIN game_runtime_scope_assignments s ON s.world_id=g.world_id AND s.channel_id=g.channel_id
 WHERE g.world_id=p_world AND g.channel_id=p_channel AND g.scope_kind=1
  AND i.world_id=p_world AND i.lifecycle=1
  AND s.scope_kind=1 AND s.state=1 AND s.ownership_generation=p_generation
  AND g.runtime_scope_ownership_generation<=p_generation
$$;
DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_decay_ground_path(UUID,UUID,UUID,NUMERIC)
  SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_decay_ground_path(UUID,UUID,UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_decay_ground_path(UUID,UUID,UUID,NUMERIC) TO oteryn_game_runtime;

-- Add a sessionless shape to the existing kind 2. Keep all three original
-- receipt shapes and the independently enforced Channel/Instance constraint.
DO $migration$
DECLARE shape RECORD;
BEGIN
 SELECT conname, pg_get_constraintdef(oid) AS definition INTO STRICT shape
 FROM pg_constraint
 WHERE conrelid='game_spell_field_temporal_receipts'::regclass AND contype='c'
  AND pg_get_constraintdef(oid) LIKE '%num_nonnulls(game_session_id,%';
 EXECUTE format('ALTER TABLE game_spell_field_temporal_receipts DROP CONSTRAINT %I',shape.conname);
 EXECUTE format('ALTER TABLE game_spell_field_temporal_receipts ADD CONSTRAINT %I CHECK (
  %s OR ((custody_kind=2 AND channel_id IS NOT NULL AND source_parent IS NOT NULL
   AND source_ordinal>0 AND num_nonnulls(character_id,game_session_id,
    character_lease_generation,connection_generation,source_slot,
    equipment_revision_before,source_stack_ordinal)=0) IS TRUE))',
  shape.conname,substring(shape.definition FROM 7));
END $migration$;

DO $migration$
DECLARE body TEXT; anchor TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
 anchor:='ELSIF r.custody_kind=2 THEN';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected nested timer predecessor proof';
 END IF;
 EXECUTE replace(body,anchor,$arm$
 ELSIF r.custody_kind=2 AND r.character_id IS NULL THEN
  DECLARE parents UUID[];
  BEGIN
   parents:=game_item_decay_ground_path(r.source_parent,r.world_id,r.channel_id,r.ownership_generation);
   IF parents IS NULL OR NOT game_item_general_decay_chain(OLD.item_instance_id) THEN
    RAISE EXCEPTION 'timer lacks qualified Channel Ground ancestry' USING ERRCODE='23514';
   END IF;
   -- A normal custody move locks and revises the moved Item. Hold the ancestry
   -- stable through commit, and retry on contention with an in-flight move.
   PERFORM 1 FROM game_item_instances WHERE item_instance_id=ANY(parents)
    ORDER BY item_instance_id FOR NO KEY UPDATE NOWAIT;
   matches:=parents IS NOT DISTINCT FROM
    game_item_decay_ground_path(r.source_parent,r.world_id,r.channel_id,r.ownership_generation)
    AND EXISTS(SELECT 1 FROM game_item_nested_entries n
     WHERE n.item_instance_id=OLD.item_instance_id AND n.world_id=r.world_id
      AND n.parent_item_instance_id=r.source_parent AND n.placement_ordinal=r.source_ordinal);
  END;
 ELSIF r.custody_kind=2 THEN
 $arm$);

 body:=pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
 anchor:='IF NEW.custody_kind<>1 AND NOT EXISTS(';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected nested timer session proof';
 END IF;
 body:=replace(body,anchor,$arm$
 IF NEW.custody_kind=2 AND NEW.character_id IS NULL
  AND game_item_decay_ground_path(NEW.source_parent,NEW.world_id,NEW.channel_id,NEW.ownership_generation) IS NULL
 THEN
  RAISE EXCEPTION 'nested timer lost its Channel Ground ancestry' USING ERRCODE='23514';
 END IF;
 IF NEW.custody_kind<>1 AND NEW.character_id IS NOT NULL AND NOT EXISTS(
 $arm$);
 anchor:='IF p.target_family IS NOT NULL AND NOT (';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected nested timer successor proof';
 END IF;
 EXECUTE replace(body,anchor,$arm$
 IF p.target_family IS NOT NULL AND NOT (
  (NEW.custody_kind=2 AND NEW.character_id IS NULL AND EXISTS(
    SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id
     AND n.world_id=NEW.world_id AND n.parent_item_instance_id=NEW.source_parent
     AND n.placement_ordinal=NEW.source_ordinal))
  OR
 $arm$);

 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 anchor:='WHERE r.custody_kind=2 AND r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id
      AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected nested timer removal proof';
 END IF;
 EXECUTE replace(body,anchor,anchor||$arm$
      AND (r.character_id IS NOT NULL OR EXISTS(
       SELECT 1 FROM game_spell_field_temporal_before b WHERE b.transaction_id=r.transaction_id
        AND b.item_instance_id=r.item_instance_id AND b.created_xact_id=pg_current_xact_id()
        AND b.custody_kind=2 AND b.character_id IS NULL
        AND b.source_parent=r.source_parent AND b.source_ordinal=r.source_ordinal))
 $arm$);
END $migration$;
