-- PODIUM-1: durable, per-item house podium display storage.
-- These owner-only storage helpers are not a player mutation boundary. Runtime
-- execution stays closed until a native dispatch owner supplies current session,
-- house-scope and node-incarnation fencing; a supplied CharacterId is not proof.
CREATE TABLE game_house_podium_displays(
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id) ON DELETE CASCADE,
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 podium_kind SMALLINT NOT NULL CHECK(podium_kind IN (1,2)),
 display_kind SMALLINT NOT NULL CHECK(display_kind IN (1,2)),
 display_key TEXT NOT NULL CHECK(display_key~'^oteryn:(outfit|mount)\.[a-z0-9_./-]{1,160}$'),
 display_variant JSONB NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(display_variant)='object' AND octet_length(display_variant::text)<=1024),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 updated_by UUID NOT NULL, updated_at BIGINT NOT NULL,
 CHECK((display_kind=1)=(display_key LIKE 'oteryn:outfit.%')),
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id) ON DELETE CASCADE
);
REVOKE ALL ON game_house_podium_displays FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_podium_set(p_world UUID,p_house TEXT,p_item UUID,p_actor UUID,
 p_kind SMALLINT,p_display_kind SMALLINT,p_display_key TEXT,p_variant JSONB,p_expected_revision NUMERIC)
RETURNS NUMERIC LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a RECORD; i game_item_instances%ROWTYPE; next_revision NUMERIC; guild UUID;
BEGIN
 IF p_world IS NULL OR p_house IS NULL OR p_item IS NULL OR p_actor IS NULL
  OR p_kind IS NULL OR p_kind NOT IN(1,2)
  OR p_display_kind IS NULL OR p_display_kind NOT IN(1,2)
  OR p_display_key IS NULL OR p_variant IS NULL
  OR jsonb_typeof(p_variant)<>'object' OR octet_length(p_variant::text)>1024
  OR p_display_key!~'^oteryn:(outfit|mount)\.[a-z0-9_./-]{1,160}$'
  OR (p_display_kind=1)<>(p_display_key LIKE 'oteryn:outfit.%')
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
 IF i.lifecycle<>1 OR i.definition_family<>'Item' OR i.definition_production_key<>(CASE p_kind WHEN 1 THEN 'oteryn:content.item.podium_of_renown'
      ELSE 'oteryn:content.item.podium_of_vigour' END) THEN RAISE EXCEPTION 'PODIUM_WRONG_ITEM'; END IF;
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

CREATE FUNCTION game_house_podium_load(p_world UUID,p_house TEXT)
RETURNS TABLE(item_instance_id BYTEA,podium_kind SMALLINT,display_kind SMALLINT,display_key TEXT,
 display_variant JSONB,revision TEXT) LANGUAGE sql SECURITY DEFINER STABLE AS $$
 SELECT uuid_send(d.item_instance_id),d.podium_kind,d.display_kind,d.display_key,d.display_variant,d.revision::text
 FROM game_house_podium_displays d JOIN game_item_house_interior_locations h USING(item_instance_id,world_id,house_key)
 JOIN game_item_instances i ON i.item_instance_id=d.item_instance_id AND i.world_id=d.world_id
 WHERE d.world_id=p_world AND d.house_key=p_house AND i.lifecycle=1
  AND i.definition_family='Item'
  AND i.definition_production_key=CASE d.podium_kind WHEN 1 THEN 'oteryn:content.item.podium_of_renown'
   ELSE 'oteryn:content.item.podium_of_vigour' END
 ORDER BY d.item_instance_id LIMIT 2000
$$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_podium_set(UUID,TEXT,UUID,UUID,SMALLINT,SMALLINT,TEXT,JSONB,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_podium_load(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_podium_set(UUID,TEXT,UUID,UUID,SMALLINT,SMALLINT,TEXT,JSONB,NUMERIC),game_house_podium_load(UUID,TEXT) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
