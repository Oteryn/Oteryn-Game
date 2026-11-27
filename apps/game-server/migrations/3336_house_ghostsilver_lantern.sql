-- Crystal 201189f8: data-global/scripts/quests/forgotten_knowledge/actions_lantern.lua:1-4.
-- Admit both lantern transitions on the existing House guard's function identity.
DO $ghostsilver_house_pair$
DECLARE
    body TEXT := pg_get_functiondef('game_item_action_house_pair(text,text)'::regprocedure);
    anchor CONSTANT TEXT := 'SELECT EXISTS(SELECT 1 FROM (VALUES';
BEGIN
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'House Item transformation pair layout changed';
    END IF;
    EXECUTE replace(body, anchor, $pairs$
 SELECT (p_before,p_after) IN (
    ('oteryn:item.tibia.i23734','oteryn:item.tibia.i23738'),
    ('oteryn:item.tibia.i23738','oteryn:item.tibia.i23734')
 ) OR EXISTS(SELECT 1 FROM (VALUES$pairs$);
END $ghostsilver_house_pair$;
