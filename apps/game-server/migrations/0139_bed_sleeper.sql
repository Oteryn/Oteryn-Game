-- BED-1b storage checkpoint. 0139 is provisional until queue allocation.
-- No direct runtime table writes are granted. The BED-1d participant below owns
-- a fenced terminal session plus sleeper; OFFLINE markers/activation and native
-- House resolver/reservation qualification remain prerequisites for gameplay.
-- Coordinates in this table are identity, never placement proof.
CREATE TABLE game_house_bed_revisions (
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    revision NUMERIC(20,0) NOT NULL CHECK (revision BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY (world_id, house_key),
    FOREIGN KEY (world_id, house_key) REFERENCES game_house_ownership(world_id, house_key)
);

CREATE TABLE game_house_bed_sleepers (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    bed_x INTEGER NOT NULL,
    bed_y INTEGER NOT NULL,
    bed_z INTEGER NOT NULL,
    lease_generation NUMERIC(20,0) NOT NULL
        CHECK (lease_generation BETWEEN 1 AND 18446744073709551615),
    occupied_variant TEXT NOT NULL CHECK (occupied_variant IN ('male', 'female')),
    -- Native build timestamps are Unix milliseconds, including zero at the epoch.
    slept_at BIGINT NOT NULL CHECK (slept_at >= 0),
    freed_at BIGINT,
    freed_cause TEXT,
    FOREIGN KEY (world_id, house_key) REFERENCES game_house_bed_revisions(world_id, house_key),
    CHECK ((freed_at IS NULL AND freed_cause IS NULL)
        OR (freed_at IS NOT NULL AND freed_cause IS NOT NULL AND freed_at >= slept_at
            AND freed_cause IN ('owner_wake', 'owner_change', 'disposition', 'bed_removed')))
);
CREATE UNIQUE INDEX game_house_bed_open_sleeper
    ON game_house_bed_sleepers(world_id, house_key, bed_x, bed_y, bed_z)
    WHERE freed_at IS NULL;

REVOKE ALL ON game_house_bed_revisions, game_house_bed_sleepers
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;

-- There is always a revision row, even for a property that has never had a sleeper.
INSERT INTO game_house_bed_revisions(world_id, house_key, revision)
    SELECT world_id, house_key, 1 FROM game_house_ownership;
CREATE FUNCTION game_house_bed_initialize() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_house_bed_revisions(world_id, house_key, revision)
        VALUES (NEW.world_id, NEW.house_key, 1);
    RETURN NULL;
END $$;
CREATE TRIGGER game_house_bed_initialize AFTER INSERT ON game_house_ownership
    FOR EACH ROW EXECUTE FUNCTION game_house_bed_initialize();

CREATE FUNCTION game_house_bed_sleeper_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM 1 FROM game_character_roots
            WHERE character_id = NEW.character_id AND world_id = NEW.world_id FOR SHARE;
        IF NOT FOUND OR NEW.freed_at IS NOT NULL OR NEW.freed_cause IS NOT NULL THEN
            RAISE EXCEPTION 'BED_SLEEPER_INVALID_IDENTITY' USING ERRCODE = '23514';
        END IF;
    ELSIF (to_jsonb(NEW) - 'freed_at' - 'freed_cause')
            IS DISTINCT FROM (to_jsonb(OLD) - 'freed_at' - 'freed_cause')
        OR OLD.freed_at IS NOT NULL OR NEW.freed_at IS NULL OR NEW.freed_cause IS NULL THEN
        RAISE EXCEPTION 'BED_SLEEPER_NOT_OPEN_SUCCESSOR' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_bed_sleeper_guard BEFORE INSERT OR UPDATE ON game_house_bed_sleepers
    FOR EACH ROW EXECUTE FUNCTION game_house_bed_sleeper_guard();

-- Future caller-TX writers must lock property, then all affected House revisions
-- in native (world_id, house_key) order, then sleepers before mutating them.
-- The trigger makes each successful row transition advance occupancy atomically;
-- rejected statements, unique conflicts and transaction rollback advance nothing.
CREATE FUNCTION game_house_bed_advance_revision() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        UPDATE game_house_bed_revisions SET revision = revision + 1
            WHERE world_id = OLD.world_id AND house_key = OLD.house_key;
    ELSE
        UPDATE game_house_bed_revisions SET revision = revision + 1
            WHERE world_id = NEW.world_id AND house_key = NEW.house_key;
    END IF;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'BED_SLEEPER_REVISION_MISSING' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER game_house_bed_advance_revision AFTER INSERT OR UPDATE OR DELETE
    ON game_house_bed_sleepers FOR EACH ROW EXECUTE FUNCTION game_house_bed_advance_revision();

-- The native ownership writer is the only activated producer in this checkpoint.
-- Specific-cause frees MUST precede ownership UPDATE in the same transaction;
-- this fallback preserves already-freed rows and their first cause/time.
CREATE FUNCTION game_house_bed_owner_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE wake_at BIGINT;
BEGIN
    IF NEW.owner_character_id IS NOT DISTINCT FROM OLD.owner_character_id THEN
        RETURN NULL;
    END IF;
    PERFORM 1 FROM game_house_bed_revisions
        WHERE world_id = NEW.world_id AND house_key = NEW.house_key FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'BED_SLEEPER_REVISION_MISSING' USING ERRCODE = '23514';
    END IF;
    wake_at := floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT;
    UPDATE game_house_bed_sleepers SET freed_at = wake_at, freed_cause = 'owner_change'
        WHERE world_id = NEW.world_id AND house_key = NEW.house_key AND freed_at IS NULL;
    RETURN NULL;
END $$;
CREATE TRIGGER game_house_bed_owner_changed AFTER UPDATE OF owner_character_id
    ON game_house_ownership FOR EACH ROW EXECUTE FUNCTION game_house_bed_owner_changed();

CREATE TRIGGER game_house_bed_sleepers_no_truncate BEFORE TRUNCATE ON game_house_bed_sleepers
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_house_bed_revisions_no_truncate BEFORE TRUNCATE ON game_house_bed_revisions
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Trigger-only functions have no directly callable role surface. Every definer
-- uses the migration's own schema, with pg_temp explicitly last.
REVOKE ALL ON FUNCTION game_house_bed_initialize(), game_house_bed_sleeper_guard(),
    game_house_bed_advance_revision(), game_house_bed_owner_changed()
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
DO $$
DECLARE function_name TEXT;
BEGIN
    FOREACH function_name IN ARRAY ARRAY['game_house_bed_initialize', 'game_house_bed_sleeper_guard',
        'game_house_bed_advance_revision', 'game_house_bed_owner_changed'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',
            function_name, current_schema());
    END LOOP;
END $$;


-- BED-1d durable participant. This receipt is NOT the OFFLINE build marker or
-- activation. Only an owning House runtime with resolved placement/reservation
-- and native sex may submit it; no production Channel owner exposes that path.
CREATE TABLE game_house_bed_sleep_receipts (
    game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    binding JSONB NOT NULL CHECK (octet_length(binding::text) <= 4096),
    slept_at BIGINT NOT NULL CHECK (slept_at >= 0),
    PRIMARY KEY (game_session_id, command_id)
);
REVOKE ALL ON game_house_bed_sleep_receipts FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
CREATE FUNCTION game_house_bed_sleep_receipt_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'BED_SLEEP_RECEIPT_IMMUTABLE' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER game_house_bed_sleep_receipt_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_house_bed_sleep_receipts FOR EACH STATEMENT
    EXECUTE FUNCTION game_house_bed_sleep_receipt_immutable();

-- This narrow definer is the only runtime sleeper mutation surface. Direct
-- sleeper/revision writes remain revoked. Refusals do not create receipts.
CREATE FUNCTION game_house_bed_sleep(
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
          AND s.session_state=1 AND s.current_transport_ref=p_transport
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
REVOKE ALL ON FUNCTION game_house_bed_sleep_receipt_immutable()
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_house_bed_sleep(UUID,UUID,UUID,TEXT,NUMERIC,NUMERIC,NUMERIC,BYTEA,NUMERIC,INTEGER,INTEGER,INTEGER,TEXT,INTEGER,UUID,NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_house_bed_sleep_receipt_immutable() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
