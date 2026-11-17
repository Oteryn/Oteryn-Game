-- GAME-CHANNEL-01 §8-§11 (chsw): the durable voluntary anti-hopping guard.
--
-- One row per CharacterId + WorldId remembers the last Channel of a successful fresh admission and
-- the server clock of the last voluntary switch. The guard advances inside the fresh admission
-- receipt insert, so the destination session and the switch outcome commit or roll back together
-- (§11): a refused or failed destination leaves the guard and the cooldown unchanged, and a retry
-- of the same admission cannot advance it twice. Runtime and control roles only read it.

-- The PvP type of each Channel (owner decision 2026-11-17): 1 Optional PvP, 2 Open PvP. Control
-- writes it with the Channel's topology; a Channel without a row has no known type.
CREATE TABLE game_channel_pvp_types (
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    pvp_type SMALLINT NOT NULL CHECK (pvp_type IN (1, 2)),
    player_capacity INTEGER NOT NULL DEFAULT 100 CHECK (player_capacity > 0),
    PRIMARY KEY (world_id, channel_id)
);
REVOKE ALL ON game_channel_pvp_types FROM PUBLIC;
GRANT SELECT ON game_channel_pvp_types TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_channel_pvp_types TO oteryn_game_control;

INSERT INTO game_channel_pvp_types(world_id, channel_id, pvp_type)
SELECT world_id, channel_id, 1 FROM game_runtime_scope_assignments WHERE scope_kind=1 AND state=1;

