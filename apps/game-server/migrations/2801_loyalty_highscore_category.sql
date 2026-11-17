-- N014 (PLATFORM_SANCTION_LOYALTY_EXPORT_V1, highscores-loyalty-v1.md): Loyalty highscores,
-- category 13, ranked from Platform's existing account loyalty projection (0406/0925).
-- Extend 2727's exact role guard for the one new read-only ranking source.
DO $highscore_role_guard$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job') THEN
        -- Roles are cluster-global: a concurrent migration of another database may create it.
        BEGIN
            CREATE ROLE oteryn_game_highscore_job NOLOGIN NOINHERIT;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job'
          AND (rolcanlogin OR rolinherit OR rolsuper OR rolbypassrls OR rolcreaterole OR rolcreatedb
               OR rolreplication)
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job is not a plain NOLOGIN NOINHERIT group'
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
    -- A member of the group could inherit, SET ROLE to or grant the highscore privileges below.
    -- Its only members may be the runtime group WITH INHERIT FALSE and without ADMIN (granted
    -- below, or already by another Game database of this cluster) and the ADMIN-only grant that
    -- PostgreSQL gives the migration role when a non-superuser CREATEROLE role creates the group.
    IF EXISTS (
        SELECT 1 FROM pg_auth_members m
        JOIN pg_roles g ON g.oid = m.roleid
        JOIN pg_roles r ON r.oid = m.member
        WHERE g.rolname = 'oteryn_game_highscore_job'
          AND NOT (
              (r.rolname = 'oteryn_game_runtime' AND NOT m.inherit_option AND NOT m.admin_option)
              OR (r.rolname = current_user AND NOT m.inherit_option AND NOT m.set_option)
          )
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job has a member other than the runtime group'
            USING ERRCODE = '42501';
    END IF;
    -- A direct privilege would reach beyond the highscore grants below. pg_shdepend records
    -- every object the group owns and every ACL, default ACL or policy naming it (as grantee or
    -- grantor), in this database and on shared objects (databases, tablespaces, parameters);
    -- another Game database's own highscore grants are its business. The group may own nothing
    -- and appear in no ACL except the exact table-level grants below, without GRANT OPTION and
    -- never as a grantor; the highscore tables and their sequence do not exist yet on a first
    -- run.
    IF EXISTS (
        WITH job AS (
            SELECT oid FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job'
        ), expected AS (
            SELECT to_regclass(t.name)::oid AS relation, t.privileges
            FROM (VALUES
                ('game_world_highscore_snapshots', ARRAY['SELECT', 'INSERT', 'DELETE']),
                ('game_world_highscore_rows', ARRAY['SELECT', 'INSERT', 'DELETE']),
                ('game_world_highscore_heads', ARRAY['SELECT', 'INSERT', 'UPDATE']),
                ('game_character_roots', ARRAY['SELECT']),
                ('game_character_build_state', ARRAY['SELECT']),
                ('game_character_progression_state', ARRAY['SELECT']),
                ('game_account_achievements', ARRAY['SELECT']),
                ('game_account_achievement_grant_requests', ARRAY['SELECT']),
                ('game_character_bestiary_progress', ARRAY['SELECT']),
                ('game_character_bosstiary_progress', ARRAY['SELECT']),
                ('game_tibiadrome_scores', ARRAY['SELECT']),
                ('game_character_boss_difficulty_records', ARRAY['SELECT']),
                ('game_boss_room_catalogue', ARRAY['SELECT']),
                ('game_account_loyalty_projections', ARRAY['SELECT'])
            ) AS t(name, privileges)
            WHERE to_regclass(t.name) IS NOT NULL
            UNION ALL
            SELECT d.objid, ARRAY['USAGE']
            FROM pg_depend d
            WHERE d.classid = 'pg_class'::regclass
              AND d.refclassid = 'pg_class'::regclass
              AND d.refobjid = to_regclass('game_world_highscore_snapshots')::oid
              AND d.deptype = 'i'
              AND EXISTS (SELECT 1 FROM pg_class q WHERE q.oid = d.objid AND q.relkind = 'S')
        )
        SELECT 1 FROM pg_shdepend s, job
        WHERE s.refclassid = 'pg_authid'::regclass AND s.refobjid = job.oid
          AND s.dbid IN (0, (SELECT oid FROM pg_database WHERE datname = current_database()))
          AND NOT (s.deptype = 'a' AND s.objsubid = 0 AND (
                   (s.classid = 'pg_class'::regclass
                    AND EXISTS (SELECT 1 FROM expected e WHERE e.relation = s.objid))
                   OR (s.classid = 'pg_proc'::regclass
                       AND s.objid = 'game_account_closure_detached(uuid)'::regprocedure)))
        UNION ALL
        SELECT 1 FROM expected e JOIN pg_class c ON c.oid = e.relation
        CROSS JOIN LATERAL aclexplode(c.relacl) a, job
        WHERE a.grantor = job.oid
           OR (a.grantee = job.oid
               AND (a.is_grantable OR NOT (a.privilege_type = ANY (e.privileges))))
        UNION ALL
        SELECT 1 FROM pg_proc p CROSS JOIN LATERAL aclexplode(p.proacl) a, job
        WHERE p.oid = 'game_account_closure_detached(uuid)'::regprocedure
          AND (a.grantor = job.oid
               OR (a.grantee = job.oid AND (a.is_grantable OR a.privilege_type <> 'EXECUTE')))
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job holds a privilege outside the highscore grants'
            USING ERRCODE = '42501';
    END IF;
END $highscore_role_guard$;

GRANT SELECT ON game_account_loyalty_projections TO oteryn_game_highscore_job;

-- Loyalty category admission for the Game snapshot store. 1105 already admits 13 on the public
-- projection metadata table; the public export of category 13 stays withheld below.
ALTER TABLE game_world_highscore_snapshots
    DROP CONSTRAINT game_world_highscore_snapshots_category_check,
    ADD CHECK (category BETWEEN 1 AND 14 OR category IN (16,17));
ALTER TABLE game_world_highscore_heads
    DROP CONSTRAINT game_world_highscore_heads_category_check,
    ADD CHECK (category BETWEEN 1 AND 14 OR category IN (16,17));

-- Loyalty is Platform-owned account state: Game ranks it for its own clients only and never
-- republishes it through OTERYN_GAME_PUBLIC_PROJECTIONS_V1, whose highscore domain stays
-- 1..=12 | 14 | 16 | 17. The snapshot trigger and the closure trigger call this for every
-- category; category 13 queues no public revision. Otherwise unchanged from 0091.
CREATE OR REPLACE FUNCTION game_public_highscores_touch(p_world UUID, p_category SMALLINT)
RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_revision BIGINT;
BEGIN
    IF p_category = 13 THEN
        RETURN;
    END IF;
    INSERT INTO game_public_highscores(world_id, category, projection_revision, revised_at)
    VALUES (p_world, p_category, 1, v_now)
    ON CONFLICT (world_id, category) DO UPDATE
        SET projection_revision = game_public_highscores.projection_revision + 1,
            revised_at = v_now
    RETURNING projection_revision INTO v_revision;
    INSERT INTO game_public_highscore_outbox(world_id, category, projection_epoch,
        projection_revision, created_at)
    SELECT p_world, p_category, projection_epoch, v_revision, v_now
    FROM game_public_projection_epoch WHERE epoch_scope = 1;
END; $$;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_public_highscores_touch(uuid, smallint) SET search_path = %I, pg_temp',
        current_schema());
END $$;
