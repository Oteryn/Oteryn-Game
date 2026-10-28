-- Platform owns Premium-day earning. Game accepts only an absolute account-wide total and
-- derives Global's five loyalty points per spent Premium day at this boundary.
CREATE FUNCTION game_account_loyalty_project_premium_days(
    p_account_id UUID,
    p_spent_premium_days BIGINT,
    p_platform_revision BIGINT
) RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_points BIGINT;
    v_current game_account_loyalty_projections%ROWTYPE;
BEGIN
    IF p_spent_premium_days < 0 OR p_spent_premium_days > 429496729 THEN
        RAISE EXCEPTION 'spent Premium-day total is outside the loyalty projection range'
            USING ERRCODE = '22003';
    END IF;
    IF p_platform_revision <= 0 THEN
        RAISE EXCEPTION 'Platform revision must be positive' USING ERRCODE = '22003';
    END IF;

    v_points := p_spent_premium_days * 5;
    SELECT * INTO v_current
      FROM game_account_loyalty_projections
     WHERE account_id = p_account_id
     FOR UPDATE;

    IF FOUND THEN
        IF p_platform_revision < v_current.platform_revision THEN
            RAISE EXCEPTION 'stale Platform loyalty revision' USING ERRCODE = '40001';
        END IF;
        IF p_platform_revision = v_current.platform_revision THEN
            IF v_points <> v_current.loyalty_points THEN
                RAISE EXCEPTION 'Platform loyalty revision conflicts with its prior value'
                    USING ERRCODE = '23000';
            END IF;
            RETURN v_current.loyalty_points;
        END IF;
        IF v_points < v_current.loyalty_points THEN
            RAISE EXCEPTION 'spent Premium-day total cannot decrease'
                USING ERRCODE = '23000';
        END IF;

        UPDATE game_account_loyalty_projections
           SET loyalty_points = v_points,
               platform_revision = p_platform_revision,
               projected_at = clock_timestamp()
         WHERE account_id = p_account_id;
    ELSE
        INSERT INTO game_account_loyalty_projections(
            account_id, loyalty_points, platform_revision
        ) VALUES (p_account_id, v_points, p_platform_revision);
    END IF;

    RETURN v_points;
END;
$$;

REVOKE ALL ON FUNCTION game_account_loyalty_project_premium_days(UUID, BIGINT, BIGINT)
    FROM PUBLIC, oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_account_loyalty_project_premium_days(UUID, BIGINT, BIGINT)
    TO oteryn_game_control;
