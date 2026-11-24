-- Capture the grant used to reserve a bed. Guild ownership and guild lists
-- both depend on live Guild state. Lock the two possible Guilds in UUID order
-- before the existing access owner locks the property.
CREATE FUNCTION game_house_bed_access(
    p_world UUID, p_house TEXT, p_character UUID
) RETURNS TABLE(role TEXT, acl_revision NUMERIC(20,0), guild_revisions JSONB)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    owner_guild UUID;
    member_guild UUID;
    locked_member UUID;
    member_revision NUMERIC;
BEGIN
    SELECT p.owner_guild_id INTO owner_guild
        FROM game_house_properties p WHERE p.world_id=p_world AND p.house_key=p_house;
    SELECT m.guild_id INTO member_guild FROM game_guild_members m
        WHERE m.character_id=p_character;
    PERFORM g.guild_id FROM game_guilds g
        WHERE g.world_id=p_world AND g.guild_id IN (owner_guild,member_guild)
        ORDER BY g.guild_id FOR SHARE;
    SELECT m.guild_id, g.revision INTO locked_member, member_revision
        FROM game_guild_members m JOIN game_guilds g USING(guild_id)
        WHERE m.character_id=p_character AND m.guild_id=member_guild
          AND g.world_id=p_world AND g.state IN (1,2) FOR SHARE OF m;
    RETURN QUERY
        SELECT a.role, a.acl_revision, COALESCE(a.guild_revisions,
            CASE WHEN locked_member IS NOT NULL AND a.role <> 'OWNER'
                 THEN jsonb_build_object(locked_member::text,member_revision::text) END)
        FROM game_house_access(p_world,p_house,p_character) a
        WHERE a.content_fenced IS FALSE AND (
            a.role = 'OWNER'
            OR locked_member IS NOT NULL
            OR EXISTS (SELECT 1 FROM game_house_acl_entries e
                WHERE e.world_id=p_world AND e.house_key=p_house
                  AND e.character_id=p_character AND e.list_key=lower(a.role))
        );
END $$;

