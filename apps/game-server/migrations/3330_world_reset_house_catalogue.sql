-- The database owner imports the exact catalogue emitted by
-- oteryn-game-ops world-reset house-catalogue --bundle <file> --digest <pin>.
-- Bind the exported JSON as a parameter for this owner-only import:
-- INSERT INTO game_world_reset_bundle_house_catalogues
-- SELECT decode(document->>'bundle_digest', 'hex'), document->'house_tiles'
-- FROM (SELECT $1::jsonb AS document) evidence;
-- Entries are [native_floor, x, y, source_house_id] in bundle traversal order.
-- Keep the evidence immutable and separate from caller-supplied reset jobs.
CREATE TABLE game_world_reset_bundle_house_catalogues (
    bundle_digest BYTEA PRIMARY KEY CHECK (octet_length(bundle_digest) = 32),
    house_tiles JSONB NOT NULL CHECK (
        jsonb_typeof(house_tiles) = 'array'
        AND jsonb_array_length(house_tiles) <= 33554432
    )
);
REVOKE ALL ON game_world_reset_bundle_house_catalogues FROM PUBLIC;
CREATE TRIGGER game_world_reset_bundle_house_catalogues_immutable
    BEFORE UPDATE OR DELETE ON game_world_reset_bundle_house_catalogues
    FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

-- An in-place Server Save uses the same authenticated bundle as its predecessor.
-- Native entry resets retain their explicit empty-catalogue requirement.
CREATE FUNCTION game_world_reset_house_catalogue_matches(
    p_world UUID, p_epoch NUMERIC, p_target BYTEA, p_map TEXT,
    p_server BYTEA, p_tiles JSONB
) RETURNS BOOLEAN LANGUAGE sql VOLATILE STRICT AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_world_reset_epochs e
        WHERE e.world_id = p_world AND e.reset_epoch = p_epoch
          AND CASE WHEN p_target = p_server THEN
              e.target_digest = e.server_digest AND p_tiles = '[]'::jsonb
              AND NOT EXISTS (
                  SELECT 1 FROM game_world_reset_bundle_house_catalogues c
                  WHERE c.bundle_digest = CASE WHEN p_map ~ '^sha256:[0-9a-f]{64}$'
                      THEN decode(substr(p_map, 8), 'hex') ELSE NULL END
              )
          ELSE
              e.target_digest = p_target
              AND e.map_revision = p_map
              AND p_map = 'sha256:' || encode(p_target, 'hex')
              AND EXISTS (
                  SELECT 1 FROM game_world_reset_bundle_house_catalogues c
                  WHERE c.bundle_digest = p_target AND c.house_tiles = p_tiles
                  FOR SHARE
              )
          END
    )
$$;
REVOKE ALL ON FUNCTION game_world_reset_house_catalogue_matches(UUID,NUMERIC,BYTEA,TEXT,BYTEA,JSONB) FROM PUBLIC;

-- Update the installed bodies in place so existing owners, privileges,
-- search paths, drain fences and the 3328 Server Save policy stay intact.
DO $migration$
DECLARE
    publish REGPROCEDURE := 'game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)';
    activate REGPROCEDURE := 'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)';
    definition TEXT;
    target_check TEXT := $old$p_target IS DISTINCT FROM p_server OR p_revision$old$;
    publish_check TEXT := $old$p_house_tiles IS DISTINCT FROM '[]'::jsonb OR EXISTS$old$;
    activate_check TEXT := $old$j.target_house_tiles<>'[]'::jsonb OR EXISTS$old$;
    replay_check TEXT := $old$IF j.phase='ACTIVATED' THEN$old$;
BEGIN
    EXECUTE format(
        'ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',
        'game_world_reset_house_catalogue_matches(uuid,numeric,bytea,text,bytea,jsonb)'::regprocedure,
        current_schema()
    );
    SELECT pg_get_functiondef(publish) INTO definition;
    IF strpos(definition, target_check) = 0 OR strpos(definition, publish_check) = 0 THEN
        RAISE EXCEPTION 'unexpected World reset publication body';
    END IF;
    definition := replace(definition, target_check, 'p_target IS NULL OR p_revision');
    EXECUTE replace(definition, publish_check,
        'NOT COALESCE(game_world_reset_house_catalogue_matches(p_world,p_epoch,p_target,p_map,p_server,p_house_tiles), FALSE) OR EXISTS');

    SELECT pg_get_functiondef(activate) INTO definition;
    IF strpos(definition, activate_check) = 0 OR strpos(definition, replay_check) = 0 THEN
        RAISE EXCEPTION 'unexpected World reset activation body';
    END IF;
    definition := replace(definition, replay_check, $new$
 IF NOT COALESCE(game_world_reset_house_catalogue_matches(p_world,p_epoch,p_target,p_map,p_server,j.target_house_tiles), FALSE) THEN
  RAISE EXCEPTION 'World reset House qualification unavailable' USING ERRCODE='23514';
 END IF;
 IF j.phase='ACTIVATED' THEN$new$);
    EXECUTE replace(definition, activate_check, 'EXISTS');
END
$migration$;
