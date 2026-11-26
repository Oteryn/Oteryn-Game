-- TibiaWiki Equipment Upgrade, revision 1205481.
-- Keep historical limits and immutable receipts. New raises use the Global rule.
CREATE OR REPLACE FUNCTION game_forge_dust_limit_valid(p_limit INTEGER) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT SET search_path FROM CURRENT AS $$
    SELECT p_limit BETWEEN 100 AND 325
        OR (p_limit BETWEEN 326 AND 4600 AND (p_limit - 100) % 20 = 0);
$$;

CREATE OR REPLACE FUNCTION game_forge_dust_limit_cost(p_limit INTEGER) RETURNS INTEGER
LANGUAGE sql IMMUTABLE STRICT SET search_path FROM CURRENT AS $$
    SELECT CASE WHEN p_limit BETWEEN 100 AND 324 THEN p_limit - 75 END;
$$;

REVOKE ALL ON FUNCTION game_forge_dust_limit_valid(INTEGER), game_forge_dust_limit_cost(INTEGER)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_forge_dust_limit_valid(INTEGER), game_forge_dust_limit_cost(INTEGER)
    TO oteryn_game_runtime, oteryn_game_control;

ALTER TABLE game_character_forge_dust
    DROP CONSTRAINT IF EXISTS game_character_forge_dust_limit_range,
    ADD CONSTRAINT game_character_forge_dust_limit_range
        CHECK (game_forge_dust_limit_valid(dust_limit));

ALTER TABLE game_character_forge_dust_entries
    DROP CONSTRAINT IF EXISTS game_character_forge_dust_entries_dust_limit_before_check,
    DROP CONSTRAINT IF EXISTS game_character_forge_dust_entries_dust_limit_after_check,
    DROP CONSTRAINT IF EXISTS game_character_forge_dust_entry_arithmetic,
    ADD CONSTRAINT game_character_forge_dust_entries_dust_limit_before_check
        CHECK (game_forge_dust_limit_valid(dust_limit_before)),
    ADD CONSTRAINT game_character_forge_dust_entries_dust_limit_after_check
        CHECK (game_forge_dust_limit_valid(dust_limit_after)),
    ADD CONSTRAINT game_character_forge_dust_entry_arithmetic CHECK (CASE kind
        WHEN 'GAIN' THEN dust_limit_after = dust_limit_before
            AND balance_after = balance_before + amount - lost_amount
            AND (lost_amount = 0 OR balance_after = dust_limit_after)
        WHEN 'SPEND' THEN dust_limit_after = dust_limit_before AND lost_amount = 0
            AND balance_after = balance_before - amount
        WHEN 'CONVERT' THEN cause = 'forge' AND amount = 60 AND lost_amount = 0
            AND dust_limit_after = dust_limit_before AND balance_after = balance_before - amount
        WHEN 'LIMIT_RAISE' THEN dust_limit_after = dust_limit_before + 1
            AND amount IS NOT DISTINCT FROM game_forge_dust_limit_cost(dust_limit_before)
            AND lost_amount = 0 AND balance_after = balance_before - amount
        ELSE FALSE END) NOT VALID;

ALTER TABLE game_forge_dust_limit_receipts
    DROP CONSTRAINT IF EXISTS game_forge_dust_limit_receipt_closed,
    ADD CONSTRAINT game_forge_dust_limit_receipt_closed CHECK (
        (outcome = 1 AND previous_entry_id IS NOT NULL AND game_character_is_uuid_v7(previous_entry_id)
            AND amount IS NOT NULL AND balance_before IS NOT NULL AND balance_after IS NOT NULL
            AND dust_limit_before IS NOT NULL AND dust_limit_after IS NOT NULL
            AND envelope_sha256 IS NOT NULL AND octet_length(envelope_sha256) = 32
            AND dust_limit_after = dust_limit_before + 1
            AND amount IS NOT DISTINCT FROM game_forge_dust_limit_cost(dust_limit_before)
            AND balance_before BETWEEN amount AND dust_limit_before AND balance_after = balance_before - amount)
        OR (outcome = 2 AND previous_entry_id IS NULL AND amount IS NULL
            AND balance_before IS NULL AND balance_after IS NULL
            AND dust_limit_before IS NULL AND dust_limit_after IS NULL AND envelope_sha256 IS NULL)
    ) NOT VALID;

-- Preserve the original reservation revision for replay. A stale pending
-- occurrence terminates with REVISION_CHANGED instead of reinterpreting its stage.
ALTER TABLE game_forge_dust_limit_reservations
    DROP CONSTRAINT IF EXISTS game_forge_dust_limit_reservations_ruleset_revision_check,
    ADD CONSTRAINT game_forge_dust_limit_reservations_ruleset_revision_check
        CHECK (ruleset_revision IN ('D1230', 'global-forge-v2'));

ALTER TABLE game_forge_reservations
    DROP CONSTRAINT IF EXISTS game_forge_reservations_ruleset_revision_check,
    ADD CONSTRAINT game_forge_reservations_ruleset_revision_check
        CHECK (ruleset_revision = 'crystal-201189f-forge-v1'
            OR (ruleset_revision = 'global-forge-v2' AND get_byte(intent, 0) IN (3, 7)));

-- Patch the installed guard bodies in place, retaining triggers and privileges.
-- Each fragment must be present in either its original or its updated form.
DO $migration$
DECLARE
    patch RECORD;
    definition TEXT;
BEGIN
    FOR patch IN SELECT * FROM (VALUES
        ('game_forge_dust_limit_proof_guard()', '''D1230''', '''global-forge-v2'''),
        ('game_forge_dust_limit_proof_guard()',
            '100+20*v_reserved.expected_upgrade_count', '100+v_reserved.expected_upgrade_count'),
        ('game_forge_dust_source_proven()',
            '(NEW.dust_limit_before-100)/20', '(NEW.dust_limit_before-100)'),
        ('game_forge_operation_admit()',
            'NEW.dust_limit NOT BETWEEN 100 AND 4600 OR (NEW.dust_limit-100)%20<>0',
            'NOT game_forge_dust_limit_valid(NEW.dust_limit)'),
        ('game_forge_transition_proven()',
            'r.ruleset_revision<>''crystal-201189f-forge-v1''',
            'r.ruleset_revision IS DISTINCT FROM (CASE WHEN p.operation=3 THEN ''global-forge-v2'' ELSE ''crystal-201189f-forge-v1'' END)'),
        ('game_forge_transition_proven()',
            'WHEN 4 THEN price.convergence_transfer ELSE price.gold END',
            'WHEN 4 THEN price.convergence_transfer WHEN 3 THEN (SELECT admitted.gold FROM game_forge_admitted_price(pa.classification,source) AS admitted) ELSE price.gold END'),
        ('game_forge_transition_proven()', 'p.gold_spent<>gold', 'p.gold_spent IS DISTINCT FROM gold')
    ) AS updates(signature, old_text, new_text)
    LOOP
        SELECT pg_get_functiondef(patch.signature::regprocedure) INTO STRICT definition;
        IF strpos(definition, patch.old_text) > 0 THEN
            EXECUTE replace(definition, patch.old_text, patch.new_text);
        ELSIF strpos(definition, patch.new_text) = 0 THEN
            RAISE EXCEPTION 'Forge guard % is missing its expected definition', patch.signature;
        END IF;
    END LOOP;
END;
$migration$;