-- Count live and reconnectable sessions as the Channel load command does. The existing
-- Control-issued account type grants GM access at level 4 (the Gamemaster room predicate).
CREATE FUNCTION game_channel_full(p_character UUID, p_world UUID, p_channel UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT EXISTS (
        SELECT FROM game_channel_pvp_types c
        WHERE c.world_id=p_world AND c.channel_id=p_channel AND c.player_capacity <= (
            SELECT count(*) FROM game_durability_reconnect_sessions s
            WHERE s.world_id=p_world AND s.session_state IN (1,2)
              AND COALESCE(s.runtime_scope_channel_id,s.origin_channel_id)=p_channel
        )
    ) AND NOT EXISTS (
        SELECT FROM game_chat_roles r WHERE r.character_id=p_character AND r.account_type >= 4
    )
$$;
REVOKE ALL ON FUNCTION game_channel_full(UUID,UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_channel_full(UUID,UUID,UUID) TO oteryn_game_runtime, oteryn_game_control;

CREATE TABLE game_channel_switch_guards (
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    last_channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(last_channel_id)),
    -- The fresh admission that last set the remembered Channel.
    last_game_session_id UUID NOT NULL,
    -- Server clock (epoch ms) of the last voluntary switch; NULL until the first one.
    switched_at BIGINT CHECK (switched_at >= 0),
    switch_count BIGINT NOT NULL DEFAULT 0 CHECK (switch_count >= 0),
    world_policy_revision SMALLINT NOT NULL DEFAULT 1 CHECK (world_policy_revision = 1),
    PRIMARY KEY (character_id, world_id),
    CHECK ((switch_count = 0) = (switched_at IS NULL))
);
REVOKE ALL ON game_channel_switch_guards FROM PUBLIC;
GRANT SELECT ON game_channel_switch_guards TO oteryn_game_runtime, oteryn_game_control;

-- Fresh receipts retain the increasing character lease generation across admissions.
LOCK TABLE game_durability_fresh_admission_receipts IN SHARE ROW EXCLUSIVE MODE;
INSERT INTO game_channel_switch_guards (character_id, world_id, last_channel_id, last_game_session_id)
SELECT DISTINCT ON (character_id, world_id) character_id, world_id, channel_id, game_session_id
FROM game_durability_fresh_admission_receipts
ORDER BY character_id, world_id, character_lease_generation DESC, authorization_decided_at DESC, game_session_id DESC;

-- Five minutes between voluntary switches (owner decision 2026-11-17, §10).
CREATE FUNCTION game_channel_switch_cooldown_ms() RETURNS BIGINT LANGUAGE sql IMMUTABLE AS $$
    SELECT 300000::BIGINT
$$;

CREATE FUNCTION game_channel_switch_now_ms() RETURNS BIGINT LANGUAGE sql VOLATILE AS $$
    SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT
$$;

-- The hard locks that the database itself can prove for a switch into p_channel (§8): an open
-- in-fight logout block, and a red or black skull entering a Channel that is not known Open PvP.
-- Returns NULL when none applies, else the refusal reason.
CREATE FUNCTION game_channel_switch_lock(p_character UUID, p_world UUID, p_channel UUID)
RETURNS TEXT LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT CASE
        WHEN game_channel_full(p_character,p_world,p_channel) THEN 'CHANNEL_FULL'
        WHEN EXISTS (SELECT 1 FROM game_character_last_positions p
                     WHERE p.character_id = p_character AND p.world_id = p_world
                       AND p.house_key IS NOT NULL AND p.origin_channel_id <> p_channel)
            THEN 'TRANSITION'
        WHEN EXISTS (SELECT 1 FROM game_character_pvp_state s WHERE s.character_id = p_character
                     AND GREATEST(s.logout_block_until, s.kill_block_until, s.pz_block_until) > clock_timestamp()) THEN 'COMBAT'
        WHEN EXISTS (SELECT 1 FROM game_character_pvp_state s WHERE s.character_id = p_character
                     AND s.skull IN (1, 2) AND s.skull_until > clock_timestamp())
             AND COALESCE((SELECT t.pvp_type FROM game_channel_pvp_types t
                           WHERE t.world_id = p_world AND t.channel_id = p_channel), 1) <> 2
            THEN 'SKULL'
    END
$$;
REVOKE ALL ON FUNCTION game_channel_switch_lock(UUID, UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_channel_switch_lock(UUID, UUID, UUID)
    TO oteryn_game_runtime, oteryn_game_control;

-- The remembered Channel and the cooldown left (ms, server clock) for one Character in one World.
-- No row: the next admission sets the baseline and no cooldown runs.
CREATE FUNCTION game_channel_switch_status(p_character UUID, p_world UUID)
RETURNS TABLE (last_channel_id UUID, cooldown_left_ms BIGINT)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT g.last_channel_id,
           CASE WHEN g.switched_at IS NULL THEN 0::BIGINT
                ELSE GREATEST(0::BIGINT, g.switched_at + game_channel_switch_cooldown_ms()
                                          - game_channel_switch_now_ms()) END
    FROM game_channel_switch_guards g
    WHERE g.character_id = p_character AND g.world_id = p_world
$$;
REVOKE ALL ON FUNCTION game_channel_switch_status(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_channel_switch_status(UUID, UUID)
    TO oteryn_game_runtime, oteryn_game_control;

-- Classification inside the final admission boundary (§10-§11). A relog into another Channel is a
-- switch too. Refusals raise, so the admission and everything it wrote roll back.
CREATE FUNCTION game_channel_switch_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    g game_channel_switch_guards;
    now_ms BIGINT := game_channel_switch_now_ms();
    lock_reason TEXT;
BEGIN
    -- The admission store holds the reconnect-session relation exclusively through commit.
    IF game_channel_full(NEW.character_id,NEW.world_id,NEW.channel_id) THEN
        RAISE EXCEPTION 'Channel full' USING ERRCODE='OTH02', DETAIL='CHANNEL_FULL';
    END IF;
    INSERT INTO game_channel_switch_guards (character_id, world_id, last_channel_id, last_game_session_id)
    VALUES (NEW.character_id, NEW.world_id, NEW.channel_id, NEW.game_session_id)
    ON CONFLICT DO NOTHING;
    IF FOUND THEN
        RETURN NULL; -- the first admission in this World sets the baseline
    END IF;
    SELECT * INTO STRICT g FROM game_channel_switch_guards
    WHERE character_id = NEW.character_id AND world_id = NEW.world_id FOR UPDATE;
    IF g.last_channel_id = NEW.channel_id THEN
        RETURN NULL; -- the same Channel is not a switch
    END IF;
    IF g.switched_at IS NOT NULL AND now_ms - g.switched_at < game_channel_switch_cooldown_ms() THEN
        RAISE EXCEPTION 'Channel switch cooldown'
            USING ERRCODE = 'OTH01',
                  DETAIL = (g.switched_at + game_channel_switch_cooldown_ms() - now_ms)::TEXT;
    END IF;
    lock_reason := game_channel_switch_lock(NEW.character_id, NEW.world_id, NEW.channel_id);
    IF lock_reason IS NOT NULL THEN
        RAISE EXCEPTION 'Channel switch locked' USING ERRCODE = 'OTH02', DETAIL = lock_reason;
    END IF;
    UPDATE game_channel_switch_guards
    SET last_channel_id = NEW.channel_id, last_game_session_id = NEW.game_session_id,
        switched_at = now_ms, switch_count = switch_count + 1
    WHERE character_id = NEW.character_id AND world_id = NEW.world_id;
    RETURN NULL;
END $$;
CREATE TRIGGER game_channel_switch_admit AFTER INSERT ON game_durability_fresh_admission_receipts
    FOR EACH ROW EXECUTE FUNCTION game_channel_switch_admit();
