-- Current Wiki cadence: source stage checks every600s, fixed1800s eating rounds.
CREATE TABLE game_seasonal_cake_stage_polls (
 world_id UUID NOT NULL,event_start BIGINT NOT NULL,poll_slot BIGINT NOT NULL CHECK(poll_slot>=1),
 observed_at BIGINT NOT NULL,opened_bridge BOOLEAN NOT NULL,created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,event_start,poll_slot),FOREIGN KEY(world_id,event_start) REFERENCES game_seasonal_cake_worlds
);
CREATE TABLE game_seasonal_cake_windows (
 world_id UUID NOT NULL,event_start BIGINT NOT NULL,window_start BIGINT NOT NULL,
 layer SMALLINT NOT NULL CHECK(layer BETWEEN 1 AND 4),observed_at BIGINT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,event_start,window_start),FOREIGN KEY(world_id,event_start) REFERENCES game_seasonal_cake_worlds
);
CREATE UNIQUE INDEX game_cake_one_bridge_open ON game_seasonal_cake_stage_polls(world_id,event_start) WHERE opened_bridge;
CREATE TRIGGER game_cake_stage_polls_immutable BEFORE UPDATE OR DELETE ON game_seasonal_cake_stage_polls FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_cake_windows_immutable BEFORE UPDATE OR DELETE ON game_seasonal_cake_windows FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_seasonal_cake_stage_polls,game_seasonal_cake_windows FROM PUBLIC;
GRANT SELECT,INSERT ON game_seasonal_cake_stage_polls,game_seasonal_cake_windows TO oteryn_game_runtime;
GRANT SELECT ON game_seasonal_cake_stage_polls,game_seasonal_cake_windows TO oteryn_game_control;
CREATE FUNCTION game_cake_fixed_clock_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_cake_world_node_context%ROWTYPE; w game_seasonal_cake_worlds%ROWTYPE; previous JSONB; anchor BIGINT; slot BIGINT; prior_window BIGINT;
BEGIN
 SELECT * INTO c FROM game_cake_world_node_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR game_cake_world_context_is_current(NEW.world_id,NEW.event_start) IS DISTINCT FROM TRUE
  OR NEW.observed_at<>c.now_seconds OR c.now_seconds>=c.event_start+518400 THEN
  RAISE EXCEPTION 'Cake clock claim lacks exact current Node occurrence' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT w FROM game_seasonal_cake_worlds WHERE world_id=NEW.world_id AND event_start=NEW.event_start FOR UPDATE;
 previous:=CASE WHEN c.captured THEN c.initial_state ELSE w.state END;
 NEW.created_xact_id:=pg_current_xact_id();
 IF TG_TABLE_NAME='game_seasonal_cake_stage_polls' THEN
  IF (previous->>'stage')::integer<>1 OR NEW.poll_slot<>(c.now_seconds-c.event_start)/600
   OR NEW.poll_slot<1 OR NEW.opened_bridge IS DISTINCT FROM ((previous->>'substances')::integer=200 AND (previous->>'golems')::integer=200) THEN
   RAISE EXCEPTION 'Cake stage poll is not its due source bucket or predecessor' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT observed_at INTO anchor FROM game_seasonal_cake_stage_polls WHERE world_id=NEW.world_id AND event_start=NEW.event_start AND opened_bridge;
  slot:=CASE WHEN anchor IS NULL THEN NULL ELSE anchor+((c.now_seconds-anchor)/1800)*1800 END;
  SELECT max(window_start) INTO prior_window FROM game_seasonal_cake_tiles WHERE world_id=NEW.world_id AND event_start=NEW.event_start AND layer=NEW.layer-1;
  IF (previous->>'stage')::integer<>2 OR anchor IS NULL OR c.now_seconds<anchor+1800
   OR NEW.window_start IS DISTINCT FROM slot OR NEW.layer NOT IN ((previous->>'layer')::smallint,(previous->>'layer')::smallint+1)
   OR (prior_window IS NOT NULL AND NEW.window_start<=prior_window)
   OR ((previous->>'window_end') IS NOT NULL AND c.now_seconds<(previous->>'window_end')::bigint)
  THEN RAISE EXCEPTION 'Cake eating window is not the next fixed source interval' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_fixed_clock_admit BEFORE INSERT ON game_seasonal_cake_stage_polls FOR EACH ROW EXECUTE FUNCTION game_cake_fixed_clock_admit();
CREATE TRIGGER game_cake_fixed_clock_admit BEFORE INSERT ON game_seasonal_cake_windows FOR EACH ROW EXECUTE FUNCTION game_cake_fixed_clock_admit();
CREATE TRIGGER game_cake_z_invalidate BEFORE INSERT ON game_seasonal_cake_stage_polls FOR EACH ROW EXECUTE FUNCTION game_cake_world_invalidate();
CREATE TRIGGER game_cake_z_invalidate BEFORE INSERT ON game_seasonal_cake_windows FOR EACH ROW EXECUTE FUNCTION game_cake_world_invalidate();
CREATE CONSTRAINT TRIGGER game_cake_fixed_clock_complete AFTER INSERT ON game_seasonal_cake_stage_polls DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_world_deferred();
CREATE CONSTRAINT TRIGGER game_cake_fixed_clock_complete AFTER INSERT ON game_seasonal_cake_windows DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_world_deferred();
-- Old first-USE window receipts remain readable for immutable historical replay;
-- they cannot authorize new rounds under the current fixed source clock.
CREATE OR REPLACE FUNCTION game_cake_window_start_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN RAISE EXCEPTION 'Cake windows are opened by the fixed native clock' USING ERRCODE='23514'; END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOREACH f IN ARRAY ARRAY['game_cake_fixed_clock_admit()'::regprocedure,'game_cake_window_start_admit()'::regprocedure] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
