-- N009 (PLATFORM_SANCTION_LOYALTY_EXPORT_V1): Platform-owned sanction decisions projected into
-- Game for the player's own read (capability 497). Platform remains the only sanction authority:
-- Game enforces nothing from these rows and never writes them outside this control-only
-- projection, which follows the 0406/0925 loyalty projection pattern.
CREATE TABLE game_account_sanction_projections (
    sanction_stream_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(sanction_stream_id)),
    decision_revision BIGINT NOT NULL CHECK (decision_revision > 0),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    character_id UUID,
    -- 1 mute, 2 jail, 3 character_ban, 4 account_ban, 5 namelock, 6 notation.
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 6),
    reason_code INTEGER NOT NULL CHECK (reason_code BETWEEN 1 AND 65535),
    reason_text TEXT NOT NULL
        CHECK (octet_length(reason_text) BETWEEN 1 AND 120 AND reason_text !~ '[[:cntrl:]]'),
    starts_at_ms BIGINT NOT NULL CHECK (starts_at_ms > 0),
    ends_at_ms BIGINT CHECK (ends_at_ms IS NULL OR ends_at_ms > starts_at_ms),
    revoked BOOLEAN NOT NULL,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CHECK (CASE kind WHEN 4 THEN character_id IS NULL WHEN 6 THEN true
                     ELSE character_id IS NOT NULL END),
    CHECK (CASE WHEN kind IN (1, 2) THEN ends_at_ms IS NOT NULL
                WHEN kind IN (5, 6) THEN ends_at_ms IS NULL
                ELSE true END)
);
CREATE INDEX game_account_sanction_projections_account
    ON game_account_sanction_projections(account_id, starts_at_ms DESC, sanction_stream_id DESC);

REVOKE ALL ON game_account_sanction_projections FROM PUBLIC;
GRANT SELECT ON game_account_sanction_projections TO oteryn_game_runtime, oteryn_game_control;

-- Idempotent by decision revision: 1 applied (new stream or higher revision), 2 identical replay,
-- 3 stale (lower revision, stored row kept). An equal revision with other content, or a higher
-- revision that moves the stream to another account, Character or kind, is a conflict (23000)
-- and changes nothing. A Character must belong to the named account (23503).
CREATE FUNCTION game_account_sanction_project(
    p_sanction_stream_id UUID,
    p_decision_revision BIGINT,
    p_account_id UUID,
    p_character_id UUID,
    p_kind SMALLINT,
    p_reason_code INTEGER,
    p_reason_text TEXT,
    p_starts_at_ms BIGINT,
    p_ends_at_ms BIGINT,
    p_revoked BOOLEAN
) RETURNS SMALLINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_current game_account_sanction_projections%ROWTYPE;
BEGIN
    IF p_decision_revision IS NULL OR p_decision_revision <= 0 THEN
        RAISE EXCEPTION 'Platform decision revision must be positive' USING ERRCODE = '22003';
    END IF;
    IF p_character_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_character_roots
        WHERE character_id = p_character_id AND account_id = p_account_id
    ) THEN
        RAISE EXCEPTION 'sanctioned Character does not belong to the sanctioned account'
            USING ERRCODE = '23503';
    END IF;

    SELECT * INTO v_current
      FROM game_account_sanction_projections
     WHERE sanction_stream_id = p_sanction_stream_id
     FOR UPDATE;

    IF NOT FOUND THEN
        INSERT INTO game_account_sanction_projections(sanction_stream_id, decision_revision,
            account_id, character_id, kind, reason_code, reason_text, starts_at_ms, ends_at_ms,
            revoked)
        VALUES (p_sanction_stream_id, p_decision_revision, p_account_id, p_character_id, p_kind,
            p_reason_code, p_reason_text, p_starts_at_ms, p_ends_at_ms, p_revoked)
        ON CONFLICT (sanction_stream_id) DO NOTHING;
        IF FOUND THEN
            RETURN 1;
        END IF;
        -- A concurrent projection inserted the stream first: decide against its row.
        SELECT * INTO STRICT v_current
          FROM game_account_sanction_projections
         WHERE sanction_stream_id = p_sanction_stream_id
         FOR UPDATE;
    END IF;

    IF p_decision_revision < v_current.decision_revision THEN
        RETURN 3;
    END IF;
    IF v_current.account_id <> p_account_id
       OR v_current.character_id IS DISTINCT FROM p_character_id
       OR v_current.kind <> p_kind THEN
        RAISE EXCEPTION 'Platform sanction stream changed its subject or kind'
            USING ERRCODE = '23000';
    END IF;
    IF p_decision_revision = v_current.decision_revision THEN
        IF v_current.reason_code <> p_reason_code
           OR v_current.reason_text <> p_reason_text
           OR v_current.starts_at_ms <> p_starts_at_ms
           OR v_current.ends_at_ms IS DISTINCT FROM p_ends_at_ms
           OR v_current.revoked <> p_revoked THEN
            RAISE EXCEPTION 'Platform sanction revision conflicts with its prior value'
                USING ERRCODE = '23000';
        END IF;
        RETURN 2;
    END IF;
    UPDATE game_account_sanction_projections
       SET decision_revision = p_decision_revision,
           reason_code = p_reason_code,
           reason_text = p_reason_text,
           starts_at_ms = p_starts_at_ms,
           ends_at_ms = p_ends_at_ms,
           revoked = p_revoked,
           projected_at = clock_timestamp()
     WHERE sanction_stream_id = p_sanction_stream_id;
    RETURN 1;
END;
$$;

REVOKE ALL ON FUNCTION game_account_sanction_project(UUID, BIGINT, UUID, UUID, SMALLINT, INTEGER,
    TEXT, BIGINT, BIGINT, BOOLEAN) FROM PUBLIC, oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_account_sanction_project(UUID, BIGINT, UUID, UUID, SMALLINT,
    INTEGER, TEXT, BIGINT, BIGINT, BOOLEAN) TO oteryn_game_control;
