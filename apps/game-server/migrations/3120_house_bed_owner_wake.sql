-- A USE occurrence is decided once, even if the bed was vacant on first use.
-- The canonical bed head identifies the whole pair, including foot USE.
CREATE TABLE game_house_bed_use_receipts (
    game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    binding JSONB NOT NULL CHECK (octet_length(binding::text) <= 4096),
    outcome TEXT NOT NULL CHECK (outcome IN ('vacant', 'occupied', 'woken')),
    PRIMARY KEY (game_session_id, command_id)
);
REVOKE ALL ON game_house_bed_use_receipts FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
CREATE TRIGGER game_house_bed_use_receipt_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_house_bed_use_receipts FOR EACH STATEMENT
    EXECUTE FUNCTION game_house_bed_sleep_receipt_immutable();

CREATE FUNCTION game_house_bed_use(
    p_session UUID, p_character UUID, p_world UUID, p_house TEXT,
    p_generation NUMERIC, p_lease NUMERIC, p_scope NUMERIC, p_transport BYTEA,
    p_command NUMERIC, p_x INTEGER, p_y INTEGER, p_z INTEGER,
    p_node UUID, p_registration NUMERIC
) RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    intent JSONB;
    prior game_house_bed_use_receipts%ROWTYPE;
    access_role TEXT;
    result TEXT;
BEGIN
    IF p_command IS NULL OR p_command NOT BETWEEN 1 AND 18446744073709551615
       OR p_command <> trunc(p_command)
       OR p_x IS NULL OR p_y IS NULL OR p_z IS NULL THEN
        RETURN 'denied';
    END IF;

    -- Serialize the occurrence on the still-active native transport.
    PERFORM 1 FROM game_durability_reconnect_sessions s
        JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id
        JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id
        WHERE s.game_session_id=p_session AND s.character_id=p_character
          AND s.current_generation=p_generation AND s.character_lease_generation=p_lease
          AND s.scope_ownership_generation=p_scope AND s.current_transport_ref=p_transport
          AND COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) IS NOT NULL
          AND s.session_state=2
          AND c.eligible AND c.account_id=s.account_id AND c.world_id=p_world
          AND c.lease_generation=p_lease AND c.holder_game_session_id=p_session
          AND a.presence_character_id=p_character AND a.holder_game_session_id=p_session
        FOR UPDATE OF s FOR SHARE OF c,a;
    IF NOT FOUND THEN RETURN 'stale'; END IF;
    -- Platform locks this session before publishing its durable revocation.
    IF game_security_session_revoked(p_session) IS DISTINCT FROM FALSE THEN
        RETURN 'stale';
    END IF;
    IF NOT game_item_action_house_current(p_session,p_character,p_generation,p_lease,p_scope,
                                           p_node,p_registration,p_world,p_house) THEN
        RETURN 'stale';
    END IF;

    intent := jsonb_build_array(p_character,p_world,p_house,p_generation,p_lease,p_scope,
                               encode(p_transport,'hex'),p_x,p_y,p_z);
    SELECT * INTO prior FROM game_house_bed_use_receipts
        WHERE game_session_id=p_session AND command_id=p_command;
    IF FOUND THEN
        IF prior.binding = intent THEN RETURN prior.outcome; END IF;
        RETURN 'stale';
    END IF;

    -- Native ACL locks current guild (where applicable) and property. Keep
    -- the bed writers' order of ownership anchor -> bed revision -> sleeper.
    SELECT role INTO access_role FROM game_house_access(p_world,p_house,p_character)
        WHERE content_fenced IS FALSE;
    IF NOT FOUND THEN RETURN 'denied'; END IF;
    PERFORM 1 FROM game_house_ownership WHERE world_id=p_world AND house_key=p_house FOR SHARE;
    PERFORM 1 FROM game_house_bed_revisions
        WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
    IF NOT FOUND THEN RETURN 'denied'; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM game_house_bed_sleepers
          WHERE world_id=p_world AND house_key=p_house AND bed_x=p_x AND bed_y=p_y
            AND bed_z=p_z AND freed_at IS NULL
    ) THEN
        result := 'vacant';
    ELSIF access_role IS DISTINCT FROM 'OWNER' THEN
        result := 'occupied';
    ELSE
        UPDATE game_house_bed_sleepers
            SET freed_at = greatest(slept_at, floor(extract(epoch FROM clock_timestamp())*1000)::bigint),
                freed_cause = 'owner_wake'
            WHERE world_id=p_world AND house_key=p_house AND bed_x=p_x AND bed_y=p_y
              AND bed_z=p_z AND freed_at IS NULL;
        result := 'woken';
    END IF;

    INSERT INTO game_house_bed_use_receipts VALUES(p_session,p_command,intent,result);
    RETURN result;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,UUID,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,UUID,NUMERIC) FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,UUID,NUMERIC) TO oteryn_game_runtime;

-- Loss publication has acquired the same session lock as the writer. Read the
-- original transport's immutable receipt without requiring current custody.
CREATE FUNCTION game_house_bed_use_receipt(
    p_session UUID, p_character UUID, p_generation NUMERIC,
    p_lease NUMERIC, p_scope NUMERIC, p_transport BYTEA, p_command NUMERIC
) RETURNS TEXT LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT r.outcome FROM game_house_bed_use_receipts r
    WHERE r.game_session_id=p_session AND r.command_id=p_command
      AND r.binding->0=to_jsonb(p_character)
      AND r.binding->3=to_jsonb(p_generation)
      AND r.binding->4=to_jsonb(p_lease)
      AND r.binding->5=to_jsonb(p_scope)
      AND r.binding->6=to_jsonb(encode(p_transport,'hex'))
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC)
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC)
    TO oteryn_game_runtime;
