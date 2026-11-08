-- A marker is a personal participant in its existing fenced quest reward receipt.
CREATE TABLE game_character_quest_markers (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL,
 transition_key TEXT NOT NULL,
 payload JSONB NOT NULL CHECK ((
   jsonb_typeof(payload)='object' AND payload->>'icon'='cross'
   AND octet_length(payload->>'text') BETWEEN 1 AND 128
   AND payload->>'text' !~ '[[:cntrl:]]'
   AND (payload #>> '{position,x}')::integer BETWEEN 0 AND 65535
   AND (payload #>> '{position,y}')::integer BETWEEN 0 AND 65535
   AND (payload #>> '{position,floor}')::integer BETWEEN -15 AND 0
   AND payload ?& ARRAY['position','icon','text']
   AND (payload->'position') ?& ARRAY['x','y','floor']
 ) IS TRUE),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
  REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_character_quest_markers_source_idx ON game_character_quest_markers(character_id,cause_id,cause_ordinal,transition_key);
CREATE FUNCTION game_quest_marker_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 IF (SELECT count(*) FROM game_character_quest_markers WHERE character_id=NEW.character_id)>=256 THEN
  RAISE EXCEPTION 'personal quest marker capacity exceeded' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_quest_marker_admit BEFORE INSERT ON game_character_quest_markers
 FOR EACH ROW EXECUTE FUNCTION game_quest_marker_admit();
CREATE FUNCTION game_quest_marker_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
  SELECT 1 FROM game_quest_reward_sources s WHERE s.character_id=NEW.character_id
   AND s.cause_id=NEW.cause_id AND s.cause_ordinal=NEW.cause_ordinal
   AND s.transition_key=NEW.transition_key AND s.created_xact_id=pg_current_xact_id()
   AND s.plan->'map_mark'=NEW.payload
 ) THEN RAISE EXCEPTION 'personal marker lacks matching quest reward' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_marker_complete AFTER INSERT ON game_character_quest_markers
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_marker_complete();
DO $migration$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_marker_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_quest_marker_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_character_quest_markers FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_marker_admit(),game_quest_marker_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_quest_markers TO oteryn_game_runtime;
GRANT SELECT ON game_character_quest_markers TO oteryn_game_control;

CREATE FUNCTION game_quest_marker_reward_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.plan ? 'map_mark' AND NEW.plan->'map_mark'<>'null'::jsonb AND NOT EXISTS(
  SELECT 1 FROM game_character_quest_markers m WHERE m.character_id=NEW.character_id
   AND m.cause_id=NEW.cause_id AND m.cause_ordinal=NEW.cause_ordinal
   AND m.transition_key=NEW.transition_key AND m.payload=NEW.plan->'map_mark'
   AND m.created_xact_id=pg_current_xact_id()
 ) THEN RAISE EXCEPTION 'quest reward lacks personal marker' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_marker_reward_complete AFTER INSERT ON game_quest_reward_sources
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_marker_reward_complete();
DO $migration$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_marker_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_marker_reward_complete() FROM PUBLIC;
