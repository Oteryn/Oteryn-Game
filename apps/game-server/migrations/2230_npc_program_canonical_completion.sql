-- Canonical completion shares the existing NPC response, quest receipt, root
-- revision and item participants. Historical responses remain non-completing.
ALTER TABLE game_quest_npc_program_receipts
    ADD COLUMN completes BOOLEAN NOT NULL DEFAULT false;

DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
    body := pg_get_functiondef('game_quest_npc_program_response_guard()'::regprocedure);
    original := 'AND r.cause_kind=''command'' AND r.created_xact_id=pg_current_xact_id()';
    replacement := original || ' AND r.completes=NEW.completes';
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'unexpected NPC programme response guard';
    END IF;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_npc_program_response_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());

    body := pg_get_functiondef('game_quest_npc_item_complete()'::regprocedure);
    original := '    IF (NEW.plan->''give''=''null''::jsonb)';
    replacement := $completion$
    IF (NEW.plan ? 'completes' AND jsonb_typeof(NEW.plan->'completes') IS DISTINCT FROM 'boolean')
        OR coalesce((NEW.plan->>'completes')::boolean,false) IS DISTINCT FROM q.completes THEN
        RAISE EXCEPTION 'NPC item completion differs from its canonical quest receipt' USING ERRCODE='23514';
    END IF;
    IF (NEW.plan->'give'='null'::jsonb)$completion$;
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'unexpected NPC item completion guard';
    END IF;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_npc_item_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
