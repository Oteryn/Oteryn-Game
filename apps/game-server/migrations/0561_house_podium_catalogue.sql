-- W18: house podiums are the catalogue's real items and gain a fenced runtime writer.
-- Renown 35973/35974 displays an outfit or a mount, vigour 38707 a boss and tenacity
-- 42367/42368 a bestiary creature (Crystal 201189f8 data/items/items.xml:67245-67252,
-- 71414-71416, 75107-75114; game.cpp:4603-4636 opens the outfit window for renown only,
-- game.cpp:11149 resolves vigour's race as a boss). Display kind 3 is a creature key.
-- The 0339 helpers stay owner-only; runtime reaches them only through the session,
-- house-scope and node-holder fence below.
ALTER TABLE game_house_podium_displays
 DROP CONSTRAINT game_house_podium_displays_podium_kind_check,
 DROP CONSTRAINT game_house_podium_displays_display_kind_check,
 DROP CONSTRAINT game_house_podium_displays_display_key_check,
 DROP CONSTRAINT game_house_podium_displays_check,
 ADD CONSTRAINT game_house_podium_displays_podium_kind_check CHECK(podium_kind IN (1,2,3)),
 ADD CONSTRAINT game_house_podium_displays_display_kind_check CHECK(display_kind IN (1,2,3)),
 ADD CONSTRAINT game_house_podium_displays_display_key_check
  CHECK(display_key~'^oteryn:(content\.outfit|content\.mount|creature)\.[a-z0-9_./-]{1,160}$'),
 ADD CONSTRAINT game_house_podium_displays_check CHECK(display_key LIKE
  CASE display_kind WHEN 1 THEN 'oteryn:content.outfit.%' WHEN 2 THEN 'oteryn:content.mount.%' ELSE 'oteryn:creature.%' END),
 ADD CONSTRAINT game_house_podium_displays_matrix_check CHECK((podium_kind=1)=(display_kind IN (1,2)));

CREATE FUNCTION game_house_podium_item_kind(p_key TEXT) RETURNS SMALLINT
LANGUAGE sql IMMUTABLE AS $$
 SELECT CASE p_key
  WHEN 'oteryn:item.tibia.i35973' THEN 1 WHEN 'oteryn:item.tibia.i35974' THEN 1
  WHEN 'oteryn:item.tibia.i38707' THEN 2
  WHEN 'oteryn:item.tibia.i42367' THEN 3 WHEN 'oteryn:item.tibia.i42368' THEN 3
 END::SMALLINT
$$;

CREATE OR REPLACE FUNCTION game_house_podium_set(p_world UUID,p_house TEXT,p_item UUID,p_actor UUID,
 p_kind SMALLINT,p_display_kind SMALLINT,p_display_key TEXT,p_variant JSONB,p_expected_revision NUMERIC)
RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a RECORD; i game_item_instances%ROWTYPE; next_revision NUMERIC; guild UUID;
BEGIN
 IF p_world IS NULL OR p_house IS NULL OR p_item IS NULL OR p_actor IS NULL
  OR p_kind IS NULL OR p_kind NOT IN(1,2,3)
  OR p_display_kind IS NULL OR p_display_kind NOT IN(1,2,3)
  OR p_display_key IS NULL OR p_variant IS NULL
  OR jsonb_typeof(p_variant)<>'object' OR octet_length(p_variant::text)>1024
  OR p_display_key!~'^oteryn:(content\.outfit|content\.mount|creature)\.[a-z0-9_./-]{1,160}$'
  OR (p_display_kind=1 AND p_display_key NOT LIKE 'oteryn:content.outfit.%')
  OR (p_display_kind=2 AND p_display_key NOT LIKE 'oteryn:content.mount.%')
  OR (p_display_kind=3 AND p_display_key NOT LIKE 'oteryn:creature.%')
  OR p_expected_revision IS NULL
  OR NOT(p_expected_revision BETWEEN 0 AND 18446744073709551614)
  OR p_expected_revision<>trunc(p_expected_revision) THEN
  RAISE EXCEPTION 'PODIUM_INVALID_INPUT';
 END IF;
 -- Guild mutations lock guild before property. Preserve that order and reject
 -- an ownership change between discovery and the locked property read.
 SELECT owner_guild_id INTO guild FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF guild IS NOT NULL THEN
  PERFORM 1 FROM game_guilds WHERE guild_id=guild AND world_id=p_world FOR SHARE;
 END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_NOT_HOUSE_ITEM'; END IF;
 IF (SELECT owner_guild_id FROM game_house_properties WHERE world_id=p_world AND house_key=p_house) IS DISTINCT FROM guild THEN
  RAISE EXCEPTION 'PODIUM_CHANGED';
 END IF;
 SELECT q.*,count(*) OVER() AS grant_count INTO a FROM game_house_access(p_world,p_house,p_actor) q;
 IF NOT FOUND OR a.grant_count<>1 OR a.content_fenced IS DISTINCT FROM false
  OR a.role IS NULL OR a.role NOT IN('OWNER','SUBOWNER') THEN
  RAISE EXCEPTION 'PODIUM_NO_ACCESS';
 END IF;
 SELECT x.* INTO i FROM game_item_instances x JOIN game_item_house_interior_locations h
   ON h.item_instance_id=x.item_instance_id AND h.world_id=x.world_id
  WHERE h.item_instance_id=p_item AND h.world_id=p_world AND h.house_key=p_house FOR UPDATE OF x,h;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_NOT_HOUSE_ITEM'; END IF;
 IF i.lifecycle<>1 OR i.definition_family<>'Item'
  OR game_house_podium_item_kind(i.definition_production_key) IS DISTINCT FROM p_kind THEN
  RAISE EXCEPTION 'PODIUM_WRONG_ITEM';
 END IF;
 IF (p_kind=1)<>(p_display_kind IN (1,2)) THEN RAISE EXCEPTION 'PODIUM_WRONG_DISPLAY'; END IF;
 SELECT revision INTO next_revision FROM game_house_podium_displays WHERE item_instance_id=p_item FOR UPDATE;
 IF coalesce(next_revision,0)<>p_expected_revision THEN RAISE EXCEPTION 'PODIUM_CHANGED'; END IF;
 next_revision:=coalesce(next_revision,0)+1;
 INSERT INTO game_house_podium_displays(item_instance_id,world_id,house_key,podium_kind,display_kind,
   display_key,display_variant,revision,updated_by,updated_at)
 VALUES(p_item,p_world,p_house,p_kind,p_display_kind,p_display_key,p_variant,next_revision,p_actor,
   floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
 ON CONFLICT(item_instance_id) DO UPDATE SET world_id=excluded.world_id,house_key=excluded.house_key,
   podium_kind=excluded.podium_kind,display_kind=excluded.display_kind,
   display_key=excluded.display_key,display_variant=excluded.display_variant,revision=excluded.revision,
   updated_by=excluded.updated_by,updated_at=excluded.updated_at;
 RETURN next_revision;
END $$;

CREATE OR REPLACE FUNCTION game_house_podium_load(p_world UUID,p_house TEXT)
RETURNS TABLE(item_instance_id BYTEA,podium_kind SMALLINT,display_kind SMALLINT,display_key TEXT,
 display_variant JSONB,revision TEXT) LANGUAGE sql SECURITY DEFINER STABLE AS $$
 SELECT uuid_send(d.item_instance_id),d.podium_kind,d.display_kind,d.display_key,d.display_variant,d.revision::text
 FROM game_house_podium_displays d JOIN game_item_house_interior_locations h USING(item_instance_id,world_id,house_key)
 JOIN game_item_instances i ON i.item_instance_id=d.item_instance_id AND i.world_id=d.world_id
 WHERE d.world_id=p_world AND d.house_key=p_house AND i.lifecycle=1
  AND i.definition_family='Item'
  AND game_house_podium_item_kind(i.definition_production_key)=d.podium_kind
 ORDER BY d.item_instance_id LIMIT 2000
$$;

-- The current named-House session fence (house_interior.rs `named_house_current`): the
-- session's generation, character lease and house scope, and this node as scope holder.
CREATE FUNCTION game_house_podium_fence(p_session UUID,p_character UUID,p_connection_generation NUMERIC,
 p_lease_generation NUMERIC,p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,
 p_world UUID,p_house TEXT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM 1 FROM game_durability_reconnect_sessions s
  JOIN game_runtime_scope_assignments a ON a.scope_key=game_house_scope_key(s.world_id,s.runtime_scope_house_key)
  JOIN game_character_roots r ON r.character_id=s.character_id
  WHERE s.game_session_id=p_session AND s.character_id=p_character
   AND s.current_generation=p_connection_generation AND s.character_lease_generation=p_lease_generation
   AND s.session_state IN(1,2) AND s.world_id=p_world AND s.runtime_scope_kind=2
   AND s.runtime_scope_world_id=s.world_id AND s.runtime_scope_channel_id IS NULL
   AND s.runtime_scope_house_key=p_house AND s.runtime_scope_instance_id=game_house_scope_instance_id(s.world_id,p_house)
   AND s.scope_ownership_generation=p_scope_generation AND a.scope_kind=2 AND a.world_id=s.world_id
   AND a.house_key=p_house AND a.state=1 AND a.ownership_generation=s.scope_ownership_generation
   AND a.holder_node_id=p_node AND a.holder_registration_revision=p_node_revision
   AND r.account_id=s.account_id AND r.world_id=s.world_id AND r.lifecycle=1
  FOR SHARE OF s,a,r;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_STALE_SESSION'; END IF;
END $$;

-- The podium kind of one item placed in the fenced House, so the owner can check the
-- display against its content before the write; NULL when it is no live podium there.
CREATE FUNCTION game_house_podium_kind_fenced(p_session UUID,p_character UUID,p_connection_generation NUMERIC,
 p_lease_generation NUMERIC,p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,
 p_world UUID,p_house TEXT,p_item UUID) RETURNS SMALLINT LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection_generation,p_lease_generation,
  p_scope_generation,p_node,p_node_revision,p_world,p_house);
 RETURN (SELECT game_house_podium_item_kind(i.definition_production_key)
  FROM game_item_instances i JOIN game_item_house_interior_locations h
   ON h.item_instance_id=i.item_instance_id AND h.world_id=i.world_id
  WHERE i.item_instance_id=p_item AND i.world_id=p_world AND h.house_key=p_house
   AND i.lifecycle=1 AND i.definition_family='Item');
END $$;

CREATE FUNCTION game_house_podium_set_fenced(p_session UUID,p_character UUID,p_connection_generation NUMERIC,
 p_lease_generation NUMERIC,p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,
 p_world UUID,p_house TEXT,p_item UUID,p_display_kind SMALLINT,p_display_key TEXT,p_variant JSONB,
 p_expected_revision NUMERIC) RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE kind SMALLINT;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection_generation,p_lease_generation,
  p_scope_generation,p_node,p_node_revision,p_world,p_house);
 SELECT game_house_podium_item_kind(definition_production_key) INTO kind
  FROM game_item_instances WHERE item_instance_id=p_item AND world_id=p_world;
 IF kind IS NULL THEN RAISE EXCEPTION 'PODIUM_WRONG_ITEM'; END IF;
 RETURN game_house_podium_set(p_world,p_house,p_item,p_character,kind,p_display_kind,p_display_key,
  p_variant,p_expected_revision);
END $$;

CREATE FUNCTION game_house_podium_load_fenced(p_session UUID,p_character UUID,p_connection_generation NUMERIC,
 p_lease_generation NUMERIC,p_scope_generation NUMERIC,p_node UUID,p_node_revision NUMERIC,
 p_world UUID,p_house TEXT)
RETURNS TABLE(item_instance_id BYTEA,podium_kind SMALLINT,display_kind SMALLINT,display_key TEXT,
 display_variant JSONB,revision TEXT) LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection_generation,p_lease_generation,
  p_scope_generation,p_node,p_node_revision,p_world,p_house);
 RETURN QUERY SELECT * FROM game_house_podium_load(p_world,p_house);
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_podium_item_kind(TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_set(UUID,TEXT,UUID,UUID,SMALLINT,SMALLINT,TEXT,JSONB,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_load(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_fence(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_kind_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_set_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID,SMALLINT,TEXT,JSONB,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_load_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_podium_item_kind(TEXT),
 game_house_podium_set(UUID,TEXT,UUID,UUID,SMALLINT,SMALLINT,TEXT,JSONB,NUMERIC),
 game_house_podium_load(UUID,TEXT),
 game_house_podium_fence(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT),
 game_house_podium_kind_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID),
 game_house_podium_set_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID,SMALLINT,TEXT,JSONB,NUMERIC),
 game_house_podium_load_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION
 game_house_podium_kind_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID),
 game_house_podium_set_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID,SMALLINT,TEXT,JSONB,NUMERIC),
 game_house_podium_load_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 TO oteryn_game_runtime;
