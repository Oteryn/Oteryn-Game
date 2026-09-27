-- OPS-NODE-BOOT-01 follow-up (#832): the control plane admits only the fresh
-- generation-one Character recovery store, through a definer function. It no
-- longer holds a direct INSERT that could write a successor admission not
-- backed by the recovery fence.

-- Admit generation one into an empty Character store. Returns TRUE when it is
-- admitted now, or when the only admission is exactly this generation one (a
-- re-run after a lost acknowledgement). Returns FALSE for any other prior
-- Character state.
CREATE FUNCTION game_character_admit_fresh_recovery(
    p_authority_scope_id TEXT, p_recovery_event_id UUID, p_issued_at NUMERIC, p_issuer_identity TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_existing BIGINT;
BEGIN
    -- Serializes concurrent fresh admissions; the second sees the first.
    LOCK TABLE game_character_recovery_admissions IN SHARE ROW EXCLUSIVE MODE;
    SELECT (SELECT count(*) FROM game_character_recovery_admissions)
         + (SELECT count(*) FROM game_character_account_guards)
         + (SELECT count(*) FROM game_character_roots)
         + (SELECT count(*) FROM game_character_operation_receipts)
         + (SELECT count(*) FROM game_character_audit_outbox)
         + (SELECT count(*) FROM game_character_audit_legal_holds)
         + (SELECT count(*) FROM game_character_bootstrap_intent_floors)
         + (SELECT count(*) FROM game_character_interpretations)
      INTO v_existing;
    IF v_existing <> 0 THEN
        -- A re-run after a lost acknowledgement finds exactly this
        -- generation-one admission and no other admission since.
        RETURN (SELECT count(*) FROM game_character_recovery_admissions) = 1 AND EXISTS (
            SELECT 1 FROM game_character_recovery_admissions
            WHERE recovery_generation = 1 AND predecessor_generation = 0
              AND authority_scope_id = p_authority_scope_id
              AND recovery_event_id = p_recovery_event_id
              AND issued_at = p_issued_at
              AND issuer_identity = p_issuer_identity);
    END IF;
    INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation,
        recovery_event_id, predecessor_generation, predecessor_digest, issued_at, issuer_identity,
        reconciled_at)
    VALUES (p_authority_scope_id, 1, p_recovery_event_id, 0, NULL, p_issued_at, p_issuer_identity,
        floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT);
    RETURN TRUE;
END; $$;

DO $$
BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_character_admit_fresh_recovery(text, uuid, numeric, text) SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON FUNCTION game_character_admit_fresh_recovery(TEXT, UUID, NUMERIC, TEXT) FROM PUBLIC;
REVOKE INSERT ON game_character_recovery_admissions FROM oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_character_admit_fresh_recovery(TEXT, UUID, NUMERIC, TEXT)
    TO oteryn_game_control;
