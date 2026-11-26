-- Keep the existing connection-fenced mailbox and its bound. For an invite,
-- p_character is the invitee and the event subject is the inviting leader.
CREATE OR REPLACE FUNCTION game_enqueue_party_membership_notice(
    p_party uuid, p_character uuid, p_kind smallint
)
RETURNS void LANGUAGE sql SET search_path = pg_catalog, public AS $$
    WITH recipients AS (
        SELECT m.character_id, p_character AS subject
        FROM game_party_members m
        WHERE m.party_id = p_party AND p_kind IN (1, 2, 4)
          AND EXISTS (
              SELECT 1 FROM game_party_members changed
              WHERE changed.party_id = p_party AND changed.character_id = p_character
          )
        UNION ALL
        SELECT i.invitee, p.leader_character_id
        FROM game_party_invitations i
        JOIN game_parties p USING (party_id)
        WHERE i.party_id = p_party AND i.invitee = p_character AND p_kind = 3
          AND i.expires_at > clock_timestamp()
    )
    INSERT INTO game_party_membership_notices AS mailbox
        (character_id, game_session_id, connection_generation, events)
    SELECT r.character_id, s.game_session_id, s.current_generation,
           ARRAY[uuid_send(r.subject) || set_byte(decode('00','hex'), 0, p_kind)]
    FROM recipients r
    JOIN game_durability_admission_character_guards g USING (character_id)
    JOIN game_durability_reconnect_sessions s ON s.game_session_id = g.holder_game_session_id
    WHERE s.session_state = 2 AND s.character_id = r.character_id
      AND s.character_lease_generation = g.lease_generation
    ORDER BY r.character_id LIMIT 50
    ON CONFLICT (character_id) DO UPDATE SET
        game_session_id = EXCLUDED.game_session_id,
        connection_generation = EXCLUDED.connection_generation,
        events = CASE WHEN mailbox.game_session_id = EXCLUDED.game_session_id
                   AND mailbox.connection_generation = EXCLUDED.connection_generation
                 THEN (mailbox.events || EXCLUDED.events)[greatest(cardinality(mailbox.events) - 62, 1):]
                 ELSE EXCLUDED.events END
$$;

-- Capture the roster before the foreign-key cascade erases it. This also
-- covers the direct leader-only deletes after invitation expiry or removal.
CREATE FUNCTION game_party_enqueue_disband_notice() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    PERFORM game_enqueue_party_membership_notice(OLD.party_id, OLD.leader_character_id, 4::smallint);
    RETURN OLD;
END
$$;
REVOKE ALL ON FUNCTION game_party_enqueue_disband_notice() FROM PUBLIC;
CREATE TRIGGER game_party_disband_notice
    BEFORE DELETE ON game_parties
    FOR EACH ROW EXECUTE FUNCTION game_party_enqueue_disband_notice();

-- Shared by ordinary departure and the Channel-switch admission trigger.
CREATE OR REPLACE FUNCTION game_party_remove_with_succession(p_party uuid, p_character uuid)
RETURNS void LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    leader uuid;
    successor uuid;
BEGIN
    SELECT leader_character_id INTO leader FROM game_parties WHERE party_id = p_party FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    PERFORM game_enqueue_party_membership_notice(p_party, p_character, 2::smallint);
    IF leader = p_character THEN
        SELECT m.character_id INTO successor FROM game_party_members m
        WHERE m.party_id = p_party AND m.character_id <> p_character
        ORDER BY EXISTS(SELECT FROM game_durability_reconnect_sessions s
                        WHERE s.character_id = m.character_id AND s.session_state IN (1,2)) DESC, m.seq
        LIMIT 1 FOR UPDATE OF m;
        IF NOT FOUND THEN
            DELETE FROM game_parties WHERE party_id = p_party;
            RETURN;
        END IF;
        leader := successor;
    END IF;

    -- If this departure dissolves the party, let the disband trigger see the
    -- departing member as well as the remaining one before the cascade.
    IF (SELECT count(*) FROM game_party_members
        WHERE party_id = p_party AND character_id <> p_character) <= 1
        AND NOT EXISTS(SELECT FROM game_party_invitations WHERE party_id = p_party)
    THEN
        DELETE FROM game_parties WHERE party_id = p_party;
        RETURN;
    END IF;

    UPDATE game_parties SET leader_character_id = leader, revision = revision + 1 WHERE party_id = p_party;
    DELETE FROM game_party_members WHERE party_id = p_party AND character_id = p_character;
END
$$;
