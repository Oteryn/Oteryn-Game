-- W05A: Global's server-save day begins at 06:00 Europe/Berlin, including DST.
ALTER TABLE game_world_reset_epochs
    DROP CONSTRAINT IF EXISTS game_world_reset_epochs_calendar_check;

ALTER TABLE game_world_reset_epochs DISABLE TRIGGER game_world_reset_epochs_immutable;

UPDATE game_world_reset_epochs
SET calendar = 'TIBIA_SERVER_SAVE_BERLIN_06',
    reset_month = to_char((activated_at AT TIME ZONE 'Europe/Berlin') - interval '6 hours', 'YYYY-MM')
WHERE calendar = 'TIBIA_SERVER_SAVE_BERLIN_10';

ALTER TABLE game_world_reset_epochs ENABLE TRIGGER game_world_reset_epochs_immutable;

ALTER TABLE game_world_reset_epochs
    ADD CONSTRAINT game_world_reset_epochs_calendar_check
    CHECK (calendar = 'TIBIA_SERVER_SAVE_BERLIN_06');

-- Preserve the accepted owner-only functions, changing only their persisted calendar value.
DO $$
DECLARE
    procedure REGPROCEDURE;
    definition TEXT;
BEGIN
    FOREACH procedure IN ARRAY ARRAY[
        'game_world_reset_initialize(uuid,bytea,text,bytea,bytea,bytea,text,numeric)'::regprocedure,
        'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure,
        'game_daily_reward_reset_window()'::regprocedure
    ] LOOP
        SELECT pg_get_functiondef(procedure) INTO definition;
        definition := replace(definition, 'TIBIA_SERVER_SAVE_BERLIN_10', 'TIBIA_SERVER_SAVE_BERLIN_06');
        definition := replace(definition, 'interval ''10 hours''', 'interval ''6 hours''');
        EXECUTE definition;
    END LOOP;
END $$;

REVOKE ALL ON game_world_reset_epochs, game_world_reset_active,
    game_world_reset_jobs, game_world_reset_channels FROM PUBLIC;
GRANT SELECT ON game_world_reset_epochs, game_world_reset_active,
    game_world_reset_jobs, game_world_reset_channels
    TO oteryn_game_runtime, oteryn_game_control;
