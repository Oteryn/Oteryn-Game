-- Free quest blessings join the existing Quest/Item reward source and receipt.
-- Keep the complete paid-purchase proof arm; no zero-price purchase is invented.
DO $migration$
DECLARE body TEXT; needle TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_character_blessing_purchase_guard()'::regprocedure);
    needle:='AND r.created_xact_id=pg_current_xact_id()) THEN';
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'unexpected blessing purchase proof body';
    END IF;
    replacement:=$arm$AND r.created_xact_id=pg_current_xact_id())
        AND NOT EXISTS (SELECT 1 FROM game_quest_reward_sources s
            JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key)
            WHERE s.character_id=NEW.character_id
                AND s.created_xact_id=pg_current_xact_id()
                AND r.created_xact_id=pg_current_xact_id() AND r.cause_kind='command'
                AND s.plan->>'blessing'='oteryn:service.blessing.' || substr(NEW.blessing_key,17)
                AND NEW.provenance='quest:' || s.cause_id::text || ':' || s.cause_ordinal::text
                    || ':' || encode(sha256(convert_to(s.transition_key,'UTF8')),'hex')) THEN$arm$;
    EXECUTE replace(body,needle,replacement);
    EXECUTE format('ALTER FUNCTION game_character_blessing_purchase_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE 'REVOKE ALL ON FUNCTION game_character_blessing_purchase_guard() FROM PUBLIC';

    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    needle:='RETURN NULL;';
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'unexpected quest reward proof body';
    END IF;
    replacement:=$arm$IF NEW.plan->>'blessing' IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_character_blessings b WHERE b.character_id=NEW.character_id
            AND NEW.plan->>'blessing'='oteryn:service.blessing.' || substr(b.blessing_key,17)) THEN
        RAISE EXCEPTION 'quest reward lacks the authored held blessing' USING ERRCODE='23514';
    END IF;
    RETURN NULL;$arm$;
    EXECUTE replace(body,needle,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE 'REVOKE ALL ON FUNCTION game_quest_reward_complete() FROM PUBLIC';
END $migration$;
