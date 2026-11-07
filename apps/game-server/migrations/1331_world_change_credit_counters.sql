-- Extend WORLDCHG-Q's existing World-event owner. No new DAILY or RAID scheduler.
CREATE TABLE game_world_change_counters (
    world_id UUID NOT NULL,
    change_key TEXT NOT NULL CHECK (change_key IN ('deeplings','demon_wars','fire_from_the_earth','hive_born','overhunting','the_fire_feathered_serpent','the_mage_s_tower')),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    state JSONB NOT NULL CHECK (jsonb_typeof(state)='object' AND octet_length(state::text)<=2048),
    PRIMARY KEY(world_id,change_key),
    FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE INDEX game_world_change_counters_epoch ON game_world_change_counters(world_id,reset_epoch);
CREATE TABLE game_world_change_credit_receipts (
    world_id UUID NOT NULL,
    source_kind SMALLINT NOT NULL CHECK (source_kind BETWEEN 1 AND 5),
    occurrence BYTEA NOT NULL CHECK (octet_length(occurrence) BETWEEN 16 AND 128),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    binding_digest BYTEA NOT NULL CHECK (octet_length(binding_digest)=32),
    PRIMARY KEY(world_id,source_kind,occurrence),
    FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE INDEX game_world_change_credit_receipts_epoch ON game_world_change_credit_receipts(world_id,reset_epoch);
CREATE TRIGGER game_world_change_credit_receipts_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_credit_receipts FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TABLE game_world_change_personal_counters (
    world_id UUID NOT NULL,
    change_key TEXT NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots ON DELETE CASCADE,
    counter NUMERIC(20,0) NOT NULL CHECK(counter BETWEEN 0 AND 18446744073709551615),
    PRIMARY KEY(world_id,change_key,character_id),
    FOREIGN KEY(world_id,change_key) REFERENCES game_world_change_counters
);
CREATE INDEX game_world_change_personal_character ON game_world_change_personal_counters(character_id);
REVOKE ALL ON game_world_change_counters,game_world_change_credit_receipts,game_world_change_personal_counters FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_world_change_counters,game_world_change_personal_counters TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_change_credit_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_counters,game_world_change_credit_receipts,game_world_change_personal_counters TO oteryn_game_control;
