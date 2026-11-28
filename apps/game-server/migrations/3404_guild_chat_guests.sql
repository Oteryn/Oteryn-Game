CREATE TABLE game_guild_chat_guests (
    game_session_id uuid NOT NULL REFERENCES game_durability_reconnect_sessions ON DELETE CASCADE,
    guild_id uuid NOT NULL REFERENCES game_guilds,
    granted_at_us bigint NOT NULL CHECK (granted_at_us > 0),
    PRIMARY KEY (game_session_id, guild_id)
);
CREATE INDEX game_guild_chat_guests_by_guild ON game_guild_chat_guests (guild_id);
REVOKE ALL ON game_guild_chat_guests FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_guild_chat_guests TO oteryn_game_runtime;
GRANT SELECT ON game_guild_chat_guests TO oteryn_game_control;

CREATE FUNCTION game_guild_chat_guests_end() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
BEGIN
    DELETE FROM public.game_guild_chat_guests WHERE game_session_id = NEW.game_session_id;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION game_guild_chat_guests_end() FROM PUBLIC;
CREATE TRIGGER game_guild_chat_guests_end
AFTER UPDATE OF session_state ON game_durability_reconnect_sessions
FOR EACH ROW WHEN (NEW.session_state = 3 AND OLD.session_state <> 3)
EXECUTE FUNCTION game_guild_chat_guests_end();
