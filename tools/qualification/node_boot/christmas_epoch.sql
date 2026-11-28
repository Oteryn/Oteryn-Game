-- Only for the disposable compiled World, before assignment and admission.
-- Start with two historical saves so commit order and daily reward history agree.
WITH initial_epoch AS (
    INSERT INTO game_world_reset_epochs
        (world_id, reset_epoch, target_digest, map_revision, server_digest,
         client_digest, frame_digest, calendar, calendar_revision, activated_at, reset_month)
    SELECT world_id, 0, decode($3, 'hex'), 'sha256:' || $4, server_artifact_digest,
           client_artifact_digest, frame_binding_digest, 'TIBIA_SERVER_SAVE_BERLIN_06', 1,
           TIMESTAMPTZ '2025-12-11 06:00 Europe/Berlin', '2025-12'
    FROM game_content_activations
    WHERE world_id = $1 AND channel_id = $2 AND activation_sequence = 1
    RETURNING *
), christmas_epoch AS (
    INSERT INTO game_world_reset_epochs
        (world_id, reset_epoch, target_digest, map_revision, server_digest,
         client_digest, frame_digest, calendar, calendar_revision, activated_at, reset_month)
    SELECT world_id, 1, target_digest, map_revision, server_digest,
           client_digest, frame_digest, calendar, calendar_revision,
           TIMESTAMPTZ '2025-12-12 06:00 Europe/Berlin', reset_month
    FROM initial_epoch
    RETURNING world_id, reset_epoch
), christmas_calendar AS (
    INSERT INTO game_world_event_epochs
        (world_id, reset_epoch, active, announcement, seasonal_calendar_revision, seasonal_event_keys)
    SELECT world_id, reset_epoch, true, 'World gathering is active until the next server save.',
           1, ARRAY['christmas']
    FROM christmas_epoch
    RETURNING world_id, reset_epoch
)
INSERT INTO game_world_reset_active (world_id, reset_epoch)
SELECT world_id, reset_epoch FROM christmas_calendar
RETURNING reset_epoch::text;
