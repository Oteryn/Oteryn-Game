-- Bed sleep runs on a live (Active) House session. Session state 2 is Active and 1 is
-- Reconnectable (fresh_admission.rs); 0149 required state 1, which production sleep (reached
-- only for an Active session) never satisfied. Only the session check changes.
CREATE OR REPLACE FUNCTION game_house_bed_sleep(
    p_session UUID, p_character UUID, p_world UUID, p_house TEXT,
    p_generation NUMERIC, p_lease NUMERIC, p_scope NUMERIC, p_transport BYTEA,
    p_command NUMERIC, p_x INTEGER, p_y INTEGER, p_z INTEGER,
    p_variant TEXT, p_skill INTEGER, p_node UUID, p_registration NUMERIC
) RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    intent JSONB;
    prior JSONB;
    owner_id UUID;
    old_house TEXT;
    old_world UUID;
    sleep_time BIGINT;
BEGIN
    IF p_session IS NULL OR p_character IS NULL OR p_world IS NULL OR p_house IS NULL
       OR p_generation IS NULL OR p_lease IS NULL OR p_scope IS NULL
       OR p_transport IS NULL OR octet_length(p_transport) <> 16
       OR p_command IS NULL OR p_command < 1 OR p_command > 18446744073709551615
       OR p_command <> trunc(p_command) OR p_x IS NULL OR p_y IS NULL OR p_z IS NULL
       OR p_variant IS NULL OR p_variant NOT IN ('male','female')
       OR p_skill IS NULL OR p_skill NOT BETWEEN 1 AND 6 THEN
        RETURN 'denied';
    END IF;
    intent := jsonb_build_array(p_character,p_world,p_house,p_generation,p_lease,p_scope,
                               encode(p_transport,'hex'),p_x,p_y,p_z,p_variant,p_skill);
    -- Serialize attempts and replay on the exact session before any effect.
    PERFORM 1 FROM game_durability_reconnect_sessions WHERE game_session_id=p_session FOR UPDATE;
    SELECT binding INTO prior FROM game_house_bed_sleep_receipts
        WHERE game_session_id=p_session AND command_id=p_command;
    IF FOUND THEN
        IF prior = intent THEN RETURN 'slept'; ELSE RETURN 'conflict'; END IF;
    END IF;
    PERFORM 1 FROM game_durability_reconnect_sessions s
        JOIN game_character_roots r ON r.character_id=s.character_id
        JOIN game_runtime_scope_assignments a
          ON a.scope_key=game_house_scope_key(p_world,p_house)
        WHERE s.game_session_id=p_session AND s.character_id=p_character
          AND s.world_id=p_world AND s.current_generation=p_generation
          AND s.character_lease_generation=p_lease AND s.scope_ownership_generation=p_scope
          AND s.session_state IN (1,2) AND s.current_transport_ref=p_transport
          AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=p_world
          AND s.runtime_scope_house_key=p_house AND s.runtime_scope_channel_id IS NULL
          AND s.runtime_scope_instance_id=game_house_scope_instance_id(p_world,p_house)
          AND r.world_id=p_world AND r.account_id=s.account_id AND r.lifecycle=1
          AND EXISTS (SELECT 1 FROM game_durability_session_use_ledgers l
                      JOIN game_durability_session_use_memberships m
                        ON m.character_id=l.character_id AND m.membership_revision=l.revision
                      WHERE l.character_id=p_character AND l.complete AND l.version=1
                        AND m.game_session_id=p_session)
          AND a.scope_kind=2 AND a.world_id=p_world AND a.house_key=p_house
          AND a.state=1 AND a.ownership_generation=p_scope
          AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
        FOR SHARE OF a FOR UPDATE OF r;
    IF NOT FOUND THEN RETURN 'stale'; END IF;

    SELECT world_id,house_key INTO old_world,old_house FROM game_house_bed_sleepers
        WHERE character_id=p_character;
    -- Property -> revisions -> sleepers, in native order, including an older
    -- row in another House. The ACL writer takes property FOR UPDATE on saves.
    PERFORM 1 FROM game_house_ownership
        WHERE (world_id=p_world AND house_key=p_house)
           OR (world_id=old_world AND house_key=old_house)
        ORDER BY world_id,house_key FOR SHARE;
    SELECT owner_character_id INTO owner_id FROM game_house_ownership
        WHERE world_id=p_world AND house_key=p_house;
    IF NOT FOUND THEN RETURN 'denied'; END IF;
    IF owner_id IS DISTINCT FROM p_character THEN
        PERFORM 1 FROM game_house_acl
            WHERE world_id=p_world AND house_key=p_house AND list_id IN (-1,-2)
              AND (allow_everyone OR p_character=ANY(members))
            ORDER BY list_id FOR SHARE;
        IF NOT FOUND THEN RETURN 'denied'; END IF;
    END IF;
    PERFORM 1 FROM game_house_bed_revisions
        WHERE (world_id=p_world AND house_key=p_house)
           OR (world_id=old_world AND house_key=old_house)
        ORDER BY world_id,house_key FOR UPDATE;
    IF EXISTS (SELECT 1 FROM game_house_bed_sleepers
               WHERE world_id=p_world AND house_key=p_house
                 AND bed_x=p_x AND bed_y=p_y AND bed_z=p_z AND freed_at IS NULL
                 AND character_id<>p_character) THEN
        RETURN 'occupied';
    END IF;
    PERFORM 1 FROM game_house_bed_sleepers WHERE character_id=p_character FOR UPDATE;
    DELETE FROM game_house_bed_sleepers WHERE character_id=p_character;
    sleep_time := floor(extract(epoch FROM clock_timestamp())*1000)::BIGINT;
    INSERT INTO game_house_bed_sleepers(character_id,world_id,house_key,bed_x,bed_y,bed_z,
                                      lease_generation,occupied_variant,slept_at)
        VALUES(p_character,p_world,p_house,p_x,p_y,p_z,p_lease,p_variant,sleep_time);
    UPDATE game_durability_reconnect_sessions
        SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL
        WHERE game_session_id=p_session;
    UPDATE game_durability_reconnect_attempts SET state=4
        WHERE game_session_id=p_session AND state IN (1,5);
    INSERT INTO game_house_bed_sleep_receipts VALUES(p_session,p_command,intent,sleep_time);
    RETURN 'slept';
END $$;
REVOKE ALL ON FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC)
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC) TO oteryn_game_runtime;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
