-- W07 Soul Core fusion, Soul Prism and Exalted Core extend the existing CRAFT owner.
-- Crystal 201189f soulpit.lua:155-174, soul_prism.lua:43-107, exalted_core.lua:34-95.
-- One immutable craft receipt proves every input decrement, one fresh/stack result, its exact
-- carried/ground custody and the existing Type2 audit. No alternative ItemInstance ledger.
-- Forward rollback: stop these recipes; retain all receipts, items and audit rows.
ALTER TABLE game_item_craft_records ADD COLUMN soul_core_proof JSONB;
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
    CHECK (jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 0 AND 4
        AND (jsonb_array_length(outputs)>0
            OR (NOT consume_source AND recipe='oteryn:gathering.crystal')
            OR (consume_source AND recipe='oteryn:soulpit.entry')));
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_soul_core_shape CHECK (
    (recipe IN ('oteryn:soulpit.fuse','oteryn:soulpit.prism','oteryn:soulpit.exalted','oteryn:soulpit.entry'))
        = (soul_core_proof IS NOT NULL)
    AND (soul_core_proof IS NULL OR (
        jsonb_typeof(soul_core_proof) IS NOT DISTINCT FROM 'object'
        AND octet_length(soul_core_proof::text) BETWEEN 1 AND 8192
        AND soul_core_proof ?& ARRAY['inputs','delivery','channel','scope_generation']
        AND jsonb_typeof(soul_core_proof->'inputs') IS NOT DISTINCT FROM 'array'
        AND jsonb_array_length(soul_core_proof->'inputs') BETWEEN 1 AND 2
        AND jsonb_typeof(soul_core_proof->'delivery') IS NOT DISTINCT FROM 'object'
        AND jsonb_typeof(soul_core_proof->'channel') IS NOT DISTINCT FROM 'string'
        AND jsonb_typeof(soul_core_proof->'scope_generation') IS NOT DISTINCT FROM 'string'
        AND ((soul_core_proof->'delivery'->>'kind'='stack')=(soul_core_proof ? 'credit'))
        AND jsonb_array_length(outputs)=CASE WHEN recipe='oteryn:soulpit.entry' THEN 0 ELSE 1 END
        AND consume_source))
);

CREATE FUNCTION game_soul_core_craft_admit(p game_item_craft_records) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE input JSONB; item game_item_instances%ROWTYPE; output JSONB; credit JSONB;
        delivery JSONB := p.soul_core_proof->'delivery';
        source_count INT := 0; target_count INT := 0;
