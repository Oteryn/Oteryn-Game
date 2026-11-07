-- Source-derived per-Character exhaustion stays with the existing terrain receipt owner.
-- Successful Gobbler actions alone yield12065; retries retain their original receipt.
CREATE INDEX game_terrain_tool_gobbler_character_time
    ON game_terrain_tool_receipts(character_id, committed_at DESC)
    WHERE transformed_item_id = 12065;
