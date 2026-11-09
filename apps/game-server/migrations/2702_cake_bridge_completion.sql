-- Owner 2026-11-08 approves Crystal201189f config.lua:249-350,473-483.
-- Bridge projections replace every original item at each donor tile; immutable
-- base facts retain the original complete stack for event-end restoration.
ALTER TABLE game_terrain_tool_transforms DROP CONSTRAINT game_terrain_tool_cake_source;
ALTER TABLE game_terrain_tool_transforms ADD CONSTRAINT game_terrain_tool_cake_source CHECK(
 (cake_event_year IS NULL AND cake_source_item IS NULL) OR
 (cake_event_year IS NOT NULL AND cake_source_item IS NOT NULL
  AND cake_event_year BETWEEN 2000 AND 9999 AND cake_source_item BETWEEN 0 AND 65535));

ALTER FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB)
 RENAME TO game_cake_project_tower;
REVOKE ALL ON FUNCTION game_cake_project_tower(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) FROM oteryn_game_runtime;
CREATE FUNCTION game_cake_project_channel(p_world UUID,p_channel UUID,p_bundle BYTEA,p_map TEXT,
 p_content BYTEA,p_epoch BIGINT,p_year INTEGER,p_sources JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE tower JSONB; bridge JSONB; cleaned BOOLEAN;
BEGIN
 IF jsonb_typeof(p_sources)<>'array' OR jsonb_array_length(p_sources)>8192 THEN RETURN FALSE; END IF;
 SELECT coalesce(jsonb_agg(s),'[]') INTO tower FROM jsonb_array_elements(p_sources) s WHERE (s->>'bridge_item')::integer=0;
 SELECT coalesce(jsonb_agg(s),'[]') INTO bridge FROM jsonb_array_elements(p_sources) s WHERE (s->>'bridge_item')::integer<>0;
 IF jsonb_array_length(tower)+jsonb_array_length(bridge)<>jsonb_array_length(p_sources)
  OR NOT game_cake_project_tower(p_world,p_channel,p_bundle,p_map,p_content,p_epoch,p_year,tower) THEN RETURN FALSE; END IF;
 SELECT event_end_cleaned_up INTO cleaned FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR SHARE;
 IF NOT FOUND THEN RETURN TRUE; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(bridge) s WHERE
  length(s->>'placement')<>80 OR decode(s->>'placement','hex')<>p_bundle||decode(s->>'spatial','hex')
  OR (s->>'z')::integer<>7 OR (s->>'y')::integer NOT BETWEEN 32052 AND 32059
  OR (s->>'source_item')::integer NOT BETWEEN 0 AND 65535
  OR NOT (((s->>'x')::integer BETWEEN 33301 AND 33305 AND (s->>'bridge_item')::integer=48227)
    OR ((s->>'x')::integer IN(33300,33306) AND (s->>'bridge_item')::integer=12113)))
  OR (SELECT count(DISTINCT s->>'placement') FROM jsonb_array_elements(bridge) s)<>jsonb_array_length(bridge)
 THEN RETURN FALSE; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(bridge) s JOIN game_terrain_tool_transforms t
  ON t.world_id=p_world AND t.channel_id=p_channel AND t.reset_epoch=p_epoch AND t.placement=decode(s->>'placement','hex')
  WHERE t.map_revision<>p_map OR t.content_generation<>p_content
   OR (t.cake_event_year IS NOT NULL AND (t.cake_event_year<>p_year OR t.cake_source_item<>(s->>'source_item')::integer))
   OR (t.cake_event_year IS NULL AND t.state_item_id<>(s->>'source_item')::integer)) THEN RETURN FALSE; END IF;
 WITH sources AS(SELECT s, row_number() OVER(PARTITION BY s->>'x',s->>'y',s->>'z' ORDER BY s->>'spatial') n
  FROM jsonb_array_elements(bridge) s), desired AS(
  SELECT decode(s->>'placement','hex') placement,(s->>'source_item')::integer original,
   CASE WHEN cleaned THEN (s->>'source_item')::integer
    WHEN n=1 THEN (s->>'bridge_item')::integer ELSE 0 END state FROM sources)
 INSERT INTO game_terrain_tool_transforms(world_id,channel_id,placement,reset_epoch,map_revision,content_generation,
  state_item_id,object_revision,decay_states,decay_delays,decay_step,decay_at_unix_ms,cake_event_year,cake_source_item)
 SELECT p_world,p_channel,placement,p_epoch,p_map,p_content,state,1,'{}','{}',0,NULL,p_year,original FROM desired
 ON CONFLICT(world_id,channel_id,placement,reset_epoch) DO UPDATE SET state_item_id=EXCLUDED.state_item_id,
  object_revision=game_terrain_tool_transforms.object_revision+CASE WHEN game_terrain_tool_transforms.state_item_id<>EXCLUDED.state_item_id THEN 1 ELSE 0 END,
  cake_event_year=EXCLUDED.cake_event_year,cake_source_item=EXCLUDED.cake_source_item
 WHERE game_terrain_tool_transforms.state_item_id<>EXCLUDED.state_item_id OR game_terrain_tool_transforms.cake_event_year IS NULL;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) TO oteryn_game_runtime;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- Real movement/NPC owners now hold the current Character/Channel fences.
GRANT EXECUTE ON FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER),
 game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT),
 game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) TO oteryn_game_runtime;
GRANT SELECT ON game_cake_errands TO oteryn_game_runtime;
-- Normal once-claim ownership is permanent per key. Qualify each entitlement by
-- its donor annual occurrence rather than reusing a permanent claim next year.
DO $$ DECLARE body TEXT; old TEXT; BEGIN
 body:=pg_get_functiondef('game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT)'::regprocedure);
 old:=$anchor$ELSE 'oteryn:reward-claim.cake-brutus-cupcake' END;$anchor$;
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'unexpected Cake annual claim predecessor'; END IF;
 EXECUTE replace(body,old,$replacement$ELSE 'oteryn:reward-claim.cake-brutus-cupcake' END || '.' || p_year::text;$replacement$);
END $$;

-- Cake USE has a map source, never a fabricated carried tool. Preserve2031's
-- Item snapshot ownership; admit only the same Type2 bite + terrain CAS here.
DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_terrain_tool_capture_definition()'::regprocedure);
 anchor:=$old$    SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref$old$;
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected Cake map-source receipt predecessor';
 END IF;
 replacement:=$new$    IF NEW.tool_item_instance_id IS NULL AND NEW.transformed_item_id=0
        AND EXISTS (
            SELECT 1 FROM public.game_cake_bite_receipts b
            JOIN public.game_terrain_tool_transforms t
                ON t.world_id=b.world_id AND t.channel_id=NEW.channel_id
                AND t.placement=b.placement AND t.reset_epoch=NEW.reset_epoch
            WHERE b.game_session_id=NEW.game_session_id AND b.command_id=NEW.command_id
                AND b.character_id=NEW.character_id AND b.world_id=NEW.world_id
                AND b.placement=NEW.placement AND b.intent=NEW.intent
                AND b.created_xact_id=pg_current_xact_id()
                AND t.cake_event_year=b.event_year AND t.cake_source_item=b.source_item
                AND t.state_item_id=0 AND t.object_revision=b.before_revision+1
        ) THEN
        RETURN NEW;
    END IF;
    SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref$new$;
 EXECUTE replace(body,anchor,replacement);
END $migration$;