BEGIN
    IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(p.soul_core_proof->'inputs'))
        <>jsonb_array_length(p.soul_core_proof->'inputs')
    THEN RAISE EXCEPTION 'duplicate Soul Core input' USING ERRCODE='23514'; END IF;
    FOR input IN SELECT value FROM jsonb_array_elements(p.soul_core_proof->'inputs') LOOP
        IF jsonb_typeof(input) IS DISTINCT FROM 'object'
            OR NOT input ?& ARRAY['item','before','parent','ordinal','nested','consume']
            OR jsonb_typeof(input->'before') IS DISTINCT FROM 'object'
            OR jsonb_typeof(input->'item') IS DISTINCT FROM 'string'
            OR jsonb_typeof(input->'parent') IS DISTINCT FROM 'string'
            OR jsonb_typeof(input->'ordinal') IS DISTINCT FROM 'string'
            OR jsonb_typeof(input->'nested') IS DISTINCT FROM 'boolean'
            OR jsonb_typeof(input->'consume') IS DISTINCT FROM 'number'
            OR (input->>'consume') !~ '^[0-2]$'
            OR (input->>'consume')::int NOT BETWEEN 0 AND 2
        THEN RAISE EXCEPTION 'invalid Soul Core input proof' USING ERRCODE='23514'; END IF;
        SELECT * INTO item FROM game_item_instances
            WHERE item_instance_id=(input->>'item')::uuid FOR UPDATE;
        IF item.item_instance_id IS NULL OR to_jsonb(item) IS DISTINCT FROM input->'before'
            OR item.lifecycle<>1 OR item.quantity<(input->>'consume')::int
            OR item.world_id<>p.world_id OR item.definition_family<>'Item'
            OR NOT EXISTS (SELECT 1 FROM game_item_loop_owner((input->>'parent')::uuid) owner
                WHERE owner.owner_character=p.character_id AND owner.depth BETWEEN 0 AND 7)
            OR NOT EXISTS (SELECT 1 FROM (
                SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested
                    FROM game_item_container_entries WHERE character_id=p.character_id
                UNION ALL
                SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true
                    FROM game_item_nested_entries
                ) entry WHERE entry.item_instance_id=item.item_instance_id AND entry.world_id=p.world_id
                    AND entry.parent_item_instance_id=(input->>'parent')::uuid
                    AND entry.placement_ordinal=(input->>'ordinal')::numeric
                    AND entry.nested=(input->>'nested')::boolean)
            OR EXISTS (SELECT 1 FROM game_item_keys WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_timed_states WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_texts WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_imbuements WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=item.item_instance_id)
        THEN RAISE EXCEPTION 'Soul Core input is not an unchanged plain carried leaf' USING ERRCODE='23514'; END IF;
        IF item.item_instance_id=p.source_item_instance_id THEN
            source_count := (input->>'consume')::int;
            IF p.before_item IS DISTINCT FROM input->'before'
                OR p.parent_item_instance_id<>(input->>'parent')::uuid
                OR p.source_ordinal<>(input->>'ordinal')::numeric
            THEN RAISE EXCEPTION 'Soul Core primary input binding' USING ERRCODE='23514'; END IF;
        ELSE
            target_count := (input->>'consume')::int;
        END IF;
    END LOOP;
    IF p.recipe='oteryn:soulpit.entry' THEN
        IF source_count<>1 OR target_count<>0
            OR jsonb_array_length(p.soul_core_proof->'inputs')<>1
            OR delivery->>'kind' IS DISTINCT FROM 'handoff'
            OR jsonb_typeof(delivery->'handoff') IS DISTINCT FROM 'string'
            OR NOT EXISTS (SELECT 1 FROM game_activity_handoff_groups g
                JOIN game_activity_handoff_participants participant USING(handoff_id)
                WHERE g.handoff_id=(delivery->>'handoff')::uuid AND g.state IN (1,2)
                    AND g.world_id=p.world_id AND participant.character_id=p.character_id
                    AND participant.source_game_session_id=p.game_session_id)
        THEN RAISE EXCEPTION 'Soulpit core debit requires its shared handoff' USING ERRCODE='23514'; END IF;
        RETURN;
    END IF;
    output := p.outputs->0;
    IF source_count<1 OR source_count+target_count NOT BETWEEN 1 AND 2
        OR (p.recipe IN ('oteryn:soulpit.fuse','oteryn:soulpit.exalted') AND source_count+target_count<>2)
        OR (p.recipe<>'oteryn:soulpit.fuse' AND source_count<>1)
        OR jsonb_typeof(output) IS DISTINCT FROM 'object'
        OR NOT output ?& ARRAY['item','key','revision','count','ordinal']
        OR jsonb_typeof(output->'item') IS DISTINCT FROM 'string'
        OR jsonb_typeof(output->'key') IS DISTINCT FROM 'string'
        OR jsonb_typeof(output->'revision') IS DISTINCT FROM 'string'
        OR jsonb_typeof(output->'count') IS DISTINCT FROM 'number'
        OR jsonb_typeof(output->'ordinal') IS DISTINCT FROM 'string'
        OR output->>'count' IS DISTINCT FROM '1'
        OR NOT game_character_is_uuid_v7((output->>'item')::uuid)
        OR (delivery->>'kind'<>'stack' AND EXISTS (SELECT 1 FROM game_item_instances WHERE item_instance_id=(output->>'item')::uuid))
        OR (output->>'count')::int<>1
        OR octet_length(output->>'key') NOT BETWEEN 1 AND 512
        OR octet_length(output->>'revision') NOT BETWEEN 1 AND 512
    THEN RAISE EXCEPTION 'invalid Soul Core consumption or output' USING ERRCODE='23514'; END IF;
    IF delivery->>'kind'='stack' THEN
        credit := p.soul_core_proof->'credit';
        IF jsonb_typeof(credit) IS DISTINCT FROM 'object'
            OR NOT credit ?& ARRAY['item','before','parent','ordinal','nested','consume']
            OR jsonb_typeof(credit->'item') IS DISTINCT FROM 'string'
            OR jsonb_typeof(credit->'before') IS DISTINCT FROM 'object'
            OR jsonb_typeof(credit->'parent') IS DISTINCT FROM 'string'
            OR jsonb_typeof(credit->'ordinal') IS DISTINCT FROM 'string'
            OR jsonb_typeof(credit->'nested') IS DISTINCT FROM 'boolean'
            OR credit->'consume' IS DISTINCT FROM '0'::jsonb
            OR credit->>'item' IS DISTINCT FROM output->>'item'
            OR credit->>'ordinal' IS DISTINCT FROM output->>'ordinal'
        THEN RAISE EXCEPTION 'invalid Soul Core stack credit proof' USING ERRCODE='23514'; END IF;
        SELECT * INTO item FROM game_item_instances WHERE item_instance_id=(credit->>'item')::uuid FOR UPDATE;
        IF item.item_instance_id IS NULL OR to_jsonb(item) IS DISTINCT FROM credit->'before'
            OR item.world_id<>p.world_id OR item.lifecycle<>1 OR item.quantity NOT BETWEEN 1 AND 99
            OR item.definition_family<>'Item' OR item.definition_production_key IS DISTINCT FROM output->>'key'
            OR item.definition_revision_ref IS DISTINCT FROM output->>'revision'
            OR NOT EXISTS (SELECT 1 FROM game_item_loop_owner((credit->>'parent')::uuid) owner
                WHERE owner.owner_character=p.character_id AND owner.depth BETWEEN 0 AND 7)
            OR NOT EXISTS (SELECT 1 FROM (
                SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested
                    FROM game_item_container_entries WHERE character_id=p.character_id
                UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries
                ) entry WHERE entry.item_instance_id=item.item_instance_id AND entry.world_id=p.world_id
                    AND entry.parent_item_instance_id=(credit->>'parent')::uuid
                    AND entry.placement_ordinal=(credit->>'ordinal')::numeric AND entry.nested=(credit->>'nested')::boolean)
            OR EXISTS (SELECT 1 FROM game_item_keys WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_timed_states WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_texts WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_imbuements WHERE item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=item.item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=item.item_instance_id)
        THEN RAISE EXCEPTION 'Soul Core stack credit requires exact compatible carried leaf' USING ERRCODE='23514'; END IF;
    ELSIF delivery->>'kind'='container' THEN
        IF NOT delivery ?& ARRAY['parent','nested']
            OR jsonb_typeof(delivery->'parent') IS DISTINCT FROM 'string'
            OR jsonb_typeof(delivery->'nested') IS DISTINCT FROM 'boolean'
            OR (output->>'ordinal')::numeric<1
            OR NOT EXISTS (SELECT 1 FROM game_item_loop_owner((delivery->>'parent')::uuid) owner
                WHERE owner.owner_character=p.character_id AND owner.depth BETWEEN 0 AND 7
                    AND (owner.depth>0)=(delivery->>'nested')::boolean)
            -- The donor adds the result before consuming either input. Later-depleted
            -- stacks cannot provide a carried slot or tree capacity for this admission.
            OR game_item_loop_tree_size(p.character_id)>=500
            OR (SELECT count(*) FROM (
                SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=(delivery->>'parent')::uuid
                UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=(delivery->>'parent')::uuid
                ) entries)>=20
            OR EXISTS (SELECT 1 FROM (
                SELECT parent_item_instance_id,placement_ordinal FROM game_item_container_entries
                UNION ALL SELECT parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
                ) entry WHERE entry.parent_item_instance_id=(delivery->>'parent')::uuid
                    AND entry.placement_ordinal=(output->>'ordinal')::numeric)
        THEN RAISE EXCEPTION 'invalid Soul Core output container' USING ERRCODE='23514'; END IF;
    ELSIF delivery->>'kind'='ground' THEN
        IF NOT delivery ?& ARRAY['position','corpse_ref','map_revision','content_revision','context']
            OR jsonb_typeof(delivery->'position') IS DISTINCT FROM 'string'
            OR jsonb_typeof(delivery->'corpse_ref') IS DISTINCT FROM 'string'
            OR jsonb_typeof(delivery->'map_revision') IS DISTINCT FROM 'string'
            OR jsonb_typeof(delivery->'content_revision') IS DISTINCT FROM 'string'
            OR jsonb_typeof(delivery->'context') IS DISTINCT FROM 'string'
            OR NOT game_character_is_uuid_v7((p.soul_core_proof->>'channel')::uuid)
            OR (p.soul_core_proof->>'scope_generation')::numeric NOT BETWEEN 1 AND 18446744073709551615
            OR octet_length(decode(delivery->>'position','hex'))<>10
            OR NOT game_item_loop_drop_ref(decode(delivery->>'corpse_ref','hex'))
        THEN RAISE EXCEPTION 'invalid Soul Core ground output' USING ERRCODE='23514'; END IF;
    ELSE
        RAISE EXCEPTION 'unknown Soul Core output custody' USING ERRCODE='23514';
    END IF;
