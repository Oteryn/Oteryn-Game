-- GMOTD: enforce new edits on the immutable operation journal, preserving historical rows.
ALTER TABLE game_guild_operations
    DROP CONSTRAINT game_guild_operations_check3,
    ADD CONSTRAINT game_guild_operations_text_edit_check CHECK (
        CASE
            WHEN action IN (7, 8) AND result = 0 THEN
                expected_revision IS NOT NULL AND expected_revision = previous_revision
                AND expected_revision >= 1 AND edit_text IS NOT NULL
                AND CASE
                    WHEN action = 7 THEN char_length(edit_text) <= 29
                    ELSE octet_length(edit_text) <= 255
                        AND (edit_text = '' OR (
                            edit_text COLLATE "C" !~ U&'[\0001-\001F\007F-\009F]'
                            AND btrim(edit_text, U&' \00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000') <> ''
                        ))
                END
            WHEN action IN (9, 10, 11) AND result = 0 THEN
                expected_revision IS NOT NULL AND expected_revision = previous_revision
                AND expected_revision >= 1 AND edit_text IS NULL
            ELSE expected_revision IS NULL AND edit_text IS NULL
        END
    ) NOT VALID;

CREATE FUNCTION game_guild_motd_require_leader() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_guild_members
        WHERE guild_id = NEW.guild_id AND character_id = NEW.actor AND rank_level = 1
    ) THEN
        RAISE EXCEPTION 'Guild MOTD edits require the current Leader' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

-- Extend the operation guard chain at both insertion and deferred verification.
CREATE TRIGGER game_guild_motd_allowed
BEFORE INSERT ON game_guild_operations FOR EACH ROW
WHEN (NEW.action = 8 AND NEW.result = 0)
EXECUTE FUNCTION game_guild_motd_require_leader();

CREATE CONSTRAINT TRIGGER game_guild_motd_proven
AFTER INSERT ON game_guild_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
WHEN (NEW.action = 8 AND NEW.result = 0)
EXECUTE FUNCTION game_guild_motd_require_leader();

REVOKE ALL ON FUNCTION game_guild_motd_require_leader() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_guild_motd_require_leader() TO oteryn_game_runtime;
