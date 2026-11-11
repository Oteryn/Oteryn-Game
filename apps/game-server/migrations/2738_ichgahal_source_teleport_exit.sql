-- Crystal201189f rotten_ichgahal.lua selects33004,32343,15 as its raw exit.
-- The pinned OTBM Teleport34111 sends that exact tile to34065,31717,15.
-- Keep the raw tile closed and persist the same qualified arrival as Content.
-- Patch only this recipe in the current function; retain every authority fence,
-- SECURITY DEFINER/search_path setting and existing function ACL. Ordinary
-- authenticated activation updates its catalogue row; no live rows are rewritten.
DO $migration$
DECLARE
    definition TEXT;
    predecessor TEXT := $before$'{"x":33004,"y":32343,"floor":15}'::jsonb,'{"x":33008,"y":32333,"floor":15}'::jsonb,'oteryn:room.rotten-blood-ichgahal-source'$before$;
    successor TEXT := $after$'{"x":34065,"y":31717,"floor":15}'::jsonb,'{"x":33008,"y":32333,"floor":15}'::jsonb,'oteryn:room.rotten-blood-ichgahal-source'$after$;
    predecessor_count INTEGER;
    successor_count INTEGER;
BEGIN
    definition := pg_get_functiondef(format(
        '%I.game_rotten_blood_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT)',
        current_schema()
    )::regprocedure);
    predecessor_count := (length(definition) - length(replace(definition, predecessor, '')))
        / length(predecessor);
    successor_count := (length(definition) - length(replace(definition, successor, '')))
        / length(successor);
    IF predecessor_count = 1 AND successor_count = 0 THEN
        EXECUTE replace(definition, predecessor, successor);
    ELSIF predecessor_count <> 0 OR successor_count <> 1 THEN
        RAISE EXCEPTION 'Ichgahal exit recipe must have one exact predecessor or successor';
    END IF;
END $migration$;
