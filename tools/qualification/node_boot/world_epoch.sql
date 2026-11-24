-- Disposable Control setup, using the compiled World pin or the issued native entry.
-- Epoch zero is never created by the runtime or by schema installation.
SELECT game_world_reset_initialize(
    $1::uuid, coalesce(decode($3, 'hex'), server_artifact_digest), 'sha256:' || $4,
    server_artifact_digest, client_artifact_digest, frame_binding_digest,
    'TIBIA_SERVER_SAVE_BERLIN_06', 1
)
FROM game_content_activations
WHERE world_id = $1::uuid AND channel_id = $2::uuid AND activation_sequence = 1;
