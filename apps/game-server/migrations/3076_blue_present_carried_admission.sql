-- Blue Present gifts enter a compatible carried stack, a free backpack slot,
-- or the existing fenced Ground owner. All destinations share the craft debit.
DO $migration$
DECLARE body TEXT; anchor TEXT; arm TEXT; begin_at INT; previous TEXT;
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
    WHERE conrelid='game_item_craft_records'::regclass
      AND conname='game_item_craft_records_outputs_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE format($check$ALTER TABLE game_item_craft_records
        ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (
            consume_source AND recipe='oteryn:reward-bag.event_6570'
            AND before_item->>'definition_production_key'='oteryn:item.tibia.i6570'
            AND jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 5 AND 10
        ))$check$, previous);

    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    anchor:=$old$OR (SELECT count(*) FROM game_item_container_entries
          WHERE parent_item_instance_id=NEW.parent_item_instance_id)
          < (o->'ground'->>'parent_capacity')::int$old$;
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing Blue Present slot-only Ground admission'; END IF;
    body:=replace(body,anchor,'');
    body:=replace(body,'a carried present, a full backpack and current Channel','a carried present and current Channel');

    anchor:='FOR o IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output admission'; END IF;
    arm:=$arm$
        IF o ? 'merge_before' THEN
            IF NEW.recipe<>'oteryn:reward-bag.event_6570' OR NOT NEW.consume_source
                OR NEW.ingredient_item_instance_id IS NOT NULL
                OR i.definition_production_key<>'oteryn:item.tibia.i6570'
                OR (o->>'item')::uuid=NEW.source_item_instance_id OR o ? 'ground'
                OR (o->>'count')::int NOT BETWEEN 1 AND 10
                OR (SELECT sum((line->>'count')::int) FROM jsonb_array_elements(NEW.outputs) line) NOT BETWEEN 1 AND 10
                OR EXISTS (SELECT 1 FROM jsonb_array_elements(NEW.outputs) line
                    WHERE line ? 'ground' OR line->>'key' IS DISTINCT FROM o->>'key'
                       OR line->>'revision' IS DISTINCT FROM o->>'revision')
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_instances n
                    JOIN game_item_container_entries e USING(item_instance_id)
                    WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=NEW.world_id
                      AND to_jsonb(n)=o->'merge_before' AND n.lifecycle=1
                      AND n.quantity+(o->>'count')::int<=100
                      AND n.definition_family='Item' AND n.definition_production_key=o->>'key'
                      AND n.definition_revision_ref=o->>'revision'
                      AND e.world_id=NEW.world_id AND e.character_id=NEW.character_id
                      AND e.parent_item_instance_id=NEW.parent_item_instance_id
                      AND e.placement_ordinal=(o->>'ordinal')::numeric
                      AND NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=n.item_instance_id)
                      AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=n.item_instance_id)
                    FOR UPDATE OF n,e
                ) THEN RAISE EXCEPTION 'invalid Blue Present stack merge' USING ERRCODE='23514'; END IF;
            CONTINUE;
        END IF;
$arm$;
    EXECUTE replace(body,anchor,anchor||arm);

    body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
    anchor:='FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft mint count'; END IF;
    body:=replace(body,anchor,$count$FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
        <>(SELECT count(*) FROM jsonb_array_elements(r.outputs) line WHERE NOT (line ? 'merge_before'))$count$);
    anchor:='FROM game_item_instances WHERE last_transaction_id=r.transaction_id)';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft update count'; END IF;
    body:=replace(body,anchor,$count$FROM game_item_instances WHERE last_transaction_id=r.transaction_id
        AND item_instance_id NOT IN (SELECT (line->>'item')::uuid
                                     FROM jsonb_array_elements(r.outputs) line WHERE line ? 'merge_before'))$count$);
    anchor:='FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP';
    IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output completion'; END IF;
    arm:=$arm$
        IF o ? 'merge_before' THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_item_instances n
                JOIN game_item_container_entries e USING(item_instance_id)
                WHERE n.item_instance_id=(o->>'item')::uuid
                  AND n.last_transaction_id=r.transaction_id
                  AND n.quantity=(o->'merge_before'->>'quantity')::bigint+(o->>'count')::bigint
                  AND n.state_revision=(o->'merge_before'->>'state_revision')::numeric+1
                  AND (to_jsonb(n)-ARRAY['quantity','last_transaction_id','state_revision']) =
                      ((o->'merge_before')-ARRAY['quantity','last_transaction_id','state_revision'])
                  AND e.world_id=r.world_id AND e.character_id=r.character_id
                  AND e.parent_item_instance_id=r.parent_item_instance_id
                  AND e.placement_ordinal=(o->>'ordinal')::numeric
            ) THEN RAISE EXCEPTION 'incomplete Blue Present stack merge' USING ERRCODE='23514'; END IF;
            CONTINUE;
        END IF;
$arm$;
    EXECUTE replace(body,anchor,anchor||arm);

    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing instance guard body'; END IF;
    arm:=$arm$
        IF TG_OP='UPDATE' AND NEW.last_transaction_id IS NOT NULL
           AND (to_jsonb(NEW)-ARRAY['quantity','last_transaction_id']) =
               (to_jsonb(OLD)-ARRAY['quantity','last_transaction_id'])
           AND EXISTS (
              SELECT 1 FROM game_item_craft_records r, LATERAL jsonb_array_elements(r.outputs) o
              WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
                AND r.recipe='oteryn:reward-bag.event_6570' AND o->'merge_before'=to_jsonb(OLD)
                AND (o->>'item')::uuid=OLD.item_instance_id
                AND NEW.quantity=OLD.quantity+(o->>'count')::bigint
           ) THEN
            NEW.state_revision:=OLD.state_revision+1;
            RETURN NEW;
        END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

    body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing deferred instance guard body'; END IF;
    arm:=$arm$
        IF NEW.last_transaction_id IS NOT NULL AND EXISTS (
            SELECT 1 FROM game_item_craft_records r, LATERAL jsonb_array_elements(r.outputs) o
            WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
              AND r.recipe='oteryn:reward-bag.event_6570' AND o ? 'merge_before'
              AND (o->>'item')::uuid=NEW.item_instance_id
        ) THEN
            PERFORM game_item_craft_complete(NEW.last_transaction_id);
            RETURN NULL;
        END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;
