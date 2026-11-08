-- Crystal 201189f data/libs/functions/boss_lever.lua:63-76,183-218:
-- qualified room policy belongs to its loaded World/content interpretation.
-- A live node retaining another interpretation must not replace that policy.
CREATE TABLE game_boss_room_publications (
 LIKE game_boss_room_catalogue INCLUDING DEFAULTS INCLUDING CONSTRAINTS,
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 origin_frame_digest BYTEA NOT NULL CHECK(octet_length(origin_frame_digest)=32),
 origin_map_digest BYTEA NOT NULL CHECK(octet_length(origin_map_digest)=32),
 server_artifact_digest BYTEA NOT NULL CHECK(octet_length(server_artifact_digest)=32),
 activation_frame_digest BYTEA NOT NULL CHECK(octet_length(activation_frame_digest)=32),
 PRIMARY KEY(world_id,boss_key,content_revision,origin_frame_digest,origin_map_digest,server_artifact_digest),
 FOREIGN KEY(boss_key) REFERENCES game_boss_room_catalogue(boss_key)
);
CREATE TRIGGER game_boss_room_publication_immutable BEFORE UPDATE OR DELETE ON game_boss_room_publications
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_room_publication_no_truncate BEFORE TRUNCATE ON game_boss_room_publications
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Retain the original boss-key FK target as insert-once metadata, never as
-- mutable production policy. Legacy fixture rows without publications retain
-- their existing administrative configuration behavior.
CREATE FUNCTION game_published_boss_metadata_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_boss_room_publications WHERE boss_key=OLD.boss_key) THEN
  RAISE EXCEPTION 'published boss identity metadata is immutable' USING ERRCODE='23514';
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_published_boss_metadata_immutable BEFORE UPDATE OR DELETE ON game_boss_room_catalogue
 FOR EACH ROW EXECUTE FUNCTION game_published_boss_metadata_immutable();
REVOKE ALL ON game_boss_room_publications FROM PUBLIC;
REVOKE ALL ON FUNCTION game_published_boss_metadata_immutable() FROM PUBLIC;
GRANT SELECT,INSERT ON game_boss_room_publications TO oteryn_game_runtime;
GRANT SELECT ON game_boss_room_publications TO oteryn_game_control;
GRANT INSERT ON game_boss_room_catalogue TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_published_boss_metadata_immutable() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