END $$;

CREATE FUNCTION game_soul_core_craft_complete(p_transaction UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE receipt game_item_craft_records%ROWTYPE; input JSONB;
        item game_item_instances%ROWTYPE; output JSONB; delivery JSONB; credit JSONB;
        expected BIGINT; changed INT := 0;
BEGIN
    SELECT * INTO receipt FROM game_item_craft_records WHERE transaction_id=p_transaction;
    IF receipt.transaction_id IS NULL OR receipt.soul_core_proof IS NULL
        OR receipt.created_xact_id<>pg_current_xact_id()
    THEN RAISE EXCEPTION 'Soul Core craft requires current receipt' USING ERRCODE='23514'; END IF;
    credit := receipt.soul_core_proof->'credit';
    FOR input IN SELECT value FROM jsonb_array_elements(receipt.soul_core_proof->'inputs') LOOP
        SELECT * INTO item FROM game_item_instances WHERE item_instance_id=(input->>'item')::uuid;
        expected := (input->'before'->>'quantity')::bigint-(input->>'consume')::int
            +CASE WHEN credit->>'item'=input->>'item' THEN 1 ELSE 0 END;
        IF item.item_instance_id IS NULL OR item.quantity<>expected
            OR item.lifecycle<>(CASE WHEN expected=0 THEN 2 ELSE 1 END)
            OR (to_jsonb(item)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
                ((input->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
            OR (((input->>'consume')::int>0 OR credit->>'item'=input->>'item') AND item.last_transaction_id IS DISTINCT FROM p_transaction)
            OR ((input->>'consume')::int=0 AND credit->>'item' IS DISTINCT FROM input->>'item'
                AND to_jsonb(item) IS DISTINCT FROM input->'before')
            OR (expected=0 AND (EXISTS (SELECT 1 FROM game_item_container_entries WHERE item_instance_id=item.item_instance_id)
                OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=item.item_instance_id)))
            OR (expected>0 AND NOT EXISTS (SELECT 1 FROM (
                SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested
                    FROM game_item_container_entries WHERE character_id=receipt.character_id
                UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true
                    FROM game_item_nested_entries
                ) entry WHERE entry.item_instance_id=item.item_instance_id AND entry.world_id=receipt.world_id
                    AND entry.parent_item_instance_id=(input->>'parent')::uuid
                    AND entry.placement_ordinal=(input->>'ordinal')::numeric
                    AND entry.nested=(input->>'nested')::boolean))
        THEN RAISE EXCEPTION 'Soul Core input decrement or retained custody mismatch' USING ERRCODE='23514'; END IF;
        IF (input->>'consume')::int>0 OR credit->>'item'=input->>'item' THEN changed := changed+1; END IF;
    END LOOP;
    IF receipt.recipe='oteryn:soulpit.entry' THEN
        IF changed<>1 OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>1
            OR EXISTS (SELECT 1 FROM game_item_instances WHERE minted_transaction_id=p_transaction)
            OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
            OR NOT EXISTS (SELECT 1 FROM game_soulpit_encounters f
                JOIN game_activity_handoff_groups g USING(handoff_id)
                WHERE f.source_transaction_id=p_transaction AND g.state=2
                    AND f.handoff_id=(receipt.soul_core_proof->'delivery'->>'handoff')::uuid
                    AND f.created_xact_id=pg_current_xact_id())
            OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
                WHERE a.event_id=receipt.event_id AND a.transaction_id=p_transaction
                    AND a.item_instance_id=receipt.source_item_instance_id
                    AND a.occurred_at=receipt.occurred_at AND a.envelope_sha256=receipt.envelope_sha256
                    AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1
                    AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL)
        THEN RAISE EXCEPTION 'Soulpit debit, fight, shared handoff and audit must commit atomically' USING ERRCODE='23514'; END IF;
        RETURN;
    END IF;
    output := receipt.outputs->0;
    delivery := receipt.soul_core_proof->'delivery';
    SELECT * INTO item FROM game_item_instances WHERE item_instance_id=(output->>'item')::uuid;
    IF delivery->>'kind'='stack' THEN
        IF NOT EXISTS (SELECT 1 FROM jsonb_array_elements(receipt.soul_core_proof->'inputs') input
            WHERE input->>'item'=credit->>'item') THEN changed:=changed+1; END IF;
        expected := (credit->'before'->>'quantity')::bigint+1-coalesce((SELECT (input->>'consume')::int
            FROM jsonb_array_elements(receipt.soul_core_proof->'inputs') input WHERE input->>'item'=credit->>'item'),0);
        IF item.item_instance_id IS NULL OR item.quantity<>expected OR item.lifecycle<>1
            OR item.last_transaction_id IS DISTINCT FROM p_transaction
            OR (to_jsonb(item)-ARRAY['quantity','last_transaction_id']) IS DISTINCT FROM
                ((credit->'before')-ARRAY['quantity','last_transaction_id'])
            OR EXISTS (SELECT 1 FROM game_item_instances WHERE minted_transaction_id=p_transaction)
            OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>changed
            OR NOT EXISTS (SELECT 1 FROM (
                SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested
                    FROM game_item_container_entries WHERE character_id=receipt.character_id
                UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries
                ) entry WHERE entry.item_instance_id=item.item_instance_id AND entry.world_id=receipt.world_id
                    AND entry.parent_item_instance_id=(credit->>'parent')::uuid
                    AND entry.placement_ordinal=(credit->>'ordinal')::numeric AND entry.nested=(credit->>'nested')::boolean)
            OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
            OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
                WHERE a.event_id=receipt.event_id AND a.transaction_id=p_transaction
                    AND a.item_instance_id=receipt.source_item_instance_id AND a.occurred_at=receipt.occurred_at
                    AND a.envelope_sha256=receipt.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()
                    AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL)
        THEN RAISE EXCEPTION 'Soul Core stack credit, custody and audit mismatch' USING ERRCODE='23514'; END IF;
        RETURN;
    END IF;
    IF item.item_instance_id IS NULL OR item.world_id<>receipt.world_id OR item.lifecycle<>1
        OR item.quantity<>1 OR item.definition_family<>'Item'
        OR item.definition_production_key IS DISTINCT FROM output->>'key'
        OR item.definition_revision_ref IS DISTINCT FROM output->>'revision'
        OR item.minted_transaction_id IS DISTINCT FROM p_transaction OR item.last_transaction_id IS NOT NULL
        OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>1
        OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>changed
        OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
        OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox audit
            WHERE audit.event_id=receipt.event_id AND audit.transaction_id=p_transaction
                AND audit.item_instance_id=receipt.source_item_instance_id
                AND audit.occurred_at=receipt.occurred_at AND audit.envelope_sha256=receipt.envelope_sha256
                AND audit.created_xact_id=pg_current_xact_id() AND audit.transaction_ordinal=1
                AND audit.transaction_count=1 AND audit.publication_state=1 AND audit.published_at IS NULL)
    THEN RAISE EXCEPTION 'Soul Core mint or durable audit mismatch' USING ERRCODE='23514'; END IF;
    IF delivery->>'kind'='container' THEN
        IF NOT EXISTS (SELECT 1 FROM (
            SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id,false AS nested
                FROM game_item_container_entries WHERE character_id=receipt.character_id
            UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id,true
                FROM game_item_nested_entries
            ) entry WHERE entry.item_instance_id=item.item_instance_id AND entry.world_id=receipt.world_id
                AND entry.parent_item_instance_id=(delivery->>'parent')::uuid
                AND entry.placement_ordinal=(output->>'ordinal')::numeric
                AND entry.placed_transaction_id=p_transaction AND entry.nested=(delivery->>'nested')::boolean)
            OR (SELECT count(*) FROM (
                SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=(delivery->>'parent')::uuid
                UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=(delivery->>'parent')::uuid
                ) entries)>20
            OR game_item_loop_tree_size(receipt.character_id)>500
        THEN RAISE EXCEPTION 'Soul Core output container custody mismatch' USING ERRCODE='23514'; END IF;
    ELSE
        IF NOT EXISTS (SELECT 1 FROM game_item_ground_locations ground
            WHERE ground.item_instance_id=item.item_instance_id AND ground.world_id=receipt.world_id
                AND ground.channel_id=(receipt.soul_core_proof->>'channel')::uuid
                AND ground.runtime_scope_ownership_generation=(receipt.soul_core_proof->>'scope_generation')::numeric
                AND ground.spatial_position=decode(delivery->>'position','hex')
                AND ground.corpse_ref=decode(delivery->>'corpse_ref','hex')
                AND ground.map_revision=delivery->>'map_revision'
                AND ground.content_revision=delivery->>'content_revision'
                AND ground.native_room_placement_context=decode(delivery->>'context','hex'))
        THEN RAISE EXCEPTION 'Soul Core ground custody mismatch' USING ERRCODE='23514'; END IF;
    END IF;
