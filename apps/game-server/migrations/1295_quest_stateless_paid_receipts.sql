-- Lane1290 composes Father acceptance with main0999-1008. Extend the source-reward zero-effect
-- arm while retaining the exact eleven-track Father allowance and every
-- existing array/cardinality bound. Deferred source/fee/MINT proofs remain mandatory.
DO $migration$
DECLARE definition TEXT; marker TEXT:=' OR completes';
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
        WHERE conrelid='game_character_quest_receipts'::regclass
            AND conname='game_character_quest_receipt_effects_shape';
    IF (length(definition)-length(replace(definition,marker,'')))/length(marker)<>1 THEN
        RAISE EXCEPTION 'shared quest receipt shape changed'; END IF;
    EXECUTE 'ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape';
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape '
        || replace(definition,marker,marker || ' OR (transition_key LIKE ''oteryn:quest-reward/%'')');
END $migration$;

-- Lane extension of the shared QENGINE value-only proof: monetary sales.
-- Shared main migrations 0999-1008 remain byte-identical.
CREATE FUNCTION game_quest_stateless_paid_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source game_quest_reward_sources%ROWTYPE;
BEGIN
    IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;
    SELECT * INTO source FROM game_quest_reward_sources
        WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id
            AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    -- Other source-owned value-only shapes retain QENGINE's own proof.
    IF NOT FOUND OR source.plan->'repeatable' IS DISTINCT FROM 'true'::jsonb
        OR jsonb_typeof(source.plan->'fee') IS DISTINCT FROM 'object'
        OR jsonb_typeof(source.plan->'give') IS DISTINCT FROM 'object' THEN RETURN NULL; END IF;
    IF NEW.cause_kind<>'command'
        OR NEW.created_xact_id<>pg_current_xact_id()
        OR source.created_xact_id<>pg_current_xact_id()
        OR source.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR source.plan->>'quest' IS DISTINCT FROM NEW.quest_key
        OR source.plan->'storage' IS DISTINCT FROM '[]'::jsonb
        OR source.plan->'repeatable' IS DISTINCT FROM 'true'::jsonb
        OR jsonb_typeof(source.plan->'fee') IS DISTINCT FROM 'object'
        OR jsonb_typeof(source.plan->'give') IS DISTINCT FROM 'object'
        OR source.mint_transaction_id IS NULL
        OR NOT EXISTS(SELECT 1 FROM game_quest_reward_fees fee
            WHERE fee.character_id=NEW.character_id AND fee.cause_id=NEW.cause_id
                AND fee.cause_ordinal=NEW.cause_ordinal AND fee.transition_key=NEW.transition_key
                AND fee.created_xact_id=pg_current_xact_id()
                AND fee.committed_character_revision=NEW.committed_character_revision
                AND source.plan->'fee'->>'gold'=fee.fee_gold_units::text
                AND source.plan->'fee'->>'include_bank'=fee.include_bank::text)
    THEN RAISE EXCEPTION 'stateless paid quest lacks exact same-transaction source, fee and MINT'
        USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_stateless_paid_complete AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_stateless_paid_complete();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_stateless_paid_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_stateless_paid_complete() FROM PUBLIC;

DO $migration$
DECLARE body TEXT; marker TEXT:='    IF jsonb_typeof(v_source.plan->''give'')=''object''';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure) INTO STRICT body;
    IF (length(body)-length(replace(body,marker,'')))/length(marker)<>1 THEN
        RAISE EXCEPTION 'shared value-only quest proof shape changed'; END IF;
    -- This arm also has the independently deferred exact same-xact fee proof above,
    -- while the unchanged shared source completion proves the native MINT.
    EXECUTE replace(body,marker,
        '    IF v_source.plan->''repeatable''=''true''::jsonb
        AND jsonb_typeof(v_source.plan->''fee'')=''object''
        AND (v_source.plan #>> ''{fee,gold}'')::bigint BETWEEN 1 AND 1000000000000
        AND jsonb_typeof(v_source.plan->''give'')=''object''
        AND v_source.mint_transaction_id IS NOT NULL THEN RETURN NULL; END IF;
' || marker);
END $migration$;
