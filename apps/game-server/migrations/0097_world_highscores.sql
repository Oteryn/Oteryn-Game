-- RANK-1 / HS-1: the World highscore snapshots (HIGHSCORES0 §§3-6). A rebuildable, committed
-- projection of character state; never gameplay authority.
--
-- Scope of this migration only:
--   * `game_world_highscore_snapshots`: one immutable header per published (World, category)
--     cut, categories 1..=11;
--   * `game_world_highscore_rows`: the ranked rows of a snapshot, one list per vocation filter
--     (0 all, 1 knight .. 5 monk), at most 1000 positions each, immutable once written;
--   * `game_world_highscore_heads`: the current and previous snapshot of each (World, category).
--     A reader pages the current snapshot or, by its token, the previous one;
--   * the guards: a header and a row never change; a published snapshot never acquires rows;
--     a row leaves only with its header; no table can be truncated;
--   * the public projection attachment (0091 §11): each snapshot insert advances the public
--     (World, category) revision in the job's own transaction, so a failed cut publishes none;
--   * the job's role (HIGHSCORES0 §4): the NOLOGIN group `oteryn_game_highscore_job` alone
--     writes and deletes the snapshot tables and reads their committed sources. A pre-existing
--     role is refused unless it is a plain NOLOGIN group that is a member of no role. The
--     runtime group is its member WITH INHERIT FALSE, SET TRUE: a runtime login holds none of its
--     privileges and gains them only for the job's own transaction (`SET LOCAL ROLE`). The
--     runtime and control roles SELECT the snapshot tables only.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- trigger, the three tables and the two guard functions, and revokes the job role's grants (the
-- cluster-global role itself may be shared with another Game database). The public projection then reads no
-- highscores source again (`to_regclass` in 0091).

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job') THEN
        -- Roles are cluster-global: a concurrent migration of another database may create it.
        BEGIN
            CREATE ROLE oteryn_game_highscore_job NOLOGIN;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job'
          AND (rolcanlogin OR rolsuper OR rolbypassrls OR rolcreaterole OR rolcreatedb OR rolreplication)
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job is not a plain NOLOGIN group'
            USING ERRCODE = '42501';
    END IF;
    -- A membership of its own would let the job's transaction inherit, or SET ROLE to,
    -- privileges beyond the highscore grants below: the group must be a member of no role.
    IF EXISTS (
        SELECT 1 FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member
        WHERE r.rolname = 'oteryn_game_highscore_job'
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job is a member of another role'
            USING ERRCODE = '42501';
    END IF;
END $$;
GRANT oteryn_game_highscore_job TO oteryn_game_runtime WITH INHERIT FALSE, SET TRUE;

CREATE TABLE game_world_highscore_snapshots (
    snapshot_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    world_id UUID NOT NULL,
    category INTEGER NOT NULL CHECK (category BETWEEN 1 AND 11),
    computed_at_ms BIGINT NOT NULL CHECK (computed_at_ms >= 0),
    source_cut TEXT NOT NULL,
    row_count INTEGER NOT NULL CHECK (row_count BETWEEN 0 AND 6000),
    UNIQUE (world_id, category, snapshot_id)
);

CREATE TABLE game_world_highscore_rows (
    snapshot_id BIGINT NOT NULL
        REFERENCES game_world_highscore_snapshots ON DELETE CASCADE,
    filter SMALLINT NOT NULL CHECK (filter BETWEEN 0 AND 5),
    position INTEGER NOT NULL CHECK (position BETWEEN 1 AND 1000),
    rank BIGINT NOT NULL CHECK (rank >= 1),
    character_id UUID NOT NULL,
    name TEXT NOT NULL,
    vocation TEXT NOT NULL,
    level BIGINT NOT NULL CHECK (level BETWEEN 1 AND 4294967295),
    value_primary BIGINT NOT NULL CHECK (value_primary >= 0),
    value_progress BIGINT NOT NULL CHECK (value_progress >= 0),
    PRIMARY KEY (snapshot_id, filter, position),
    UNIQUE (snapshot_id, filter, character_id)
);

CREATE TABLE game_world_highscore_heads (
    world_id UUID NOT NULL,
    category INTEGER NOT NULL CHECK (category BETWEEN 1 AND 11),
    current_snapshot_id BIGINT,
    previous_snapshot_id BIGINT,
    PRIMARY KEY (world_id, category),
    CHECK (previous_snapshot_id IS NULL OR current_snapshot_id <> previous_snapshot_id),
    FOREIGN KEY (world_id, category, current_snapshot_id)
        REFERENCES game_world_highscore_snapshots (world_id, category, snapshot_id),
    FOREIGN KEY (world_id, category, previous_snapshot_id)
        REFERENCES game_world_highscore_snapshots (world_id, category, snapshot_id)
);

CREATE FUNCTION game_highscore_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'immutable highscore projection row' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_highscore_header_immutable BEFORE UPDATE
    ON game_world_highscore_snapshots
    FOR EACH ROW EXECUTE FUNCTION game_highscore_immutable();
CREATE TRIGGER game_highscore_row_immutable BEFORE UPDATE
    ON game_world_highscore_rows
    FOR EACH ROW EXECUTE FUNCTION game_highscore_immutable();

-- A published snapshot never acquires rows, and rows leave only with their header (the
-- cascade of the header's delete).
CREATE FUNCTION game_highscore_row_membership_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF EXISTS (SELECT 1 FROM game_world_highscore_heads
                    WHERE current_snapshot_id = NEW.snapshot_id
                       OR previous_snapshot_id = NEW.snapshot_id) THEN
            RAISE EXCEPTION 'published snapshot cannot acquire rows' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF EXISTS (SELECT 1 FROM game_world_highscore_snapshots
                WHERE snapshot_id = OLD.snapshot_id) THEN
        RAISE EXCEPTION 'snapshot rows only delete with their header' USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER game_highscore_row_membership_guard BEFORE INSERT OR DELETE
    ON game_world_highscore_rows
    FOR EACH ROW EXECUTE FUNCTION game_highscore_row_membership_guard();

CREATE TRIGGER game_world_highscore_snapshots_no_truncate BEFORE TRUNCATE
    ON game_world_highscore_snapshots
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_world_highscore_rows_no_truncate BEFORE TRUNCATE
    ON game_world_highscore_rows
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_world_highscore_heads_no_truncate BEFORE TRUNCATE
    ON game_world_highscore_heads
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

-- 0091 §11 interface: the snapshot insert and the public revision commit together.
CREATE TRIGGER game_public_highscores_snapshot_insert AFTER INSERT
    ON game_world_highscore_snapshots
    FOR EACH ROW EXECUTE FUNCTION game_public_highscores_snapshot_trigger();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_highscore_immutable() SET search_path = %I, pg_temp',
        current_schema());
    EXECUTE format('ALTER FUNCTION game_highscore_row_membership_guard() SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON game_world_highscore_snapshots, game_world_highscore_rows,
    game_world_highscore_heads FROM PUBLIC;
REVOKE ALL ON FUNCTION game_highscore_immutable(), game_highscore_row_membership_guard()
    FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_world_highscore_snapshots, game_world_highscore_rows
    TO oteryn_game_highscore_job;
GRANT SELECT, INSERT, UPDATE ON game_world_highscore_heads TO oteryn_game_highscore_job;
DO $$ BEGIN
    EXECUTE format('GRANT USAGE ON SEQUENCE %s TO oteryn_game_highscore_job',
        pg_get_serial_sequence('game_world_highscore_snapshots', 'snapshot_id'));
END $$;
-- The committed sources of categories 1-11 (HIGHSCORES0 §3), read in the job's cut.
GRANT SELECT ON game_character_roots, game_character_build_state,
    game_character_progression_state, game_account_achievements,
    game_account_achievement_grant_requests, game_character_bestiary_progress
    TO oteryn_game_highscore_job;
GRANT SELECT ON game_world_highscore_snapshots, game_world_highscore_rows,
    game_world_highscore_heads TO oteryn_game_runtime, oteryn_game_control;
