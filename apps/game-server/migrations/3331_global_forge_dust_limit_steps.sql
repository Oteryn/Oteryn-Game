-- Global Dust Limit purchases advance by 20. Historical one-point ledger links
-- and receipts keep their original arithmetic, revision and bytes.
CREATE OR REPLACE FUNCTION game_forge_dust_limit_cost(p_limit INTEGER) RETURNS INTEGER
LANGUAGE sql IMMUTABLE STRICT SET search_path FROM CURRENT AS $$
    SELECT CASE WHEN p_limit BETWEEN 100 AND 4580 AND (p_limit - 100) % 20 = 0
        THEN 25 + (p_limit - 100) / 20 END;
$$;

CREATE FUNCTION game_forge_dust_raise_valid(p_before INTEGER, p_after INTEGER, p_amount BIGINT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE STRICT SET search_path FROM CURRENT AS $$
    SELECT (p_before BETWEEN 100 AND 324 AND p_after = p_before + 1 AND p_amount = p_before - 75)
        OR (p_after = p_before + 20 AND p_amount IS NOT DISTINCT FROM game_forge_dust_limit_cost(p_before));
$$;

REVOKE ALL ON FUNCTION game_forge_dust_limit_cost(INTEGER),
    game_forge_dust_raise_valid(INTEGER, INTEGER, BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_forge_dust_limit_cost(INTEGER),
    game_forge_dust_raise_valid(INTEGER, INTEGER, BIGINT)
    TO oteryn_game_runtime, oteryn_game_control;

ALTER TABLE game_character_forge_dust_entries
    DROP CONSTRAINT game_character_forge_dust_entry_arithmetic,
    ADD CONSTRAINT game_character_forge_dust_entry_arithmetic CHECK (CASE kind
        WHEN 'GAIN' THEN dust_limit_after = dust_limit_before
            AND balance_after = balance_before + amount - lost_amount
            AND (lost_amount = 0 OR balance_after = dust_limit_after)
        WHEN 'SPEND' THEN dust_limit_after = dust_limit_before AND lost_amount = 0
            AND balance_after = balance_before - amount
        WHEN 'CONVERT' THEN cause = 'forge' AND amount = 60 AND lost_amount = 0
            AND dust_limit_after = dust_limit_before AND balance_after = balance_before - amount
        WHEN 'LIMIT_RAISE' THEN game_forge_dust_raise_valid(dust_limit_before, dust_limit_after, amount)
            AND lost_amount = 0 AND balance_after = balance_before - amount
        ELSE FALSE END) NOT VALID;

ALTER TABLE game_forge_dust_limit_receipts
    DROP CONSTRAINT game_forge_dust_limit_receipt_closed,
    ADD CONSTRAINT game_forge_dust_limit_receipt_closed CHECK (
        (outcome = 1 AND previous_entry_id IS NOT NULL AND game_character_is_uuid_v7(previous_entry_id)
            AND amount IS NOT NULL AND balance_before IS NOT NULL AND balance_after IS NOT NULL
            AND dust_limit_before IS NOT NULL AND dust_limit_after IS NOT NULL
            AND envelope_sha256 IS NOT NULL AND octet_length(envelope_sha256) = 32
            AND game_forge_dust_raise_valid(dust_limit_before, dust_limit_after, amount)
            AND balance_before BETWEEN amount AND dust_limit_before AND balance_after = balance_before - amount)
        OR (outcome = 2 AND previous_entry_id IS NULL AND amount IS NULL
            AND balance_before IS NULL AND balance_after IS NULL
            AND dust_limit_before IS NULL AND dust_limit_after IS NULL AND envelope_sha256 IS NULL)
    ) NOT VALID;

ALTER TABLE game_forge_dust_limit_reservations
    DROP CONSTRAINT game_forge_dust_limit_reservations_ruleset_revision_check,
    ADD CONSTRAINT game_forge_dust_limit_reservations_ruleset_revision_check
        CHECK (ruleset_revision IN ('D1230', 'global-forge-v2', 'global-forge-dust-v3'));

ALTER TABLE game_forge_reservations
    DROP CONSTRAINT game_forge_reservations_ruleset_revision_check,
    ADD CONSTRAINT game_forge_reservations_ruleset_revision_check
        CHECK (ruleset_revision = 'crystal-201189f-forge-v1'
            OR (ruleset_revision = 'global-forge-v2' AND get_byte(intent, 0) IN (3, 7))
            OR (ruleset_revision = 'global-forge-dust-v3' AND get_byte(intent, 0) = 7));

-- Keep the installed proof bodies, trigger bindings, and privileges. A pending
-- old reservation may only terminate REVISION_CHANGED. New paid receipts must
-- prove the new revision and exact paid stage even though the CHECK accepts history.
DO $migration$
DECLARE
    proof TEXT := pg_get_functiondef('game_forge_dust_limit_proof_guard()'::regprocedure);
    source TEXT := pg_get_functiondef('game_forge_dust_source_proven()'::regprocedure);
BEGIN
    IF strpos(proof, '100+v_reserved.expected_upgrade_count') = 0
       OR strpos(proof, '''global-forge-v2''') = 0
       OR strpos(proof, 'OR v_reserved.simulation_profile_revision<>1') = 0
       OR strpos(source, '(NEW.dust_limit_before-100)') = 0 THEN
        RAISE EXCEPTION '3331 requires the installed 3323 Forge Dust Limit proofs';
    END IF;
    proof := replace(proof, '''global-forge-v2''', '''global-forge-dust-v3''');
    proof := replace(proof, '100+v_reserved.expected_upgrade_count',
                     '100+20*v_reserved.expected_upgrade_count');
    proof := replace(proof, 'OR v_reserved.simulation_profile_revision<>1',
                     'OR v_reserved.simulation_profile_revision<>1
       OR v_receipt.dust_limit_after<>v_receipt.dust_limit_before+20');
    EXECUTE proof;
    EXECUTE replace(source, '(NEW.dust_limit_before-100)',
        '(CASE WHEN NEW.dust_limit_after=NEW.dust_limit_before+1
               THEN NEW.dust_limit_before-100 ELSE (NEW.dust_limit_before-100)/20 END)');
END;
$migration$;
