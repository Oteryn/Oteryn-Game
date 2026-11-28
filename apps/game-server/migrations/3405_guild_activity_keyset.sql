CREATE INDEX game_guild_event_outbox_activity
    ON game_guild_event_outbox (guild_id, occurred_at DESC, event_id DESC);
