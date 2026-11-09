-- Pinned reset definitions are sparse in otherwise high-volume event history.
-- Actor-local lookup must not scan every objective receipt from every save.
CREATE INDEX game_event_quest_restart_profiles
    ON game_character_event_quest_progress
        (character_id,quest_key,committed_character_revision DESC)
    WHERE transition_key LIKE 'oteryn:quest-transition/save-event-restart/%';
