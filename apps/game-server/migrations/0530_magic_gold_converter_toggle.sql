-- MAGICCONV: the magic gold converter toggles between its inactive (i28525) and active
-- (i28526) forms. Cause 9 moves the unchanged charge row with the definition under one
-- timed write; the older guard bodies are retained and only gain this closed arm.
ALTER TABLE game_item_timed_state_writes DROP CONSTRAINT game_item_timed_state_writes_cause_check,
 ADD CONSTRAINT game_item_timed_state_writes_cause_check CHECK(cause BETWEEN 1 AND 9);

CREATE FUNCTION game_coin_converter_toggle_complete(w game_item_timed_state_writes) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; t game_item_timed_states%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=w.item_instance_id;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=w.item_instance_id;
 IF w.cause<>9 OR w.created_xact_id<>pg_current_xact_id() OR w.holder_character_id IS NULL
  OR w.definition_before_family<>'Item' OR w.definition_after_family<>'Item'
  OR (w.definition_before_production_key,w.definition_after_production_key) NOT IN (
     ('oteryn:item.tibia.i28525','oteryn:item.tibia.i28526'),
     ('oteryn:item.tibia.i28526','oteryn:item.tibia.i28525'))
  OR w.full_charges IS NULL OR w.full_remaining_ms IS NOT NULL OR w.lit_continuous
  OR w.remaining_ms_before IS NOT NULL OR w.remaining_ms_after IS NOT NULL
  OR w.deadline_before IS NOT NULL OR w.deadline_after IS NOT NULL
  OR w.charges_before IS NULL OR w.charges_after IS DISTINCT FROM w.charges_before
  OR w.charges_after>w.full_charges
  OR (w.expected_revision=0 AND w.charges_before<>w.full_charges)
  OR t.item_instance_id IS NULL OR t.state_revision<>w.expected_revision+1
  OR t.charges IS DISTINCT FROM w.charges_after OR t.remaining_ms IS NOT NULL OR t.deadline_at IS NOT NULL
  OR i.lifecycle<>1 OR i.quantity<>1 OR i.last_transaction_id IS DISTINCT FROM w.transaction_id
  OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    IS DISTINCT FROM (w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref)
  OR EXISTS(SELECT 1 FROM game_item_timed_state_writes other WHERE other.created_xact_id=pg_current_xact_id()
    AND (other.item_instance_id,other.expected_revision)<>(w.item_instance_id,w.expected_revision))
  OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e
     WHERE e.item_instance_id=w.item_instance_id AND e.character_id=w.holder_character_id) THEN
  RAISE EXCEPTION 'converter toggle requires its fixed pair, unchanged charges, row and custody' USING ERRCODE='23514';
 END IF;
 -- Both forms share one pinned charge budget.
 INSERT INTO game_item_timed_definitions(definition_family,definition_production_key,definition_revision_ref,full_charges,full_remaining_ms,lit_continuous)
 VALUES(w.definition_before_family,w.definition_before_production_key,w.definition_before_revision_ref,w.full_charges,NULL,false),
       (w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref,w.full_charges,NULL,false)
 ON CONFLICT DO NOTHING;
 IF (SELECT count(*) FROM game_item_timed_definitions d WHERE
     ((d.definition_family,d.definition_production_key,d.definition_revision_ref)=(w.definition_before_family,w.definition_before_production_key,w.definition_before_revision_ref)
      OR (d.definition_family,d.definition_production_key,d.definition_revision_ref)=(w.definition_after_family,w.definition_after_production_key,w.definition_after_revision_ref))
     AND d.full_charges=w.full_charges AND d.full_remaining_ms IS NULL AND NOT d.lit_continuous)<>2 THEN
  RAISE EXCEPTION 'converter toggle disagrees with pinned definitions' USING ERRCODE='23514';
 END IF;
END;
$$;

DO $$
DECLARE body TEXT; name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard()','game_item_instance_change_proven()'] LOOP
  SELECT pg_get_functiondef(name::regprocedure) INTO body;
  IF position('w.cause IN (2, 3, 8)' IN body)=0 THEN RAISE EXCEPTION 'missing timed transform arm in %',name; END IF;
  EXECUTE replace(body,'w.cause IN (2, 3, 8)','w.cause IN (2, 3, 8, 9)');
 END LOOP;
 SELECT pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure) INTO body;
 IF position('IF NEW.cause=8 THEN' IN body)=0 THEN RAISE EXCEPTION 'missing repair arm'; END IF;
 EXECUTE replace(body,'IF NEW.cause=8 THEN',
   E'IF NEW.cause=9 THEN PERFORM game_coin_converter_toggle_complete(NEW); RETURN NULL; END IF;\n IF NEW.cause=8 THEN');
 name:='game_coin_converter_toggle_complete(game_item_timed_state_writes)';
 EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
END;
$$;
