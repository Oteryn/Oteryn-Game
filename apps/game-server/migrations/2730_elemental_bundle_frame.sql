-- Elemental source maps use the bundle frame, while the activation issuance
-- retains the independently qualified entry-room frame. This helper recognizes
-- exactly those two representations; it grants no map/reset/owner authority.
-- The bundle encoding matches BundleWorld::frame_binding_digest byte for byte.
CREATE FUNCTION game_elemental_bundle_frame(p_world UUID,p_map_revision TEXT) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog AS $$
 SELECT sha256(convert_to('OTERYN_WORLD_BUNDLE_FRAME_BINDING/v1','UTF8')||decode('00','hex')
   ||uuid_send(p_world)
   ||int8send(octet_length(convert_to('oteryn:frame/world-bundle-v3','UTF8'))::bigint)
   ||convert_to('oteryn:frame/world-bundle-v3','UTF8')
   ||int8send(octet_length(convert_to(p_map_revision,'UTF8'))::bigint)
   ||convert_to(p_map_revision,'UTF8'));
$$;
CREATE FUNCTION game_elemental_placement_frame_matches(
 p_world UUID,p_map_revision TEXT,p_placement BYTEA,p_activation_frame BYTEA
) RETURNS BOOLEAN LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT octet_length(p_placement)=32 AND octet_length(p_activation_frame)=32
   AND p_map_revision ~ '^sha256:[0-9a-f]{64}$'
   AND (p_placement=p_activation_frame
     OR p_placement=game_elemental_bundle_frame(p_world,p_map_revision));
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_elemental_placement_frame_matches(UUID,TEXT,BYTEA,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_elemental_bundle_frame(UUID,TEXT),game_elemental_placement_frame_matches(UUID,TEXT,BYTEA,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_elemental_bundle_frame(UUID,TEXT),game_elemental_placement_frame_matches(UUID,TEXT,BYTEA,BYTEA) TO oteryn_game_runtime,oteryn_game_control;

-- Replace only the frame predicates in the fully migrated definitions. In
-- particular, retain 2408's shared channel field budget and every later guard.
DO $migration$
DECLARE name TEXT; predecessor TEXT; successor TEXT; body TEXT;
BEGIN
 FOR name,predecessor,successor IN SELECT * FROM (VALUES
  ('game_elemental_arrival_admit()',$before$g.native_room_placement_context=a.frame_binding_digest$before$,$after$game_elemental_placement_frame_matches(m.world_id,g.map_revision,g.native_room_placement_context,a.frame_binding_digest)$after$),
  ('game_elemental_energy_field_admit()',$before$NEW.placement_context:=a.frame_binding_digest$before$,$after$NEW.placement_context:=game_elemental_bundle_frame(r.world_id,t.map_revision)$after$),
  ('game_elemental_energy_adoption_stamp()',$before$r.placement_context=c.frame_binding_digest$before$,$after$(r.placement_context=c.frame_binding_digest OR EXISTS(
        SELECT 1 FROM game_item_craft_records craft
        JOIN game_terrain_tool_receipts source ON source.game_session_id=craft.game_session_id AND source.command_id=craft.command_id
        JOIN game_terrain_tool_transforms terrain ON terrain.world_id=source.world_id AND terrain.channel_id=source.channel_id
          AND terrain.placement=source.placement AND terrain.reset_epoch=source.reset_epoch
        WHERE craft.transaction_id=r.craft_transaction_id AND source.world_id=r.world_id AND source.channel_id=r.channel_id
          AND source.reset_epoch=r.reset_epoch
          AND r.map_revision='sha256:'||encode(sha256(convert_to(terrain.map_revision,'UTF8')),'hex')
          AND r.placement_context=game_elemental_bundle_frame(r.world_id,terrain.map_revision)))$after$)
 ) AS patches(name,predecessor,successor) LOOP
  body:=pg_get_functiondef(format('%I.%s',current_schema(),name)::regprocedure);
  IF length(body)-length(replace(body,predecessor,''))<>length(predecessor)
  THEN RAISE EXCEPTION 'elemental frame predecessor must occur exactly once: %',name; END IF;
  EXECUTE replace(body,predecessor,successor);
 END LOOP;
END $migration$;
