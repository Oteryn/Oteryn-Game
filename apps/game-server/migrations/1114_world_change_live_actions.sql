-- WCONTRIB: mid-epoch effects stay inside the existing World changes owner.
ALTER TABLE game_world_raid_schedule_sources
 DROP CONSTRAINT game_world_raid_schedule_sources_source_kind_check,
 ADD CONSTRAINT game_world_raid_schedule_sources_source_kind_check CHECK(source_kind IN (1,2,3,4));
CREATE INDEX game_world_raid_firings_world_epoch_key
 ON game_world_raid_firings(world_id,reset_epoch,raid_key);
CREATE TABLE game_world_change_live (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 state JSONB NOT NULL CHECK(jsonb_typeof(state)='object' AND octet_length(state::text)<=2048),
 updated_at_ms BIGINT NOT NULL CHECK(updated_at_ms>=0),
 PRIMARY KEY(world_id,reset_epoch),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_states
);
CREATE TABLE game_world_change_action_receipts (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 operation_id BYTEA NOT NULL CHECK(octet_length(operation_id)=16),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 before_state JSONB NOT NULL,
 after_state JSONB NOT NULL,
 updated_at_ms BIGINT NOT NULL CHECK(updated_at_ms>=0),
 character_id UUID,
 game_session_id UUID,
 cause_ordinal NUMERIC(20,0),
 transition_key TEXT,
 node_id UUID,
 node_registration_revision NUMERIC(20,0),
 channel_id UUID,
 scope_generation NUMERIC(20,0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,reset_epoch,revision),
 UNIQUE(world_id,reset_epoch,operation_id),
 UNIQUE(character_id,game_session_id,cause_ordinal),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_states,
 FOREIGN KEY(character_id,game_session_id,cause_ordinal,transition_key)
   REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
   DEFERRABLE INITIALLY DEFERRED,
 CHECK(jsonb_typeof(before_state)='object' AND octet_length(before_state::text)<=2048),
 CHECK(jsonb_typeof(after_state)='object' AND octet_length(after_state::text)<=2048),
 CHECK(CASE WHEN character_id IS NOT NULL THEN
   game_session_id IS NOT NULL AND cause_ordinal IS NOT NULL AND cause_ordinal BETWEEN 1 AND 18446744073709551615
   AND transition_key IS NOT NULL AND node_id IS NULL AND node_registration_revision IS NULL
   AND channel_id IS NULL AND scope_generation IS NULL
 ELSE game_session_id IS NULL AND cause_ordinal IS NULL AND transition_key IS NULL
   AND node_id IS NOT NULL AND node_registration_revision IS NOT NULL AND node_registration_revision BETWEEN 1 AND 18446744073709551615
   AND channel_id IS NOT NULL AND scope_generation IS NOT NULL AND scope_generation BETWEEN 1 AND 18446744073709551615 END)
);

CREATE FUNCTION game_world_change_action_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT game_world_reset_map_epoch_current(NEW.world_id,NEW.reset_epoch)
   OR NOT EXISTS(SELECT 1 FROM game_world_reset_active a
     WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch) THEN
   RAISE EXCEPTION 'world change action requires current epoch' USING ERRCODE='23514';
 END IF;
 IF NEW.character_id IS NULL THEN
   IF NOT game_node_lock_current_registration(NEW.node_id,NEW.node_registration_revision) THEN
     RAISE EXCEPTION 'world change tick requires current node' USING ERRCODE='23514';
   END IF;
   PERFORM a.scope_key FROM game_runtime_scope_assignments a
    JOIN game_durability_admission_runtime_guards g USING(scope_key)
    WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.channel_id AND a.scope_kind=1 AND a.state=1
      AND a.holder_node_id=NEW.node_id AND a.holder_registration_revision=NEW.node_registration_revision
      AND a.ownership_generation=NEW.scope_generation AND g.ready AND g.ownership_generation=a.ownership_generation
    FOR SHARE OF a,g;
   IF NOT FOUND THEN
     RAISE EXCEPTION 'world change tick requires current Channel generation' USING ERRCODE='23514';
   END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_world_change_live_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_change_action_receipts r
   WHERE (r.world_id,r.reset_epoch,r.revision)=(NEW.world_id,NEW.reset_epoch,NEW.revision)
     AND r.after_state=NEW.state AND r.updated_at_ms=NEW.updated_at_ms
     AND r.created_xact_id=pg_current_xact_id()
     AND (TG_OP='INSERT' OR (r.before_state=OLD.state AND NEW.updated_at_ms>=OLD.updated_at_ms))) THEN
   RAISE EXCEPTION 'world live state lacks atomic action receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_world_change_live_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' AND NEW.revision=1 THEN RETURN NEW; END IF;
 IF TG_OP='UPDATE' AND NEW.world_id=OLD.world_id AND NEW.reset_epoch=OLD.reset_epoch
   AND NEW.revision=OLD.revision+1 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'world live state cannot be deleted, rekeyed or skip revision' USING ERRCODE='23514';
END $$;
CREATE FUNCTION game_world_change_action_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_change_live s
   WHERE (s.world_id,s.reset_epoch,s.revision)=(NEW.world_id,NEW.reset_epoch,NEW.revision)
     AND s.state=NEW.after_state AND s.updated_at_ms=NEW.updated_at_ms)
   OR NEW.created_xact_id<>pg_current_xact_id()
   OR (NEW.character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
     WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id
       AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
       AND r.created_xact_id=pg_current_xact_id() AND r.cause_kind IN ('command','use'))) THEN
   RAISE EXCEPTION 'incomplete atomic world change action' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_world_change_action_admit BEFORE INSERT ON game_world_change_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_change_action_admit();
CREATE CONSTRAINT TRIGGER game_world_change_action_complete AFTER INSERT ON game_world_change_action_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_action_complete();
CREATE CONSTRAINT TRIGGER game_world_change_live_proven AFTER INSERT OR UPDATE ON game_world_change_live
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_live_proven();
CREATE TRIGGER game_world_change_live_guard BEFORE INSERT OR UPDATE OR DELETE ON game_world_change_live
 FOR EACH ROW EXECUTE FUNCTION game_world_change_live_guard();
CREATE TRIGGER game_world_change_action_immutable BEFORE UPDATE OR DELETE ON game_world_change_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_change_action_no_truncate BEFORE TRUNCATE ON game_world_change_action_receipts
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_change_live_no_truncate BEFORE TRUNCATE ON game_world_change_live
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_world_change_action_admit','game_world_change_action_complete',
   'game_world_change_live_proven','game_world_change_live_guard'] LOOP
   EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
REVOKE ALL ON game_world_change_live,game_world_change_action_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_world_change_live TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_change_action_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_live,game_world_change_action_receipts TO oteryn_game_control;

-- Personal two-week repeat restriction reads one bounded historical return.
CREATE INDEX game_world_change_action_character_return
 ON game_world_change_action_receipts(character_id,transition_key,updated_at_ms DESC)
 WHERE character_id IS NOT NULL;
