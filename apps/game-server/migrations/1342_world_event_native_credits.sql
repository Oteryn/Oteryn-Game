-- One immutable, input-bound receipt per native death and World event cycle.
CREATE TABLE game_world_timed_kill_receipts (
 world_id UUID NOT NULL,
 event_key TEXT NOT NULL,
 cycle_id BIGINT NOT NULL,
 occurrence BYTEA NOT NULL CHECK(octet_length(occurrence)=52),
 input_binding BYTEA NOT NULL CHECK(octet_length(input_binding)=32),
 firing_id UUID NOT NULL REFERENCES game_world_timed_raid_origins(firing_id),
 channel_id UUID NOT NULL,
 wave SMALLINT NOT NULL CHECK(wave>=0),
 entry SMALLINT NOT NULL CHECK(entry>=0),
 creature_index SMALLINT NOT NULL CHECK(creature_index>=0),
 UNIQUE(firing_id,channel_id,wave,entry,creature_index),
 PRIMARY KEY(world_id,event_key,cycle_id,occurrence),
 FOREIGN KEY(world_id,event_key,cycle_id) REFERENCES game_world_timed_events
);
CREATE TRIGGER game_world_timed_kill_receipts_immutable BEFORE UPDATE OR DELETE
 ON game_world_timed_kill_receipts FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_world_timed_kill_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_timed_kill_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_timed_kill_receipts TO oteryn_game_control;
