-- LCFA-1 (D166, Q28a): the Game producer state of `ListCharactersForAccount`
-- (`docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §5).
--
--   * `game_character_account_projections`: the per-AccountId `projection_revision`.
--   * `game_character_account_projection_outbox`: one row per (account, epoch, revision)
--     change awaiting an acknowledged snapshot. `created_at` is the writing transaction's start
--     (Unix ms), never later than its commit, so the §5.1 watermark stays conservative.
--   * `game_character_account_projection_epoch`: the single `projection_epoch` (U-LC2).
--
-- A definer trigger on `game_character_roots` advances the revision and writes the outbox row
-- in the Character mutation's own transaction, whatever the writer: a root insert (bootstrap,
-- CHAR-NAME-1 name), a change of a CharacterSummary field or of the owner (both accounts), and a
-- root deletion. Roots are immutable in this slice, so today only the insert occurs.
--
-- No runtime role writes these tables directly. The publisher reads them and deletes
-- acknowledged outbox rows; the epoch raise and the full resync are one operator function.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- trigger, the three tables and both functions. The projection is derived state: dropping it
-- loses no authority, and Platform's read model goes stale through the watermark.

CREATE TABLE game_character_account_projection_epoch (
    epoch_scope SMALLINT PRIMARY KEY CHECK (epoch_scope = 1),
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    raised_at BIGINT NOT NULL CHECK (raised_at >= 0)
);
INSERT INTO game_character_account_projection_epoch VALUES (1, 1, 0);

CREATE TABLE game_character_account_projections (
    account_id UUID PRIMARY KEY REFERENCES game_character_account_guards(account_id),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0)
);

CREATE TABLE game_character_account_projection_outbox (
    account_id UUID NOT NULL REFERENCES game_character_account_projections(account_id),
    projection_epoch BIGINT NOT NULL CHECK (projection_epoch > 0),
    projection_revision BIGINT NOT NULL CHECK (projection_revision > 0),
    created_at BIGINT NOT NULL CHECK (created_at >= 0),
    PRIMARY KEY (account_id, projection_epoch, projection_revision)
);
CREATE INDEX game_character_account_projection_outbox_age
    ON game_character_account_projection_outbox (created_at, account_id);

-- Characters committed before this migration (preproduction stores) are queued once at
-- revision 1, so the initial fill needs no operator step and the watermark holds until then.
INSERT INTO game_character_account_projections(account_id, projection_revision)
SELECT DISTINCT account_id, 1 FROM game_character_roots;
INSERT INTO game_character_account_projection_outbox
SELECT account_id, 1, 1, floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT
FROM game_character_account_projections;

CREATE FUNCTION game_character_account_projection_touch(p_account UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_revision BIGINT;
BEGIN
    INSERT INTO game_character_account_projections(account_id, projection_revision)
    VALUES (p_account, 1)
    ON CONFLICT (account_id) DO UPDATE
        SET projection_revision = game_character_account_projections.projection_revision + 1
    RETURNING projection_revision INTO v_revision;
    INSERT INTO game_character_account_projection_outbox(account_id, projection_epoch,
        projection_revision, created_at)
    SELECT p_account, projection_epoch, v_revision,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT
    FROM game_character_account_projection_epoch WHERE epoch_scope = 1;
END; $$;

-- SECURITY DEFINER: the writer role holds no privilege on the projection tables, so the only
-- way a revision advances is a Character root mutation in the same transaction.
CREATE FUNCTION game_character_account_projection_trigger() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        PERFORM game_character_account_projection_touch(NEW.account_id);
    END IF;
    IF TG_OP = 'DELETE' OR (TG_OP = 'UPDATE' AND NEW.account_id <> OLD.account_id) THEN
        PERFORM game_character_account_projection_touch(OLD.account_id);
    END IF;
    RETURN NULL;
END; $$;
CREATE TRIGGER game_character_account_projection_insert_delete AFTER INSERT OR DELETE
    ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_account_projection_trigger();
CREATE TRIGGER game_character_account_projection_update AFTER UPDATE ON game_character_roots
    FOR EACH ROW WHEN ((NEW.account_id, NEW.world_id, NEW.lifecycle, NEW.name)
                       IS DISTINCT FROM (OLD.account_id, OLD.world_id, OLD.lifecycle, OLD.name))
    EXECUTE FUNCTION game_character_account_projection_trigger();

-- Operator-only resync (§5 Resync): queue the current snapshot of every account. With
-- `p_raise_epoch` it first raises the epoch to max(current + 1, now in Unix seconds), so an
-- epoch raised after a restore exceeds any epoch the restored store had published before
-- (U-LC2, stated assumption: the operator clock does not run backwards across a restore).
-- The queued rows carry the resync start, so the §5.1 watermark stays below it until every
-- account's snapshot is acknowledged. Returns the current epoch.
CREATE FUNCTION game_character_account_projection_resync(p_raise_epoch BOOLEAN) RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_epoch BIGINT;
BEGIN
    IF p_raise_epoch IS NULL THEN
        RAISE EXCEPTION 'resync requires an explicit epoch choice' USING ERRCODE = '22004';
    END IF;
    SELECT projection_epoch INTO v_epoch FROM game_character_account_projection_epoch
        WHERE epoch_scope = 1 FOR UPDATE;
    IF p_raise_epoch THEN
        v_epoch := greatest(v_epoch + 1, v_now / 1000);
        UPDATE game_character_account_projection_epoch
           SET projection_epoch = v_epoch, raised_at = v_now WHERE epoch_scope = 1;
    END IF;
    INSERT INTO game_character_account_projection_outbox(account_id, projection_epoch,
        projection_revision, created_at)
    SELECT account_id, v_epoch, projection_revision, v_now
    FROM game_character_account_projections
    ON CONFLICT DO NOTHING;
    RETURN v_epoch;
END; $$;

-- The epoch only rises and a revision only advances by one; no projection table truncates.
CREATE FUNCTION game_character_account_projection_epoch_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.epoch_scope = OLD.epoch_scope
       AND NEW.projection_epoch > OLD.projection_epoch THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'projection epoch only rises' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_account_projection_epoch_guard BEFORE UPDATE OR DELETE
    ON game_character_account_projection_epoch
    FOR EACH ROW EXECUTE FUNCTION game_character_account_projection_epoch_guard();
CREATE FUNCTION game_character_account_projection_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'UPDATE' AND NEW.account_id = OLD.account_id
       AND NEW.projection_revision = OLD.projection_revision + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'projection revision only advances by one' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_account_projection_revision_guard BEFORE UPDATE OR DELETE
    ON game_character_account_projections
    FOR EACH ROW EXECUTE FUNCTION game_character_account_projection_revision_guard();
CREATE TRIGGER game_character_account_projection_epoch_no_truncate BEFORE TRUNCATE
    ON game_character_account_projection_epoch EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_account_projections_no_truncate BEFORE TRUNCATE
    ON game_character_account_projections EXECUTE FUNCTION game_character_reject_truncate();
-- Rows leave the outbox only by an acknowledgement; a truncation would let the watermark pass
-- undelivered changes.
CREATE TRIGGER game_character_account_projection_outbox_no_truncate BEFORE TRUNCATE
    ON game_character_account_projection_outbox EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_account_projection_touch(uuid)',
        'game_character_account_projection_trigger()',
        'game_character_account_projection_resync(boolean)',
        'game_character_account_projection_epoch_guard()',
        'game_character_account_projection_revision_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_account_projection_epoch, game_character_account_projections,
    game_character_account_projection_outbox FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_account_projection_touch(uuid),
    game_character_account_projection_trigger(),
    game_character_account_projection_resync(boolean),
    game_character_account_projection_epoch_guard(),
    game_character_account_projection_revision_guard()
FROM PUBLIC;
-- Runtime publisher: read the projection state and clear acknowledged outbox rows. No INSERT or
-- UPDATE anywhere, and the CHECK constraints above call no function (PRIV-GUARD-1).
GRANT SELECT ON game_character_account_projection_epoch, game_character_account_projections
    TO oteryn_game_runtime, oteryn_game_control;
GRANT SELECT, DELETE ON game_character_account_projection_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_character_account_projection_outbox TO oteryn_game_control;
