-- Migration 3330 installed a permanent terminal stage for the self-decaying dread body.
DO $dread$
DECLARE
    body text := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    old_case constant text := 'WHEN p_key=''oteryn:item.tibia.i52559'' THEN ''{0}''::BIGINT[]';
BEGIN
    IF strpos(body, old_case) = 0 THEN
        RAISE EXCEPTION 'Expected dread corpse lifetime from migration 3330';
    END IF;
    EXECUTE replace(body, old_case, replace(old_case, '{0}', '{300000}'));
END $dread$;
