-- chsw (owner decision 2026-11-17): a Party and its chat survive a voluntary Channel switch.
-- The switch's safe exit is the ordinary terminal release, which removes the member from its
-- Party; a switch release holds the membership instead, for the time the fresh admission into
-- the target Channel needs. The admission into the same World takes the hold back; an expired
-- hold without a current session removes the member as the ordinary logout would.
CREATE TABLE game_channel_switch_party_holds (
    character_id UUID PRIMARY KEY REFERENCES game_party_members(character_id) ON DELETE CASCADE,
    held_until_ms BIGINT NOT NULL CHECK (held_until_ms > 0)
);
CREATE INDEX game_channel_switch_party_hold_deadlines
    ON game_channel_switch_party_holds (held_until_ms, character_id);
REVOKE ALL ON game_channel_switch_party_holds FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_channel_switch_party_holds TO oteryn_game_runtime;
GRANT SELECT ON game_channel_switch_party_holds TO oteryn_game_control;

-- Two minutes from the committed safe exit to the fresh admission into the target Channel.
CREATE FUNCTION game_channel_switch_party_hold_ms() RETURNS BIGINT LANGUAGE sql IMMUTABLE AS $$
    SELECT 120000::BIGINT
$$;
REVOKE ALL ON FUNCTION game_channel_switch_party_hold_ms() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_channel_switch_party_hold_ms()
    TO oteryn_game_runtime, oteryn_game_control;

-- Common departure and succession for terminal release, expiry, and explicit removal.
CREATE FUNCTION game_party_remove_with_succession(p_party UUID, p_character UUID) RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    leader UUID;
    successor UUID;
BEGIN
    SELECT leader_character_id INTO leader FROM game_parties WHERE party_id=p_party FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    IF leader=p_character THEN
        SELECT m.character_id INTO successor FROM game_party_members m
        WHERE m.party_id=p_party AND m.character_id<>p_character
        ORDER BY EXISTS(SELECT FROM game_durability_reconnect_sessions s
                        WHERE s.character_id=m.character_id AND s.session_state IN (1,2)) DESC, m.seq
        LIMIT 1 FOR UPDATE OF m;
        IF NOT FOUND THEN
            DELETE FROM game_parties WHERE party_id=p_party;
            RETURN;
        END IF;
        leader := successor;
    END IF;
    UPDATE game_parties SET leader_character_id=leader, revision=revision+1 WHERE party_id=p_party;
    DELETE FROM game_party_members WHERE party_id=p_party AND character_id=p_character;
    DELETE FROM game_parties WHERE party_id=p_party
        AND (SELECT count(*) FROM game_party_members WHERE party_id=p_party)<=1
        AND NOT EXISTS(SELECT FROM game_party_invitations WHERE party_id=p_party);
END
$$;
REVOKE ALL ON FUNCTION game_party_remove_with_succession(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_party_remove_with_succession(UUID, UUID) TO oteryn_game_runtime;

-- A committed admission ends the hold in its own transaction; a refused admission rolls back
-- and leaves the hold to run out.
CREATE FUNCTION game_channel_switch_party_return() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    deadline BIGINT;
    party UUID;
BEGIN
    IF NOT EXISTS (SELECT FROM game_channel_switch_party_holds WHERE character_id=NEW.character_id) THEN
        RETURN NULL;
    END IF;
    PERFORM pg_advisory_xact_lock(45,hashtext(replace(NEW.world_id::text,'-','')));
    DELETE FROM game_channel_switch_party_holds WHERE character_id=NEW.character_id
        RETURNING held_until_ms INTO deadline;
    IF deadline <= game_channel_switch_now_ms() THEN
        SELECT party_id INTO STRICT party FROM game_party_members WHERE character_id=NEW.character_id FOR UPDATE;
        PERFORM game_party_remove_with_succession(party, NEW.character_id);
    END IF;
    RETURN NULL;
END
$$;
REVOKE ALL ON FUNCTION game_channel_switch_party_return() FROM PUBLIC;
CREATE TRIGGER game_channel_switch_party_return
    AFTER INSERT ON game_durability_fresh_admission_receipts
    FOR EACH ROW EXECUTE FUNCTION game_channel_switch_party_return();
