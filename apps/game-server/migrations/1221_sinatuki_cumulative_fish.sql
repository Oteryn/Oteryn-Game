-- Sinatuki's pinned wiki behaviour: cumulative mixed fish, then a later whole
-- stack for five shards. Keep craft custody, audit, recovery and deferred proofs.
-- Forward rollback: disable the adapter; retain all immutable craft receipts.
CREATE INDEX game_item_craft_sinatuki_character ON game_item_craft_records(character_id,transaction_id)
    WHERE recipe='oteryn:npc-quest-exchange.sinatuki';

DO $migration$
DECLARE expression TEXT; body TEXT; before TEXT; after TEXT;
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-quest-exchange.sinatuki'' AND outputs=''[]''::jsonb))',expression);

    SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_npc_exchange_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_npc_exchange_check;
    EXECUTE format($constraint$ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_npc_exchange_check CHECK ((%s) OR (
        recipe='oteryn:npc-quest-exchange.sinatuki' AND consume_source
        AND ingredient_item_instance_id IS NULL
        AND jsonb_typeof(npc_exchange)='object'
        AND npc_exchange ?& ARRAY['npc_key','npc_identity','npc_generation','catalogue_revision','conversation_revision']
        AND npc_exchange->>'npc_key'='oteryn:npc.sinatuki'
        AND jsonb_typeof(npc_exchange->'npc_identity')='array'
        AND jsonb_array_length(npc_exchange->'npc_identity')=16
        AND (npc_exchange->>'npc_generation')::numeric BETWEEN 1 AND 18446744073709551615
        AND (npc_exchange->>'catalogue_revision')::numeric BETWEEN 1 AND 18446744073709551615
        AND (npc_exchange->>'conversation_revision')::numeric BETWEEN 1 AND 18446744073709551615
        AND before_item->>'definition_family'='Item'
        AND before_item->>'definition_revision_ref'='definition-r1'
        AND before_item->>'definition_production_key' IN ('oteryn:item.tibia.i7158','oteryn:item.tibia.i7159')
        AND (before_item->>'quantity')::bigint BETWEEN 1 AND 100
        AND (outputs='[]'::jsonb OR (jsonb_array_length(outputs)=1
            AND outputs->0->>'key'='oteryn:item.tibia.i7290'
            AND outputs->0->>'revision'='definition-r1' AND outputs->0->>'count'='5'))
    ) IS TRUE)$constraint$,expression);

    -- Add only this recipe to the existing whole-stack retirement arms.
    FOREACH before IN ARRAY ARRAY['game_item_craft_admit()', 'game_item_craft_complete(uuid)', 'game_item_instance_guard()'] LOOP
        body := pg_get_functiondef(before::regprocedure);
        IF strpos(body,'''oteryn:event-container.39710''')=0 THEN
            RAISE EXCEPTION 'missing whole-stack guard in %',before;
        END IF;
        EXECUTE replace(body,'''oteryn:event-container.39710''','''oteryn:event-container.39710'',''oteryn:npc-quest-exchange.sinatuki''');
    END LOOP;
END $migration$;

CREATE FUNCTION game_sinatuki_craft_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE eaten BIGINT; rewarded BOOLEAN;
BEGIN
    IF NEW.recipe<>'oteryn:npc-quest-exchange.sinatuki' THEN RETURN NEW; END IF;
    -- Every item/quest writer serializes on the same Character root.
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
    IF NOT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=NEW.character_id
        AND quest_key='oteryn:quest.the_ice_islands_quest'
        AND track_key='oteryn:quest-progress/quest/u8_0/barbarian_test/questline' AND value>=3)
       OR NOT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=NEW.character_id
        AND quest_key='oteryn:quest.the_ice_islands_quest'
        AND track_key='oteryn:quest-progress/quest/u8_0/the_ice_islands/questline' AND value>=5)
    THEN RAISE EXCEPTION 'Sinatuki access prerequisites' USING ERRCODE='23514'; END IF;
    SELECT coalesce(sum((before_item->>'quantity')::bigint),0),coalesce(bool_or(jsonb_array_length(outputs)>0),false)
        INTO eaten,rewarded FROM (SELECT before_item,outputs FROM game_item_craft_records
            WHERE character_id=NEW.character_id AND recipe=NEW.recipe ORDER BY transaction_id LIMIT 101) history;
    IF rewarded OR (jsonb_array_length(NEW.outputs)>0) IS DISTINCT FROM (eaten>=99)
    THEN RAISE EXCEPTION 'Sinatuki cumulative/final reward mismatch' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_sinatuki_craft_admit BEFORE INSERT ON game_item_craft_records
    FOR EACH ROW EXECUTE FUNCTION game_sinatuki_craft_admit();
REVOKE ALL ON FUNCTION game_sinatuki_craft_admit() FROM PUBLIC;