-- Use the same predecessor check for owner wake and the reserved sleep.
CREATE FUNCTION game_house_bed_placement_current(
    p_world UUID, p_house TEXT, p_bundle BYTEA,
    p_head NUMERIC, p_head_revision NUMERIC, p_foot NUMERIC, p_foot_revision NUMERIC
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-base:'||p_world::text||':'||p_house,0));
    RETURN COALESCE((SELECT d.revision FROM game_house_base_decorations d
            WHERE d.world_id=p_world AND d.house_key=p_house AND d.bundle_digest=p_bundle
              AND d.placement_key=p_head),0) = p_head_revision
       AND COALESCE((SELECT d.revision FROM game_house_base_decorations d
            WHERE d.world_id=p_world AND d.house_key=p_house AND d.bundle_digest=p_bundle
              AND d.placement_key=p_foot),0) = p_foot_revision;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_access(UUID,TEXT,UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_house_bed_placement_current(UUID,TEXT,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bed_access(UUID,TEXT,UUID),
    game_house_bed_placement_current(UUID,TEXT,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC)
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_access(UUID,TEXT,UUID),
    game_house_bed_placement_current(UUID,TEXT,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC)
    TO oteryn_game_runtime;

-- Recognize the native House grant when the existing writer settles the
-- reservation, retaining the older spell-list grant for its legacy callers.
DO $migration$
DECLARE
    body TEXT;
    before TEXT := $old$    SELECT world_id,house_key INTO old_world,old_house FROM game_house_bed_sleepers$old$;
    after TEXT := $new$    SELECT EXISTS (SELECT FROM game_house_bed_access(p_world,p_house,p_character))
        INTO native_access;
    SELECT world_id,house_key INTO old_world,old_house FROM game_house_bed_sleepers$new$;
    owner_decl TEXT := '    owner_id UUID;';
    owner_guard TEXT := 'IF owner_id IS DISTINCT FROM p_character THEN';
BEGIN
    body := pg_get_functiondef('game_house_bed_sleep(uuid,uuid,uuid,text,numeric,numeric,numeric,bytea,numeric,integer,integer,integer,text,integer,uuid,numeric)'::regprocedure);
    IF length(body) - length(replace(body,before,'')) <> length(before)
       OR length(body) - length(replace(body,owner_decl,'')) <> length(owner_decl)
       OR length(body) - length(replace(body,owner_guard,'')) <> length(owner_guard) THEN
        RAISE EXCEPTION 'expected one bed sleep access check';
    END IF;
    body := replace(body,owner_decl,'    owner_id UUID; native_access BOOLEAN;');
    body := replace(body,before,after);
    EXECUTE replace(body,owner_guard,'IF owner_id IS DISTINCT FROM p_character AND NOT native_access THEN');
END $migration$;

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
    p_command NUMERIC, p_bundle BYTEA,
    p_head NUMERIC, p_head_revision NUMERIC, p_foot NUMERIC, p_foot_revision NUMERIC,
    p_node UUID, p_registration NUMERIC
) RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    intent JSONB;
    prior game_house_bed_use_receipts%ROWTYPE;
    access_role TEXT;
    result TEXT;
    bed_head BIGINT;
    bed_foot BIGINT;
    head_x INTEGER;
    head_y INTEGER;
    head_z INTEGER;
BEGIN
    IF p_command IS NULL OR p_command NOT BETWEEN 1 AND 18446744073709551615
       OR p_command <> trunc(p_command)
       OR octet_length(p_bundle) IS DISTINCT FROM 32
       OR p_head IS NULL OR p_head NOT BETWEEN 1 AND 281474976710655 OR p_head<>trunc(p_head)
       OR p_foot IS NULL OR p_foot NOT BETWEEN 1 AND 281474976710655 OR p_foot<>trunc(p_foot)
       OR p_head=p_foot
       OR p_head_revision IS NULL OR p_head_revision NOT BETWEEN 0 AND 18446744073709551615
       OR p_head_revision<>trunc(p_head_revision)
       OR p_foot_revision IS NULL OR p_foot_revision NOT BETWEEN 0 AND 18446744073709551615
       OR p_foot_revision<>trunc(p_foot_revision) THEN
        RETURN 'denied';
    END IF;

    -- Placement keys pack the tile and its top-level ordinal.
    bed_head := p_head::bigint;
    bed_foot := p_foot::bigint;
    head_x := (bed_head >> 32)::integer;
    head_y := ((bed_head >> 16) & 65535)::integer;
    head_z := ((bed_head >> 8) & 255)::integer;
    IF head_z > 15 OR head_z <> ((bed_foot >> 8) & 255)
       OR abs(head_x - (bed_foot >> 32)) + abs(head_y - ((bed_foot >> 16) & 65535)) <> 1 THEN
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
                               encode(p_transport,'hex'),encode(p_bundle,'hex'),
                               p_head,p_head_revision,p_foot,p_foot_revision);
    SELECT * INTO prior FROM game_house_bed_use_receipts
        WHERE game_session_id=p_session AND command_id=p_command;
    IF FOUND THEN
        IF prior.binding = intent THEN RETURN prior.outcome; END IF;
        RETURN 'stale';
    END IF;

    -- Native ACL locks current guild (where applicable) and property. Keep
    -- the bed writers' order of ownership anchor -> bed revision -> sleeper.
    SELECT role INTO access_role FROM game_house_bed_access(p_world,p_house,p_character);
    IF NOT FOUND THEN RETURN 'denied'; END IF;
    -- Bed kits replace both parts under this lock. A revision of zero
    -- denotes a placement with no durable decoration yet.
    IF NOT game_house_bed_placement_current(
        p_world,p_house,p_bundle,p_head,p_head_revision,p_foot,p_foot_revision
    ) THEN RETURN 'stale'; END IF;

    PERFORM 1 FROM game_house_ownership WHERE world_id=p_world AND house_key=p_house FOR SHARE;
    PERFORM 1 FROM game_house_bed_revisions
        WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
    IF NOT FOUND THEN RETURN 'denied'; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM game_house_bed_sleepers
          WHERE world_id=p_world AND house_key=p_house AND bed_x=head_x AND bed_y=head_y
            AND bed_z=head_z AND freed_at IS NULL
    ) THEN
        result := 'vacant';
    ELSIF access_role IS DISTINCT FROM 'OWNER' THEN
        result := 'occupied';
    ELSE
        UPDATE game_house_bed_sleepers
            SET freed_at = greatest(slept_at, floor(extract(epoch FROM clock_timestamp())*1000)::bigint),
                freed_cause = 'owner_wake'
            WHERE world_id=p_world AND house_key=p_house AND bed_x=head_x AND bed_y=head_y
              AND bed_z=head_z AND freed_at IS NULL;
        result := 'woken';
    END IF;

    INSERT INTO game_house_bed_use_receipts VALUES(p_session,p_command,intent,result);
    RETURN result;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC) FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_use(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,
    BYTEA,NUMERIC,BYTEA,NUMERIC,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC) TO oteryn_game_runtime;

-- An absent receipt proves rollback only after the original generation
-- has lost control. The shared session lock excludes its active writer.
CREATE FUNCTION game_house_bed_use_receipt(
    p_session UUID, p_character UUID, p_generation NUMERIC,
    p_lease NUMERIC, p_scope NUMERIC, p_transport BYTEA, p_command NUMERIC
) RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE prior game_house_bed_use_receipts%ROWTYPE;
BEGIN
    PERFORM 1 FROM game_durability_reconnect_sessions s
        WHERE s.game_session_id=p_session AND s.character_id=p_character
          AND s.current_generation=p_generation AND s.predecessor_generation=p_generation
          AND s.character_lease_generation=p_lease AND s.scope_ownership_generation=p_scope
          AND s.runtime_scope_kind=2 AND s.session_state=1
          AND s.current_transport_ref IS NULL AND s.control_loss_epoch IS NOT NULL
        FOR SHARE OF s;
    IF NOT FOUND THEN RETURN 'unfenced'; END IF;

    SELECT * INTO prior FROM game_house_bed_use_receipts
        WHERE game_session_id=p_session AND command_id=p_command;
    IF NOT FOUND THEN RETURN 'absent'; END IF;
    IF prior.binding->0 IS DISTINCT FROM to_jsonb(p_character)
       OR prior.binding->3 IS DISTINCT FROM to_jsonb(p_generation)
       OR prior.binding->4 IS DISTINCT FROM to_jsonb(p_lease)
       OR prior.binding->5 IS DISTINCT FROM to_jsonb(p_scope)
       OR prior.binding->6 IS DISTINCT FROM to_jsonb(encode(p_transport,'hex')) THEN
        RETURN 'binding_mismatch';
    END IF;
    RETURN prior.outcome;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC)
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_bed_use_receipt(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC)
    TO oteryn_game_runtime;
