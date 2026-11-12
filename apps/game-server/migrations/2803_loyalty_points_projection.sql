-- N014 (PLATFORM_SANCTION_LOYALTY_EXPORT_V1, owner decision 2026-11-11): the Game puller
-- projects Platform's loyalty-snapshot stream. Platform states the loyalty points itself; Game
-- ranks by them and never derives or awards loyalty. Idempotent by Platform revision:
-- 1 applied (new account row or higher revision), 2 identical replay, 3 stale (lower revision,
-- stored row kept). An equal revision with other points is a conflict (23000).
CREATE FUNCTION game_account_loyalty_project_points(
    p_account_id UUID,
    p_spent_premium_days BIGINT,
    p_loyalty_points BIGINT,
    p_platform_revision BIGINT
) RETURNS SMALLINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_current game_account_loyalty_projections%ROWTYPE;
BEGIN
    IF p_spent_premium_days IS NULL OR p_spent_premium_days < 0
       OR p_spent_premium_days > 2147483647
       OR p_loyalty_points IS NULL OR p_loyalty_points < 0 OR p_loyalty_points > 2147483647 THEN
        RAISE EXCEPTION 'loyalty snapshot is outside the projection range' USING ERRCODE = '22003';
    END IF;
    IF p_platform_revision IS NULL OR p_platform_revision <= 0 THEN
        RAISE EXCEPTION 'Platform revision must be positive' USING ERRCODE = '22003';
    END IF;

    SELECT * INTO v_current
      FROM game_account_loyalty_projections
     WHERE account_id = p_account_id
     FOR UPDATE;

    IF NOT FOUND THEN
        INSERT INTO game_account_loyalty_projections(account_id, loyalty_points, platform_revision)
        VALUES (p_account_id, p_loyalty_points, p_platform_revision)
        ON CONFLICT (account_id) DO NOTHING;
        IF FOUND THEN
            RETURN 1;
        END IF;
        -- A concurrent projection inserted the account first: decide against its row.
        SELECT * INTO STRICT v_current
          FROM game_account_loyalty_projections
         WHERE account_id = p_account_id
         FOR UPDATE;
    END IF;

    IF p_platform_revision < v_current.platform_revision THEN
        RETURN 3;
    END IF;
    IF p_platform_revision = v_current.platform_revision THEN
        IF p_loyalty_points <> v_current.loyalty_points THEN
            RAISE EXCEPTION 'Platform loyalty revision conflicts with its prior value'
                USING ERRCODE = '23000';
        END IF;
        RETURN 2;
    END IF;
    UPDATE game_account_loyalty_projections
       SET loyalty_points = p_loyalty_points,
           platform_revision = p_platform_revision,
           projected_at = clock_timestamp()
     WHERE account_id = p_account_id;
    RETURN 1;
END;
$$;

REVOKE ALL ON FUNCTION game_account_loyalty_project_points(UUID, BIGINT, BIGINT, BIGINT)
    FROM PUBLIC, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_account_loyalty_project_points(UUID, BIGINT, BIGINT, BIGINT)
    TO oteryn_game_runtime;

-- Whether a streamed record names an account Game guards and, when it names one, a Character of
-- that account. The puller skips other records: an account without a Game Character has nothing
-- to show, and a skipped record never blocks the stream.
CREATE FUNCTION game_platform_stream_subject_known(p_account_id UUID, p_character_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT EXISTS (SELECT 1 FROM game_character_account_guards WHERE account_id = p_account_id)
       AND (p_character_id IS NULL OR EXISTS (
            SELECT 1 FROM game_character_roots
             WHERE character_id = p_character_id AND account_id = p_account_id));
$$;

REVOKE ALL ON FUNCTION game_platform_stream_subject_known(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_platform_stream_subject_known(UUID, UUID) TO oteryn_game_runtime;
