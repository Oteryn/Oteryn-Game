-- Only pre-occurrence-store calendars may take the one-time legacy upgrade path.
ALTER TABLE game_world_change_calendar
    ADD COLUMN decision_required boolean NOT NULL DEFAULT false;
GRANT UPDATE(decision_required) ON game_world_change_calendar TO oteryn_game_runtime;
