-- Transient World-party delivery. Keep one bounded mailbox per Character;
-- bind every batch to the exact live connection that was present at the change.
-- There is no FK to recipient session rows: the World party owner must not
-- take locks on other sessions while holding the World membership lock.
CREATE TABLE public.game_party_membership_notices (
    character_id uuid PRIMARY KEY,
    game_session_id uuid NOT NULL,
    connection_generation numeric(20,0) NOT NULL CHECK (connection_generation > 0),
    events bytea[] NOT NULL CHECK (cardinality(events) BETWEEN 1 AND 64)
);
REVOKE ALL ON TABLE public.game_party_membership_notices FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.game_party_membership_notices TO oteryn_game_runtime;

CREATE FUNCTION game_enqueue_party_membership_notice(p_party uuid, p_character uuid, p_kind smallint)
RETURNS void LANGUAGE sql SET search_path = pg_catalog, public AS $$
    INSERT INTO game_party_membership_notices AS mailbox
        (character_id, game_session_id, connection_generation, events)
    SELECT m.character_id, s.game_session_id, s.current_generation,
           ARRAY[uuid_send(p_character) || set_byte(decode('00','hex'), 0, p_kind)]
    FROM game_party_members m
    JOIN game_durability_admission_character_guards g USING (character_id)
    JOIN game_durability_reconnect_sessions s ON s.game_session_id = g.holder_game_session_id
    WHERE m.party_id = p_party AND s.session_state = 2 AND p_kind IN (1, 2)
      AND EXISTS (SELECT 1 FROM game_party_members changed WHERE changed.party_id = p_party AND changed.character_id = p_character)
      AND s.character_id = m.character_id AND s.character_lease_generation = g.lease_generation
    ORDER BY m.character_id LIMIT 50
    ON CONFLICT (character_id) DO UPDATE SET
        game_session_id = EXCLUDED.game_session_id,
        connection_generation = EXCLUDED.connection_generation,
        events = CASE WHEN mailbox.game_session_id = EXCLUDED.game_session_id
                   AND mailbox.connection_generation = EXCLUDED.connection_generation
                 THEN (mailbox.events || EXCLUDED.events)[greatest(cardinality(mailbox.events) - 62, 1):]
                 ELSE EXCLUDED.events END
$$;
REVOKE ALL ON FUNCTION game_enqueue_party_membership_notice(uuid, uuid, smallint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_enqueue_party_membership_notice(uuid, uuid, smallint) TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_party_remove_with_succession(p_party UUID, p_character UUID) RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    leader UUID;
    successor UUID;
BEGIN
    SELECT leader_character_id INTO leader FROM game_parties WHERE party_id=p_party FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM game_enqueue_party_membership_notice(p_party, p_character, 2::smallint);
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
