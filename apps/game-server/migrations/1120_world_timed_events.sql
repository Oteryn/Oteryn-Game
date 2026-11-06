-- WEVENTS extends world_event::changes; reset and Channel ownership are unchanged.
CREATE TABLE game_world_timed_events (
    world_id UUID NOT NULL,
    event_key TEXT NOT NULL CHECK (event_key IN ('flower_month','mummys_curse')),
    cycle_id BIGINT NOT NULL CHECK (cycle_id >= 0),
    state JSONB NOT NULL CHECK (jsonb_typeof(state)='object' AND octet_length(state::text)<=4096),
    PRIMARY KEY (world_id,event_key,cycle_id)
);
CREATE TABLE game_world_timed_participants (
    world_id UUID NOT NULL,
    event_key TEXT NOT NULL,
    cycle_id BIGINT NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    credited BIGINT NOT NULL DEFAULT 0 CHECK (credited >= 0),
    ready_ms BIGINT NOT NULL DEFAULT 0 CHECK (ready_ms >= 0),
    claimed BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY(world_id,event_key,cycle_id,character_id),
    FOREIGN KEY(world_id,event_key,cycle_id) REFERENCES game_world_timed_events
);
CREATE INDEX game_world_timed_participant_character ON game_world_timed_participants(character_id);
CREATE TABLE game_world_timed_receipts (
    world_id UUID NOT NULL,
    event_key TEXT NOT NULL,
    cycle_id BIGINT NOT NULL,
    character_id UUID NOT NULL,
    operation_id BYTEA NOT NULL CHECK (octet_length(operation_id)=16),
    input JSONB NOT NULL CHECK (octet_length(input::text)<=512),
    applied BOOLEAN NOT NULL,
    PRIMARY KEY(world_id,event_key,cycle_id,character_id,operation_id),
    FOREIGN KEY(world_id,event_key,cycle_id,character_id) REFERENCES game_world_timed_participants
);
CREATE INDEX game_world_timed_receipt_character ON game_world_timed_receipts(character_id);
CREATE TRIGGER game_world_timed_receipts_immutable BEFORE UPDATE OR DELETE
 ON game_world_timed_receipts FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_world_timed_events,game_world_timed_participants,game_world_timed_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_world_timed_events,game_world_timed_participants TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_timed_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_timed_events,game_world_timed_participants,game_world_timed_receipts TO oteryn_game_control;
