-- S14b/S14c. PR #2767 owner answers Q1B/Q2C/Q3B/Q4A/Q5A.
-- The S14a row remains a durable template. Only this session-bound advertisement
-- and its member map are removed when the leader's live occurrence ends.
CREATE TABLE IF NOT EXISTS game_team_finder_active_listings (
    leader_character_id UUID PRIMARY KEY
        REFERENCES game_team_finder_leader_listings(leader_character_id) ON DELETE CASCADE,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    leader_game_session_id UUID NOT NULL
        REFERENCES game_durability_reconnect_sessions(game_session_id),
    connection_generation NUMERIC(20,0) NOT NULL
        CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    UNIQUE (leader_character_id, world_id, channel_id)
);
CREATE INDEX IF NOT EXISTS game_team_finder_active_scope
    ON game_team_finder_active_listings(world_id, channel_id, leader_character_id);
CREATE INDEX IF NOT EXISTS game_team_finder_active_session
    ON game_team_finder_active_listings(leader_game_session_id);
REVOKE ALL ON game_team_finder_active_listings FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_team_finder_active_listings TO oteryn_game_runtime;

CREATE TABLE IF NOT EXISTS game_team_finder_members (
    leader_character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    member_character_id UUID NOT NULL
        REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    -- Crystal protocolgame.cpp:3147,3094-3105: pending/invited/accepted/denied.
    status SMALLINT NOT NULL CHECK (status BETWEEN 1 AND 4),
    PRIMARY KEY (leader_character_id, member_character_id),
    FOREIGN KEY (leader_character_id, world_id, channel_id)
        REFERENCES game_team_finder_active_listings(leader_character_id, world_id, channel_id)
        ON DELETE CASCADE,
    CHECK (leader_character_id <> member_character_id)
);
CREATE INDEX IF NOT EXISTS game_team_finder_members_character
    ON game_team_finder_members(member_character_id);
-- Q4A: only status 1 consumes the single pending slot in this World/Channel.
CREATE UNIQUE INDEX IF NOT EXISTS game_team_finder_one_pending_request
    ON game_team_finder_members(world_id, channel_id, member_character_id)
    WHERE status = 1;
REVOKE ALL ON game_team_finder_members FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_team_finder_members TO oteryn_game_runtime;

CREATE TABLE IF NOT EXISTS game_team_finder_member_receipts (
    game_session_id UUID NOT NULL
        REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL
        CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    binding BYTEA NOT NULL CHECK (octet_length(binding) = 32),
    wire_binding BYTEA NOT NULL CHECK (octet_length(wire_binding) = 32),
    accepted BOOLEAN NOT NULL,
    PRIMARY KEY (game_session_id, command_id)
);
CREATE INDEX IF NOT EXISTS game_team_finder_member_receipts_character
    ON game_team_finder_member_receipts(character_id);
REVOKE ALL ON game_team_finder_member_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_team_finder_member_receipts TO oteryn_game_runtime;

-- Old S14a receipts remain valid history; all new writes also bind their input.
ALTER TABLE game_team_finder_leader_receipts
    ADD COLUMN IF NOT EXISTS binding BYTEA CHECK (octet_length(binding) = 32);

-- Runs in the existing session owner's transaction, including kick/replacement
-- and terminal logout. No second logout writer or background best-effort cleanup.
CREATE OR REPLACE FUNCTION game_team_finder_session_ended() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.session_state = 3
       OR NEW.current_generation IS DISTINCT FROM OLD.current_generation
       OR NEW.character_lease_generation IS DISTINCT FROM OLD.character_lease_generation
       OR NEW.scope_ownership_generation IS DISTINCT FROM OLD.scope_ownership_generation
       OR NEW.runtime_scope_channel_id IS DISTINCT FROM OLD.runtime_scope_channel_id THEN
        DELETE FROM public.game_team_finder_active_listings
        WHERE leader_game_session_id = NEW.game_session_id;
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION game_team_finder_session_ended() FROM PUBLIC;
-- As with 0265's terminal-session trigger, runtime DML fires it without a
-- separately callable ingress; no role needs direct EXECUTE on this function.
DROP TRIGGER IF EXISTS game_team_finder_session_ended ON game_durability_reconnect_sessions;
CREATE TRIGGER game_team_finder_session_ended
    AFTER UPDATE OF session_state, current_generation, character_lease_generation,
        scope_ownership_generation, runtime_scope_channel_id
    ON game_durability_reconnect_sessions FOR EACH ROW
    EXECUTE FUNCTION game_team_finder_session_ended();
