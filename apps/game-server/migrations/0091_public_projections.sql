-- GAME-PUBLIC-PROJ-PRODUCER-1: the Game producer state of the public projections
-- (`docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §5) for `character_profile`,
-- `highscores` and `world_online`.
--
--   * `game_public_projection_epoch`: the single `projection_epoch` of the public feed. It is
--     separate from the LCFA epoch (U-PP5), so a public raise never invalidates LCFA and the
--     reverse; its external fence F_pub is a file of its own.
--   * `game_public_character_profiles` and `game_public_character_profile_outbox`: the
--     per-Character revision and the changes awaiting an acknowledged snapshot. Definer triggers
--     on the Character root, progression and build state advance it in the writing transaction.
--     There is no foreign key to the root, so a deleted Character keeps its revision and
--     publishes `profile: []`. `vacated_name_key` records the name key a root deletion or rename
--     gave up; the publisher holds a new holder of that key until the row is acknowledged or
--     is older than the bounded handoff hold, so a rename cycle cannot hold delivery forever
--     (contract §7.3 name handoff). A resync clears every recorded key.
--   * `game_public_highscores` and `game_public_highscore_outbox`: the per-(World, category)
--     revision. HS-1 advances it by attaching `game_public_highscores_snapshot_trigger()` to its
--     snapshot table (one revision per committed job run, for all six filters).
--   * `game_public_world_online`: the publisher's own last `world_online` snapshot per World
--     (revision, content digest, observed time, acknowledged). It is the only table the runtime
--     writes, so a revision is never reused for different content across restarts.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- triggers, the tables and the functions. The projections are derived state: dropping them loses
-- no authority, and Platform's read models go stale through the watermark.

CREATE TABLE game_public_projection_epoch (
    epoch_scope SMALLINT PRIMARY KEY CHECK (epoch_scope = 1),
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    raised_at BIGINT NOT NULL CHECK (raised_at >= 0)
);
INSERT INTO game_public_projection_epoch VALUES (1, 1, 0);

CREATE TABLE game_public_character_profiles (
    character_id UUID PRIMARY KEY,
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    revised_at BIGINT NOT NULL CHECK (revised_at >= 0)
);

CREATE TABLE game_public_character_profile_outbox (
    character_id UUID NOT NULL REFERENCES game_public_character_profiles(character_id),
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    created_at BIGINT NOT NULL CHECK (created_at >= 0),
    vacated_name_key TEXT NULL,
    PRIMARY KEY (character_id, projection_epoch, projection_revision)
);
CREATE INDEX game_public_character_profile_outbox_age
    ON game_public_character_profile_outbox (created_at, character_id);
CREATE INDEX game_public_character_profile_outbox_vacated
    ON game_public_character_profile_outbox (vacated_name_key)
    WHERE vacated_name_key IS NOT NULL;

CREATE TABLE game_public_highscores (
    world_id UUID NOT NULL,
    category SMALLINT NOT NULL CHECK (category BETWEEN 1 AND 11),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    revised_at BIGINT NOT NULL CHECK (revised_at >= 0),
    PRIMARY KEY (world_id, category)
);

CREATE TABLE game_public_highscore_outbox (
    world_id UUID NOT NULL,
    category SMALLINT NOT NULL,
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    created_at BIGINT NOT NULL CHECK (created_at >= 0),
    PRIMARY KEY (world_id, category, projection_epoch, projection_revision),
    FOREIGN KEY (world_id, category) REFERENCES game_public_highscores(world_id, category)
);
CREATE INDEX game_public_highscore_outbox_age
    ON game_public_highscore_outbox (created_at, world_id, category);

-- No CHECK here calls a function: the runtime role inserts and updates these rows.
CREATE TABLE game_public_world_online (
    world_id UUID PRIMARY KEY,
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    characters_digest BYTEA NOT NULL CHECK (octet_length(characters_digest) = 32),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    acked BOOLEAN NOT NULL
);

-- Characters committed before this migration (preproduction stores) are queued once at
-- revision 1, so the initial fill needs no operator step and the watermark holds until then.
INSERT INTO game_public_character_profiles(character_id, projection_revision, revised_at)
SELECT character_id, 1, floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT
FROM game_character_roots;
INSERT INTO game_public_character_profile_outbox(character_id, projection_epoch,
    projection_revision, created_at)
SELECT character_id, 1, 1, revised_at FROM game_public_character_profiles;

CREATE FUNCTION game_public_profile_touch(p_character UUID, p_vacated TEXT) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_revision BIGINT;
BEGIN
    INSERT INTO game_public_character_profiles(character_id, projection_revision, revised_at)
    VALUES (p_character, 1, v_now)
    ON CONFLICT (character_id) DO UPDATE
        SET projection_revision = game_public_character_profiles.projection_revision + 1,
            revised_at = v_now
    RETURNING projection_revision INTO v_revision;
    INSERT INTO game_public_character_profile_outbox(character_id, projection_epoch,
        projection_revision, created_at, vacated_name_key)
    SELECT p_character, projection_epoch, v_revision, v_now, p_vacated
    FROM game_public_projection_epoch WHERE epoch_scope = 1;
END; $$;

-- SECURITY DEFINER: no writer role holds a privilege on the projection tables, so a profile
-- revision advances only with a committed change of a projected field (contract §5 table).
CREATE FUNCTION game_public_profile_root_trigger() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM game_public_profile_touch(NEW.character_id, NULL);
    ELSIF TG_OP = 'DELETE' THEN
        PERFORM game_public_profile_touch(OLD.character_id, OLD.name_key);
    ELSE
        IF NEW.character_id <> OLD.character_id THEN
            PERFORM game_public_profile_touch(OLD.character_id, OLD.name_key);
        END IF;
        PERFORM game_public_profile_touch(NEW.character_id,
            CASE WHEN NEW.name_key IS DISTINCT FROM OLD.name_key
                      OR NEW.character_id <> OLD.character_id
                 THEN OLD.name_key END);
    END IF;
    RETURN NULL;
END; $$;
CREATE TRIGGER game_public_profile_root_insert_delete AFTER INSERT OR DELETE
    ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_public_profile_root_trigger();
CREATE TRIGGER game_public_profile_root_update AFTER UPDATE ON game_character_roots
    FOR EACH ROW WHEN ((NEW.character_id, NEW.world_id, NEW.lifecycle, NEW.name)
                       IS DISTINCT FROM (OLD.character_id, OLD.world_id, OLD.lifecycle, OLD.name))
    EXECUTE FUNCTION game_public_profile_root_trigger();

-- Level (progression) and vocation (build): an insert, a deletion or a change of the field.
CREATE FUNCTION game_public_profile_state_trigger() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        PERFORM game_public_profile_touch(OLD.character_id, NULL);
    END IF;
    IF TG_OP = 'INSERT' OR (TG_OP = 'UPDATE' AND NEW.character_id <> OLD.character_id) THEN
        PERFORM game_public_profile_touch(NEW.character_id, NULL);
    END IF;
    RETURN NULL;
END; $$;
CREATE TRIGGER game_public_profile_progression_insert_delete AFTER INSERT OR DELETE
    ON game_character_progression_state
    FOR EACH ROW EXECUTE FUNCTION game_public_profile_state_trigger();
CREATE TRIGGER game_public_profile_progression_update AFTER UPDATE
    ON game_character_progression_state
    FOR EACH ROW WHEN ((NEW.character_id, NEW.level) IS DISTINCT FROM (OLD.character_id, OLD.level))
    EXECUTE FUNCTION game_public_profile_state_trigger();
CREATE TRIGGER game_public_profile_build_insert_delete AFTER INSERT OR DELETE
    ON game_character_build_state
    FOR EACH ROW EXECUTE FUNCTION game_public_profile_state_trigger();
CREATE TRIGGER game_public_profile_build_update AFTER UPDATE ON game_character_build_state
    FOR EACH ROW WHEN ((NEW.character_id, NEW.vocation)
                       IS DISTINCT FROM (OLD.character_id, OLD.vocation))
    EXECUTE FUNCTION game_public_profile_state_trigger();

CREATE FUNCTION game_public_highscores_touch(p_world UUID, p_category SMALLINT) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_revision BIGINT;
BEGIN
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

-- HS-1 interface (contract §11): `AFTER INSERT ON game_world_highscore_snapshots FOR EACH ROW
-- EXECUTE FUNCTION game_public_highscores_snapshot_trigger()`. The job's insert of a snapshot
-- and this touch commit together, whatever role the job runs as.
CREATE FUNCTION game_public_highscores_snapshot_trigger() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    PERFORM game_public_highscores_touch(NEW.world_id, NEW.category::SMALLINT);
    RETURN NULL;
END; $$;

-- Operator-only resync (§5 Resync) of one family or of all. With `p_raise_epoch` (only with
-- 'all') it first raises the epoch to max(current + 1, now in Unix ms), as LCFA's resync does.
-- Profiles and highscores are touched at a new revision, so the queued rows carry the resync
-- start and the §5.1 watermark stays below it until every subject is acknowledged; every
-- `world_online` snapshot is marked unacknowledged, so the next scan sends it again (a new
-- revision when the epoch changed). A profile resync also clears every recorded
-- `vacated_name_key`: the operator's re-publication of every profile supersedes the handoff
-- order. SECURITY INVOKER: it has no grant, so only the owner login runs it, with the owner's
-- own privileges.
CREATE FUNCTION game_public_projection_resync(p_family TEXT, p_raise_epoch BOOLEAN)
RETURNS BIGINT
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_epoch BIGINT;
    v_character UUID;
    v_world UUID;
    v_category SMALLINT;
    v_pairs TEXT := 'SELECT world_id, category FROM game_public_highscores';
BEGIN
    IF p_family IS NULL OR p_raise_epoch IS NULL THEN
        RAISE EXCEPTION 'resync requires a family and an explicit epoch choice'
            USING ERRCODE = '22004';
    END IF;
    IF p_family NOT IN ('all', 'highscores', 'character_profile', 'world_online')
       OR (p_raise_epoch AND p_family <> 'all') THEN
        RAISE EXCEPTION 'unknown family, or an epoch raise of one family'
            USING ERRCODE = '22023';
    END IF;
    SELECT projection_epoch INTO v_epoch FROM game_public_projection_epoch
        WHERE epoch_scope = 1 FOR UPDATE;
    IF p_raise_epoch THEN
        v_epoch := greatest(v_epoch + 1, v_now);
        UPDATE game_public_projection_epoch
           SET projection_epoch = v_epoch, raised_at = v_now WHERE epoch_scope = 1;
    END IF;
    IF p_family IN ('all', 'character_profile') THEN
        UPDATE game_public_character_profile_outbox SET vacated_name_key = NULL
            WHERE vacated_name_key IS NOT NULL;
        FOR v_character IN
            SELECT character_id FROM game_character_roots
            UNION SELECT character_id FROM game_public_character_profiles
            ORDER BY 1
        LOOP
            PERFORM game_public_profile_touch(v_character, NULL);
        END LOOP;
    END IF;
    IF p_family IN ('all', 'highscores') THEN
        IF to_regclass('game_world_highscore_snapshots') IS NOT NULL THEN
            v_pairs := v_pairs || ' UNION SELECT world_id, category::SMALLINT'
                || ' FROM game_world_highscore_snapshots';
        END IF;
        FOR v_world, v_category IN EXECUTE v_pairs || ' ORDER BY 1, 2' LOOP
            PERFORM game_public_highscores_touch(v_world, v_category);
        END LOOP;
    END IF;
    IF p_family IN ('all', 'world_online') THEN
        UPDATE game_public_world_online SET acked = false WHERE acked;
    END IF;
    RETURN v_epoch;
END; $$;

-- The epoch only rises; a revision only advances by one; a World's online snapshot keeps its
-- pair and content except to advance by one revision; no projection table truncates.
CREATE FUNCTION game_public_projection_epoch_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.epoch_scope = OLD.epoch_scope
       AND NEW.projection_epoch > OLD.projection_epoch THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'public projection epoch only rises' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_public_projection_epoch_guard BEFORE UPDATE OR DELETE
    ON game_public_projection_epoch
    FOR EACH ROW EXECUTE FUNCTION game_public_projection_epoch_guard();

CREATE FUNCTION game_public_profile_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.projection_revision = OLD.projection_revision + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'profile revision only advances by one' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_public_profile_revision_guard BEFORE UPDATE OR DELETE
    ON game_public_character_profiles
    FOR EACH ROW EXECUTE FUNCTION game_public_profile_revision_guard();

CREATE FUNCTION game_public_highscores_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.world_id = OLD.world_id AND NEW.category = OLD.category
       AND NEW.projection_revision = OLD.projection_revision + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'highscores revision only advances by one' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_public_highscores_revision_guard BEFORE UPDATE OR DELETE
    ON game_public_highscores
    FOR EACH ROW EXECUTE FUNCTION game_public_highscores_revision_guard();

CREATE FUNCTION game_public_world_online_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.world_id = OLD.world_id
       AND NEW.projection_epoch >= OLD.projection_epoch
       AND (NEW.projection_revision = OLD.projection_revision + 1
            OR (NEW.projection_revision = OLD.projection_revision
                AND NEW.projection_epoch = OLD.projection_epoch
                AND NEW.characters_digest = OLD.characters_digest
                AND NEW.source_observed_at = OLD.source_observed_at)) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'world online snapshot only advances by one revision'
        USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_public_world_online_guard BEFORE UPDATE OR DELETE
    ON game_public_world_online
    FOR EACH ROW EXECUTE FUNCTION game_public_world_online_guard();

CREATE TRIGGER game_public_projection_epoch_no_truncate BEFORE TRUNCATE
    ON game_public_projection_epoch EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_character_profiles_no_truncate BEFORE TRUNCATE
    ON game_public_character_profiles EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_highscores_no_truncate BEFORE TRUNCATE
    ON game_public_highscores EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_world_online_no_truncate BEFORE TRUNCATE
    ON game_public_world_online EXECUTE FUNCTION game_character_reject_truncate();
-- Rows leave an outbox only by an acknowledgement; a truncation would let the watermark pass
-- undelivered changes.
CREATE TRIGGER game_public_character_profile_outbox_no_truncate BEFORE TRUNCATE
    ON game_public_character_profile_outbox EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_public_highscore_outbox_no_truncate BEFORE TRUNCATE
    ON game_public_highscore_outbox EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_public_profile_touch(uuid, text)',
        'game_public_profile_root_trigger()',
        'game_public_profile_state_trigger()',
        'game_public_highscores_touch(uuid, smallint)',
        'game_public_highscores_snapshot_trigger()',
        'game_public_projection_resync(text, boolean)',
        'game_public_projection_epoch_guard()',
        'game_public_profile_revision_guard()',
        'game_public_highscores_revision_guard()',
        'game_public_world_online_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_public_projection_epoch, game_public_character_profiles,
    game_public_character_profile_outbox, game_public_highscores, game_public_highscore_outbox,
    game_public_world_online FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_public_profile_touch(uuid, text),
    game_public_profile_root_trigger(),
    game_public_profile_state_trigger(),
    game_public_highscores_touch(uuid, smallint),
    game_public_highscores_snapshot_trigger(),
    game_public_projection_resync(text, boolean),
    game_public_projection_epoch_guard(),
    game_public_profile_revision_guard(),
    game_public_highscores_revision_guard(),
    game_public_world_online_guard()
FROM PUBLIC;
-- Runtime publisher: read the projection state, clear acknowledged outbox rows and keep its own
-- `world_online` snapshots. The resync has no grant: only the owner login runs it, as invoker.
GRANT SELECT ON game_public_projection_epoch, game_public_character_profiles,
    game_public_highscores TO oteryn_game_runtime, oteryn_game_control;
GRANT SELECT, DELETE ON game_public_character_profile_outbox, game_public_highscore_outbox
    TO oteryn_game_runtime;
GRANT SELECT ON game_public_character_profile_outbox, game_public_highscore_outbox,
    game_public_world_online TO oteryn_game_control;
GRANT SELECT, INSERT, UPDATE ON game_public_world_online TO oteryn_game_runtime;
