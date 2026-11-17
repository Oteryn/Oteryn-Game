-- Global TibiaWiki 34108/1092599 unlocks Henricus's blessing at the final report.
-- Crystal questline 23 is that report. Values 24 and 25 are later reward states.
DO $$
DECLARE
    definition text := pg_get_functiondef('game_character_npc_purchase_guard()'::regprocedure);
    previous text := 'q.track_key=''oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline'' AND q.value=25)';
    updated text := 'q.track_key=''oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline'' AND q.value BETWEEN 23 AND 25)';
BEGIN
    IF strpos(definition, previous) = 0
        OR strpos(substr(definition, strpos(definition, previous) + length(previous)), previous) <> 0 THEN
        RAISE EXCEPTION 'expected exactly one Henricus purchase eligibility predicate';
    END IF;
    EXECUTE replace(definition, previous, updated);
END;
$$;
