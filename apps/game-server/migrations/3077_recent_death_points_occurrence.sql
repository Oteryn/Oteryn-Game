-- Serve Recent Deaths attribution probes and contributor lookups by receipt.
CREATE INDEX game_character_unjustified_points_occurrence
    ON game_character_unjustified_points (death_occurrence_id, character_id);
