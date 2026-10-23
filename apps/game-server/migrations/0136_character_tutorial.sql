-- TUTORIAL-1: one session-fenced, CAS-serialized progress projection per Character.
-- The Character root lock serializes initial inserts as well as updates.
CREATE TABLE game_character_tutorial (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    version BIGINT NOT NULL CHECK (version > 0),
    step SMALLINT NOT NULL CHECK (step IN (1, 2, 3)),
    hunting BOOLEAN NOT NULL,
    corruptor BOOLEAN NOT NULL,
    pending_islet BOOLEAN NOT NULL DEFAULT FALSE,
    CHECK (NOT pending_islet OR step = 1),
    CHECK (step <> 3 OR (NOT hunting AND NOT corruptor))
);
REVOKE ALL ON game_character_tutorial FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_tutorial TO oteryn_game_runtime;

-- Free Newhaven routes reuse the accepted pending-arrival reader and consume owner.
CREATE TABLE game_character_pending_arrivals (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    occurrence_id UUID NOT NULL, world_id UUID NOT NULL,
    tile_x INTEGER NOT NULL, tile_y INTEGER NOT NULL, tile_floor SMALLINT NOT NULL
);
CREATE TABLE game_newhaven_travel_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    occurrence_id UUID NOT NULL UNIQUE,
    route TEXT NOT NULL CHECK (route IN ('rookgaard','thais_peninsula')),
    arrived_at TIMESTAMPTZ,
    PRIMARY KEY(character_id, game_session_id, command_id)
);
REVOKE ALL ON game_character_pending_arrivals, game_newhaven_travel_receipts FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_character_pending_arrivals TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE(arrived_at) ON game_newhaven_travel_receipts TO oteryn_game_runtime;

-- The death transaction already holds the Character root. A late free-travel commit
-- either precedes this trigger, or observes the pending respawn and refuses.
CREATE FUNCTION game_newhaven_death_cancels_arrival() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM game_character_pending_arrivals WHERE character_id = NEW.character_id;
    RETURN NEW;
END;
$$;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_newhaven_death_cancels_arrival() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END;
$$;
CREATE TRIGGER game_newhaven_death_cancels_arrival AFTER INSERT ON game_character_pending_respawns
    FOR EACH ROW EXECUTE FUNCTION game_newhaven_death_cancels_arrival();
REVOKE ALL ON FUNCTION game_newhaven_death_cancels_arrival() FROM PUBLIC;

-- One immutable winner per Character and real creature-death occurrence. Clearing
-- the armed track and recording its selected defined Items is one transaction;
-- existing corpse MINT causes retain their D52 partial-loot/replay semantics.
CREATE TABLE game_tutorial_corruptor_claims (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL, channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL,
    actor_id BIGINT NOT NULL,
    actor_generation NUMERIC(20,0) NOT NULL,
    item_ids INTEGER[] NOT NULL CHECK (cardinality(item_ids)<=3 AND 0<ALL(item_ids)),
    PRIMARY KEY(character_id,world_id,channel_id,scope_generation,actor_id,actor_generation)
);
REVOKE ALL ON game_tutorial_corruptor_claims FROM PUBLIC;
GRANT SELECT, INSERT ON game_tutorial_corruptor_claims TO oteryn_game_runtime;
