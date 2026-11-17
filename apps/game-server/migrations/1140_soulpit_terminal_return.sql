-- Crystal 201189f data-global/lib/others/soulpit.lua:132-140 and
-- data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106: terminal
-- encounter cleanup returns survivors after mastery is committed.
-- Shared reason 52 = next free reason 12 + SOULPIT lane offset 40.
ALTER TABLE game_activity_runtime_returns
    DROP CONSTRAINT nx1_return_reason,
    DROP CONSTRAINT nx1_return_server_command,
    ADD CONSTRAINT nx1_return_reason CHECK (reason BETWEEN 1 AND 11 OR reason=52),
    ADD CONSTRAINT nx1_return_server_command CHECK
        (command_id IS NOT NULL OR reason IN (2,3,4,5,6,7,8,9,10,11,52)),
    ADD CONSTRAINT soulpit_terminal_server_only CHECK (reason<>52 OR command_id IS NULL);

CREATE FUNCTION game_soulpit_terminal_return_proven() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.reason=52 AND NOT EXISTS (
        SELECT 1 FROM game_soulpit_encounters f
        JOIN game_soulpit_participants p USING(handoff_id)
        WHERE f.handoff_id=NEW.handoff_id AND p.character_id=NEW.character_id
          AND f.terminal
    ) THEN
        RAISE EXCEPTION 'Soulpit return requires a committed terminal encounter'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_soulpit_terminal_return_proven() SET search_path=%I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_soulpit_terminal_return_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_soulpit_terminal_return_proven
AFTER INSERT ON game_activity_runtime_returns DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_soulpit_terminal_return_proven();