END $$;

-- Prepend only the closed Soul Core proof arms. Every previous owner's body stays intact.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_craft_admit','game_item_craft_complete',
        'game_item_instance_guard','game_item_instance_change_proven',
        'game_item_container_entry_removal_proven','game_item_placement_proven',
        'game_item_nested_entry_proven','game_item_mint_consistency_guard',
        'game_item_ground_insertion_guard'] LOOP
        body := pg_get_functiondef((name||CASE WHEN name='game_item_craft_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
        begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'unexpected craft proof body: %',name; END IF;
        IF name='game_item_craft_admit' THEN
            arm := $arm$
    IF NEW.soul_core_proof IS NOT NULL THEN
        PERFORM game_soul_core_craft_admit(NEW);
        NEW.created_xact_id:=pg_current_xact_id();
        RETURN NEW;
    END IF;
$arm$;
        ELSIF name='game_item_craft_complete' THEN
            arm := $arm$
    IF EXISTS (SELECT 1 FROM game_item_craft_records WHERE transaction_id=p_transaction AND soul_core_proof IS NOT NULL) THEN
        PERFORM game_soul_core_craft_complete(p_transaction); RETURN;
    END IF;
$arm$;
        ELSIF name='game_item_instance_guard' THEN
            arm := $arm$
    IF TG_OP='UPDATE' AND EXISTS (SELECT 1 FROM game_item_craft_records receipt,
        LATERAL jsonb_array_elements(receipt.soul_core_proof->'inputs') input
        WHERE receipt.transaction_id=NEW.last_transaction_id AND receipt.created_xact_id=pg_current_xact_id()
            AND (input->>'item')::uuid=OLD.item_instance_id AND input->'before'=to_jsonb(OLD)
            AND (input->>'consume')::int>0
            AND NEW.quantity=OLD.quantity-(input->>'consume')::int
                +CASE WHEN (receipt.soul_core_proof->'credit'->>'item')::uuid=OLD.item_instance_id THEN 1 ELSE 0 END
            AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
            AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id'])
                IS NOT DISTINCT FROM (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']))
    THEN RETURN NEW; END IF;
    IF TG_OP='UPDATE' AND EXISTS (SELECT 1 FROM game_item_craft_records receipt
        WHERE receipt.transaction_id=NEW.last_transaction_id AND receipt.created_xact_id=pg_current_xact_id()
            AND (receipt.soul_core_proof->'credit'->>'item')::uuid=OLD.item_instance_id
            AND receipt.soul_core_proof->'credit'->'before'=to_jsonb(OLD)
            AND NEW.quantity=OLD.quantity+1 AND NEW.lifecycle=1
            AND (to_jsonb(NEW)-ARRAY['quantity','last_transaction_id'])
                IS NOT DISTINCT FROM (to_jsonb(OLD)-ARRAY['quantity','last_transaction_id']))
    THEN RETURN NEW; END IF;
$arm$;
        ELSIF name='game_item_instance_change_proven' THEN
            arm := $arm$
    IF EXISTS (SELECT 1 FROM game_item_craft_records receipt,
        LATERAL jsonb_array_elements(receipt.soul_core_proof->'inputs') input
        WHERE receipt.transaction_id=NEW.last_transaction_id AND receipt.created_xact_id=pg_current_xact_id()
            AND (input->>'item')::uuid=NEW.item_instance_id AND (input->>'consume')::int>0)
        OR EXISTS (SELECT 1 FROM game_item_craft_records receipt
            WHERE receipt.transaction_id=NEW.last_transaction_id AND receipt.created_xact_id=pg_current_xact_id()
                AND (receipt.soul_core_proof->'credit'->>'item')::uuid=NEW.item_instance_id)
    THEN PERFORM game_soul_core_craft_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
        ELSIF name IN ('game_item_container_entry_removal_proven','game_item_nested_entry_proven') THEN
            arm := $arm$
    IF TG_OP='DELETE' AND EXISTS (SELECT 1 FROM game_item_instances item
        JOIN game_item_craft_records receipt ON receipt.transaction_id=item.last_transaction_id,
        LATERAL jsonb_array_elements(receipt.soul_core_proof->'inputs') input
        WHERE item.item_instance_id=OLD.item_instance_id AND item.quantity=0 AND item.lifecycle=2
            AND receipt.created_xact_id=pg_current_xact_id() AND receipt.world_id=OLD.world_id
            AND (input->>'item')::uuid=OLD.item_instance_id
            AND (input->>'parent')::uuid=OLD.parent_item_instance_id
            AND (input->>'ordinal')::numeric=OLD.placement_ordinal
            AND (input->>'nested')::boolean=(TG_TABLE_NAME='game_item_nested_entries')
            AND (input->'before'->>'quantity')::int=(input->>'consume')::int)
    THEN RETURN NULL; END IF;
$arm$;
            IF name='game_item_nested_entry_proven' THEN
                arm := arm||$arm$
    IF TG_OP='INSERT' AND EXISTS (SELECT 1 FROM game_item_craft_records receipt
        WHERE receipt.transaction_id=NEW.placed_transaction_id AND receipt.soul_core_proof IS NOT NULL
            AND receipt.created_xact_id=pg_current_xact_id()
            AND (receipt.outputs->0->>'item')::uuid=NEW.item_instance_id)
    THEN PERFORM game_soul_core_craft_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$;
            END IF;
        ELSIF name='game_item_placement_proven' THEN
            arm := $arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
        IF EXISTS (SELECT 1 FROM game_item_craft_records receipt
            WHERE receipt.transaction_id=NEW.placed_transaction_id AND receipt.soul_core_proof IS NOT NULL
                AND receipt.created_xact_id=pg_current_xact_id()
                AND (receipt.outputs->0->>'item')::uuid=NEW.item_instance_id)
        THEN PERFORM game_soul_core_craft_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
    END IF;
$arm$;
        ELSIF name='game_item_mint_consistency_guard' THEN
            arm := $arm$
    IF EXISTS (SELECT 1 FROM game_item_craft_records receipt
        WHERE receipt.transaction_id=NEW.minted_transaction_id AND receipt.soul_core_proof IS NOT NULL
            AND receipt.created_xact_id=pg_current_xact_id()
            AND (receipt.outputs->0->>'item')::uuid=NEW.item_instance_id)
    THEN PERFORM game_soul_core_craft_complete(NEW.minted_transaction_id); RETURN NULL; END IF;
$arm$;
        ELSE
            arm := $arm$
    IF EXISTS (SELECT 1 FROM game_item_instances item
        JOIN game_item_craft_records receipt ON receipt.transaction_id=item.minted_transaction_id
        WHERE item.item_instance_id=NEW.item_instance_id AND receipt.soul_core_proof IS NOT NULL
            AND receipt.created_xact_id=pg_current_xact_id()
            AND (receipt.outputs->0->>'item')::uuid=NEW.item_instance_id)
    THEN PERFORM game_soul_core_craft_complete((SELECT minted_transaction_id FROM game_item_instances
        WHERE item_instance_id=NEW.item_instance_id)); RETURN NULL; END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    END LOOP;
END $migration$;

REVOKE ALL ON FUNCTION game_soul_core_craft_admit(game_item_craft_records),
    game_soul_core_craft_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_soul_core_craft_admit(game_item_craft_records),
    game_soul_core_craft_complete(UUID) TO oteryn_game_runtime;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_soul_core_craft_admit(game_item_craft_records) SET search_path=pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_soul_core_craft_complete(uuid) SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;
