-- Compose the existing Quest source and Text/MINT owners. Preserve every
-- evolved capacity, fee, participant and carried-input proof in the guard.
DO $migration$
DECLARE
    definition TEXT;
    anchor TEXT := $anchor$AND m.quantity=(NEW.plan #>> '{give,count}')::bigint$anchor$;
BEGIN
    definition := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(definition)-length(replace(definition,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'literal quest reward text requires exact MINT guard anchor';
    END IF;
    EXECUTE replace(definition, anchor, anchor ||
        $proof$ AND m.root_text IS NOT DISTINCT FROM (NEW.plan #>> '{give,text}')$proof$);
END
$migration$;
