-- Current-Tibia Cake food: exactly240s through the existing condition snapshot.
CREATE TABLE game_seasonal_cake_food_uses (
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL,world_id UUID NOT NULL,event_start BIGINT NOT NULL,
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 actor_identity BYTEA NOT NULL CHECK(octet_length(actor_identity)=16),
 movement_revision NUMERIC(20,0) NOT NULL,state_revision NUMERIC(20,0) NOT NULL,
 character_lease_generation NUMERIC(20,0) NOT NULL,
 conditions_before JSONB NOT NULL,conditions_after JSONB NOT NULL,
 conditions_sha256 BYTEA NOT NULL CHECK(octet_length(conditions_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(game_session_id,command_id),
 FOREIGN KEY(game_session_id,command_id) REFERENCES game_seasonal_cake_bite_receipts
);
REVOKE ALL ON game_seasonal_cake_food_uses FROM PUBLIC;
GRANT SELECT,INSERT ON game_seasonal_cake_food_uses TO oteryn_game_runtime;
GRANT SELECT ON game_seasonal_cake_food_uses TO oteryn_game_control;
CREATE TRIGGER game_seasonal_cake_food_uses_immutable BEFORE UPDATE OR DELETE ON game_seasonal_cake_food_uses
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_cake_food_conditions_valid(p_before JSONB,p_after JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE before_food JSONB; after_food JSONB; previous NUMERIC; following NUMERIC; before_other JSONB; after_other JSONB;
BEGIN
 IF (jsonb_typeof(p_before)='object' AND p_before->'schema'='1'::jsonb
  AND jsonb_typeof(p_after)='object' AND p_after->'schema'='1'::jsonb
  AND jsonb_typeof(p_before->'conditions')='array' AND jsonb_typeof(p_after->'conditions')='array'
  AND jsonb_array_length(p_before->'conditions')<=32 AND jsonb_array_length(p_after->'conditions')<=32
  AND octet_length(p_before::text)<=32768 AND octet_length(p_after::text)<=32768) IS DISTINCT FROM TRUE THEN RETURN FALSE; END IF;
 IF (p_before-'conditions') IS DISTINCT FROM (p_after-'conditions') THEN RETURN FALSE; END IF;
 IF (SELECT count(*) FROM jsonb_array_elements(p_before->'conditions') v WHERE v->'definition'->'values' ? 'FoodRegeneration')>1
  OR (SELECT count(*) FROM jsonb_array_elements(p_after->'conditions') v WHERE v->'definition'->'values' ? 'FoodRegeneration')<>1 THEN RETURN FALSE; END IF;
 SELECT v INTO before_food FROM jsonb_array_elements(p_before->'conditions') v WHERE v->'definition'->'values' ? 'FoodRegeneration';
 SELECT v INTO after_food FROM jsonb_array_elements(p_after->'conditions') v WHERE v->'definition'->'values' ? 'FoodRegeneration';
 IF after_food->'definition'->>'key' IS DISTINCT FROM 'food.regeneration'
  OR after_food->'definition'->'values'->'FoodRegeneration' IS DISTINCT FROM '{"added_ms":240000,"interval_ms":1000}'::jsonb
  OR after_food->>'ends_at' IS NULL OR after_food->>'ends_at' !~ '^[0-9]{1,20}$'
  OR (before_food IS NOT NULL AND (before_food->'definition'->>'key' IS DISTINCT FROM 'food.regeneration'
   OR before_food->>'ends_at' IS NULL OR before_food->>'ends_at' !~ '^[0-9]{1,20}$')) THEN RETURN FALSE; END IF;
 previous:=coalesce((before_food->>'ends_at')::numeric,0); following:=(after_food->>'ends_at')::numeric;
 IF following<>previous+240000000 OR following<=0 OR following>=1200000000 THEN RETURN FALSE; END IF;
 SELECT coalesce(jsonb_agg(v ORDER BY n),'[]'::jsonb) INTO before_other
  FROM jsonb_array_elements(p_before->'conditions') WITH ORDINALITY a(v,n) WHERE (v->'definition'->'values' ? 'FoodRegeneration') IS DISTINCT FROM TRUE;
 SELECT coalesce(jsonb_agg(v ORDER BY n),'[]'::jsonb) INTO after_other
  FROM jsonb_array_elements(p_after->'conditions') WITH ORDINALITY a(v,n) WHERE (v->'definition'->'values' ? 'FoodRegeneration') IS DISTINCT FROM TRUE;
 RETURN before_other IS NOT DISTINCT FROM after_other;
EXCEPTION WHEN data_exception THEN RETURN FALSE;
END $$;
CREATE FUNCTION game_cake_food_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_cake_food_observations%ROWTYPE;
BEGIN
 SELECT * INTO o FROM game_cake_food_observations WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR game_cake_world_context_is_current(NEW.world_id,NEW.event_start) IS DISTINCT FROM TRUE
  OR (o.world_id,o.event_start,o.character_id,o.game_session_id,o.command_id,o.lease_generation,
   o.actor_identity,o.movement_revision,o.state_revision,o.content_digest,o.conditions_before)
   IS DISTINCT FROM(NEW.world_id,NEW.event_start,NEW.character_id,NEW.game_session_id,NEW.command_id,NEW.character_lease_generation,
    NEW.actor_identity,NEW.movement_revision,NEW.state_revision,NEW.content_digest,NEW.conditions_before)
  OR game_cake_food_conditions_valid(NEW.conditions_before,NEW.conditions_after) IS DISTINCT FROM TRUE
  OR NEW.conditions_sha256 IS DISTINCT FROM sha256(convert_to(NEW.conditions_after::text,'UTF8'))
  OR NEW.conditions_sha256 IS DISTINCT FROM o.observed_after_sha256
  OR NOT EXISTS(SELECT 1 FROM game_seasonal_cake_bite_receipts b JOIN game_terrain_tool_receipts t USING(game_session_id,command_id)
   JOIN game_terrain_tool_transforms terrain ON (terrain.world_id,terrain.channel_id,terrain.placement,terrain.reset_epoch)=(t.world_id,t.channel_id,t.placement,t.reset_epoch)
   WHERE b.game_session_id=NEW.game_session_id AND b.command_id=NEW.command_id AND b.character_id=NEW.character_id
    AND b.world_id=NEW.world_id AND b.event_start=NEW.event_start AND b.channel_id=o.channel_id
    AND b.created_xact_id=pg_current_xact_id() AND t.created_xact_id=pg_current_xact_id()
    AND t.tool_item_instance_id IS NULL AND t.transformed_item_id=0 AND terrain.content_generation=NEW.content_digest)
 THEN RAISE EXCEPTION 'Cake food requires current private actor observation and exact fresh native bite' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE TRIGGER game_cake_food_admit BEFORE INSERT ON game_seasonal_cake_food_uses FOR EACH ROW EXECUTE FUNCTION game_cake_food_admit();
CREATE FUNCTION game_cake_food_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_condition_snapshots s WHERE s.character_id=NEW.character_id
  AND s.character_lease_generation=NEW.character_lease_generation AND s.content_digest=NEW.content_digest
  AND s.xmin=pg_current_xact_id()::xid AND s.snapshot=NEW.conditions_after
  AND sha256(convert_to(s.snapshot::text,'UTF8'))=NEW.conditions_sha256) THEN
  RAISE EXCEPTION 'Cake food must persist exact condition successor in its bite transaction' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cake_food_complete AFTER INSERT ON game_seasonal_cake_food_uses
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_food_complete();
CREATE FUNCTION game_cake_bite_food_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_seasonal_cake_food_uses f WHERE f.game_session_id=NEW.game_session_id AND f.command_id=NEW.command_id
  AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id AND f.event_start=NEW.event_start AND f.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Every fresh Cake bite requires its conserved food successor' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cake_bite_food_complete AFTER INSERT ON game_seasonal_cake_bite_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_bite_food_complete();
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOREACH f IN ARRAY ARRAY['game_cake_food_conditions_valid(jsonb,jsonb)'::regprocedure,'game_cake_food_admit()'::regprocedure,
 'game_cake_food_complete()'::regprocedure,'game_cake_bite_food_complete()'::regprocedure] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_cake_food_admit(),game_cake_food_complete(),game_cake_bite_food_complete() FROM oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_cake_food_conditions_valid(jsonb,jsonb) TO oteryn_game_runtime;

-- An early SET CONSTRAINTS cannot validate food and then replace its snapshot.
CREATE FUNCTION game_cake_food_snapshot_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE f game_seasonal_cake_food_uses%ROWTYPE;
BEGIN
 FOR f IN SELECT * FROM game_seasonal_cake_food_uses WHERE character_id=coalesce(NEW.character_id,OLD.character_id) AND created_xact_id=pg_current_xact_id() LOOP
  IF NOT EXISTS(SELECT 1 FROM game_character_condition_snapshots s WHERE s.character_id=f.character_id
   AND s.character_lease_generation=f.character_lease_generation AND s.content_digest=f.content_digest
   AND s.xmin=pg_current_xact_id()::xid AND s.snapshot=f.conditions_after
   AND sha256(convert_to(s.snapshot::text,'UTF8'))=f.conditions_sha256) THEN
   RAISE EXCEPTION 'Cake food snapshot successor changed after receipt validation' USING ERRCODE='23514'; END IF;
 END LOOP; RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cake_food_snapshot_complete AFTER INSERT OR UPDATE OR DELETE ON game_character_condition_snapshots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_food_snapshot_complete();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_food_snapshot_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_cake_food_snapshot_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
