-- Content-qualified publication is performed by ActivityHandoffOwner under a
-- current node incarnation; rejected templates never reach this writer.
GRANT INSERT, UPDATE ON game_boss_room_catalogue TO oteryn_game_runtime;
CREATE UNIQUE INDEX game_boss_room_template_content ON game_boss_room_catalogue(room_template_key, content_revision);
