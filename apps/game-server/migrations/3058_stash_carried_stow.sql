-- Expanded carried stow keeps one value-chain entry and bounded event per
-- definition chunk. Older commands retain their single ordinal-zero receipt.
ALTER TABLE game_character_stash_operations
    ADD COLUMN batch_ordinal SMALLINT NOT NULL DEFAULT 0 CHECK (batch_ordinal BETWEEN 0 AND 499),
    ADD CHECK (batch_ordinal=0 OR (action=2 AND outcome=2)),
    DROP CONSTRAINT game_character_stash_operations_game_session_id_command_id_key,
    ADD UNIQUE (game_session_id,command_id,batch_ordinal);
DO $constraint$
DECLARE old_name TEXT;
BEGIN
    SELECT conname INTO STRICT old_name FROM pg_constraint
    WHERE conrelid='game_character_stash_item_lines'::regclass AND contype='u'
      AND pg_get_constraintdef(oid)='UNIQUE (transaction_id, placement_ordinal)';
    EXECUTE format('ALTER TABLE game_character_stash_item_lines DROP CONSTRAINT %I',old_name);
END $constraint$;
ALTER TABLE game_character_stash_item_lines
    ADD UNIQUE (transaction_id,parent_item_instance_id,placement_ordinal);

CREATE FUNCTION game_stash_carried_placement(
    p_item UUID, p_parent UUID, p_ordinal NUMERIC, p_character UUID, p_world UUID
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    WITH RECURSIVE path(item, parent, ordinal, depth) AS (
        SELECT p_item, p_parent, p_ordinal, 0
        UNION ALL
        SELECT e.item_instance_id, e.parent_item_instance_id, e.placement_ordinal, p.depth+1
        FROM path p JOIN game_item_nested_entries c
          ON (c.item_instance_id,c.parent_item_instance_id,c.placement_ordinal,c.world_id)
            =(p.item,p.parent,p.ordinal,p_world)
        JOIN LATERAL (
            SELECT n.item_instance_id,n.parent_item_instance_id,n.placement_ordinal
            FROM game_item_nested_entries n WHERE n.item_instance_id=p.parent AND n.world_id=p_world
            UNION ALL
            SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal
            FROM game_item_container_entries e WHERE e.item_instance_id=p.parent AND e.world_id=p_world
              AND e.character_id=p_character
        ) e ON p.depth<7
    )
    SELECT EXISTS (
        SELECT 1 FROM path p JOIN game_item_container_entries e
          ON (e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.character_id,e.world_id)
           =(p.item,p.parent,p.ordinal,p_character,p_world)
        JOIN game_item_container_slots s
          ON (s.character_id,s.item_instance_id,s.world_id)=(p_character,e.parent_item_instance_id,p_world)
    )
$$;
DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
    EXECUTE format('ALTER FUNCTION game_stash_carried_placement(uuid,uuid,numeric,uuid,uuid)
                    SET search_path=pg_catalog,%I,pg_temp',current_schema());
    body:=pg_get_functiondef('game_stash_admit()'::regprocedure);
    anchor:=$old$IF TG_TABLE_NAME='game_character_stash_operations' THEN$old$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash operation guard'; END IF;
    body:=replace(body,anchor,$new$IF TG_TABLE_NAME='game_character_stash_operations' THEN
  IF NEW.batch_ordinal>0 AND NOT EXISTS(
    SELECT 1 FROM game_character_stash_operations previous
    WHERE previous.game_session_id=NEW.game_session_id AND previous.command_id=NEW.command_id
      AND previous.batch_ordinal=NEW.batch_ordinal-1
      AND previous.created_xact_id=pg_current_xact_id()
      AND (previous.character_id,previous.world_id,previous.intent,previous.action,previous.outcome)
        =(NEW.character_id,NEW.world_id,NEW.intent,2,2)
      AND (previous.house_key,previous.house_scope_ownership_generation)
        IS NOT DISTINCT FROM (NEW.house_key,NEW.house_scope_ownership_generation)
  ) THEN RAISE EXCEPTION 'stash batch requires its exact current predecessor' USING ERRCODE='23514'; END IF;$new$);
    anchor:='i.quantity<>NEW.quantity';
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash source quantity proof'; END IF;
    body:=replace(body,anchor,'i.quantity<NEW.quantity');
    anchor:=$old$OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
            ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
            WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id
              AND e.world_id=r.world_id AND b.world_id=r.world_id
              AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.placement_ordinal)$old$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash source proof'; END IF;
    EXECUTE replace(body,anchor,
      'OR NOT game_stash_carried_placement(i.item_instance_id,NEW.parent_item_instance_id,NEW.placement_ordinal,r.character_id,r.world_id)');

    body:=pg_get_functiondef('game_stash_complete(uuid)'::regprocedure);
    anchor:=$old$    OR (r.action=2 AND (SELECT count(*) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction)<>1)
$old$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash line count proof'; END IF;
    body:=replace(body,anchor,'');
    anchor:='(CASE WHEN r.action=2 THEN 1 ELSE 0 END)';
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash changed item count proof'; END IF;
    body:=replace(body,anchor,'(CASE WHEN r.action=2 THEN (SELECT count(*) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction) ELSE 0 END)');
    anchor:=$old$IF i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id IS DISTINCT FROM p_transaction$old$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash input state proof'; END IF;
    body:=replace(body,anchor,$new$IF i.quantity IS DISTINCT FROM (l.before_item->>'quantity')::bigint-l.quantity
     OR i.lifecycle IS DISTINCT FROM (CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
     OR i.last_transaction_id IS DISTINCT FROM p_transaction$new$);
    anchor:=$old$OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)$old$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash input placement proof'; END IF;
    EXECUTE replace(body,anchor,$new$OR (i.quantity=0 AND (
       EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
     OR (i.quantity>0 AND NOT game_stash_carried_placement(i.item_instance_id,l.parent_item_instance_id,l.placement_ordinal,r.character_id,r.world_id))$new$);

    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    anchor:=$old$IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
        (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
      RAISE EXCEPTION 'stash retires exactly the input'$old$;
    replacement:=$new$IF NOT EXISTS(SELECT 1 FROM game_character_stash_item_lines l
      WHERE l.item_instance_id=OLD.item_instance_id AND l.transaction_id=NEW.last_transaction_id
        AND NEW.quantity=OLD.quantity-l.quantity AND NEW.lifecycle=(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END))
       OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
      RAISE EXCEPTION 'stash debits exactly the input'$new$;
    IF position(anchor IN body)=0 THEN RAISE EXCEPTION 'missing stash item change proof'; END IF;
    EXECUTE replace(body,anchor,replacement);

    body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    IF regexp_instr(body,'\mBEGIN\M',1,1,0,'i')=0 THEN RAISE EXCEPTION 'missing nested custody guard'; END IF;
    EXECUTE regexp_replace(body,'\mBEGIN\M',$arm$BEGIN
      IF TG_OP='DELETE' AND EXISTS (
        SELECT 1 FROM game_character_stash_item_lines l
        JOIN game_character_stash_operations r USING(transaction_id)
        JOIN game_item_instances i USING(item_instance_id)
        WHERE l.direction=1 AND l.item_instance_id=OLD.item_instance_id
          AND r.world_id=OLD.world_id AND l.parent_item_instance_id=OLD.parent_item_instance_id
          AND l.placement_ordinal=OLD.placement_ordinal AND i.lifecycle=2 AND i.quantity=0
          AND i.last_transaction_id=l.transaction_id AND l.created_xact_id=pg_current_xact_id()
      ) THEN RETURN NULL; END IF;
    $arm$,'i');
END $migration$;

REVOKE ALL ON FUNCTION game_stash_carried_placement(uuid,uuid,numeric,uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_stash_carried_placement(uuid,uuid,numeric,uuid,uuid) TO oteryn_game_runtime;
