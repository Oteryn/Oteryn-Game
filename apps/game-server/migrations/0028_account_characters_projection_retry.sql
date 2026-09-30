-- LCFA-1b (#162; carried findings of the LCFA-1 review of #1330): three corrections to the
-- `ListCharactersForAccount` producer state of migration 0024. 0024 stays unchanged.
--
--   * `revised_at` (Unix ms, the writing transaction's start) records when each account's
--     current `projection_revision` was assigned. The publisher derives `source_observed_at`
--     from it, so every retry of the same (epoch, revision) publication is byte-identical, also
--     across a publisher restart, and a lost acknowledgement never turns into a 409. Rows that
--     exist when this migration runs get the migration time.
--   * The resync queues every account at a new revision, through the same touch as a Character
--     write. 0024 re-queued the current revision with ON CONFLICT DO NOTHING, so an
--     acknowledgement of a snapshot read before the resync could clear the row the resync meant
--     to queue. A new revision is never covered by such an acknowledgement. The touch locks each
--     account's revision row, so a Character write of an account waits for a running resync.
--   * An epoch raise uses Unix milliseconds: max(current + 1, now in Unix ms). In Unix seconds,
--     two raises within one second put the epoch a second ahead of the clock, and a restore
--     followed by a raise within that second reissued an epoch that was already published.
--     Now the epoch runs ahead of the clock only when raises come faster than one per
--     millisecond, and only by one millisecond per such raise; a restore and its raise do not
--     complete within that lead. Every value is above the second-based epochs of 0024, so the
--     epoch still only rises. U-LC2 assumption unchanged: the operator clock does not run
--     backwards across a restore.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that restores the
-- 0024 bodies of both functions and drops `revised_at`. The projection is derived state.

ALTER TABLE game_character_account_projections
    ADD COLUMN revised_at BIGINT NOT NULL
        DEFAULT floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT
        CHECK (revised_at >= 0);
ALTER TABLE game_character_account_projections ALTER COLUMN revised_at DROP DEFAULT;

CREATE OR REPLACE FUNCTION game_character_account_projection_touch(p_account UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_revision BIGINT;
BEGIN
    INSERT INTO game_character_account_projections(account_id, projection_revision, revised_at)
    VALUES (p_account, 1, v_now)
    ON CONFLICT (account_id) DO UPDATE
        SET projection_revision = game_character_account_projections.projection_revision + 1,
            revised_at = v_now
    RETURNING projection_revision INTO v_revision;
    INSERT INTO game_character_account_projection_outbox(account_id, projection_epoch,
        projection_revision, created_at)
    SELECT p_account, projection_epoch, v_revision, v_now
    FROM game_character_account_projection_epoch WHERE epoch_scope = 1;
END; $$;

-- Operator-only resync (§5 Resync). With `p_raise_epoch` it first raises the epoch to
-- max(current + 1, now in Unix ms). It then touches every account in AccountId order, so each
-- is queued at a new revision of the current epoch. The queued rows carry the resync start, so
-- the §5.1 watermark stays below it until every account's snapshot is acknowledged. Returns
-- the current epoch.
CREATE OR REPLACE FUNCTION game_character_account_projection_resync(p_raise_epoch BOOLEAN)
RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_now BIGINT := floor(extract(epoch FROM transaction_timestamp()) * 1000)::BIGINT;
    v_epoch BIGINT;
    v_account UUID;
BEGIN
    IF p_raise_epoch IS NULL THEN
        RAISE EXCEPTION 'resync requires an explicit epoch choice' USING ERRCODE = '22004';
    END IF;
    SELECT projection_epoch INTO v_epoch FROM game_character_account_projection_epoch
        WHERE epoch_scope = 1 FOR UPDATE;
    IF p_raise_epoch THEN
        v_epoch := greatest(v_epoch + 1, v_now);
        UPDATE game_character_account_projection_epoch
           SET projection_epoch = v_epoch, raised_at = v_now WHERE epoch_scope = 1;
    END IF;
    FOR v_account IN
        SELECT account_id FROM game_character_account_projections ORDER BY account_id
    LOOP
        PERFORM game_character_account_projection_touch(v_account);
    END LOOP;
    RETURN v_epoch;
END; $$;

-- CREATE OR REPLACE drops the per-function settings; ownership and grants are kept.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_account_projection_touch(uuid)',
        'game_character_account_projection_resync(boolean)'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;
