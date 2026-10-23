-- GUILD-WIRE-1: the private roster query pages in (rank_level,character_id) order.
-- Existing membership cleanup keeps its Character-only cursor and index unchanged.
CREATE INDEX IF NOT EXISTS game_guild_roster_keyset
 ON game_guild_members(guild_id,rank_level,character_id);
