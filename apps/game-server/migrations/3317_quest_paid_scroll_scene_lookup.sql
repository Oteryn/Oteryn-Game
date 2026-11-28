-- The immutable offering proof contains the source and exact resulting scene.
-- A rebuilt Channel finds that proof without scanning Characters or unrelated burns.
CREATE INDEX game_quest_paid_scroll_scene_lookup ON game_quest_callback_consumptions (
    (before_ground->>'world_id'),
    (before_ground->>'channel_id'),
    (before_ground->>'map_revision'),
    (before_ground->>'native_room_placement_context'),
    transaction_id
)
WHERE transition_key='oteryn:quest-callback/dreamer-dread-lord-scroll'
  AND location_kind='ground';
