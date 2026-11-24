-- Read the Berlin day containing the authoritative active reset. The Task
-- Board writer already holds the active row through game_task_board_period.
CREATE FUNCTION game_task_board_reset_day(p_world UUID) RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT ((e.activated_at AT TIME ZONE 'Europe/Berlin') - interval '6 hours')::date
    FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
    WHERE a.world_id=p_world AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_06'
$$;
REVOKE ALL ON FUNCTION game_task_board_reset_day(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_task_board_reset_day(UUID) TO oteryn_game_runtime,oteryn_game_control;

-- Bonus claim provenance travels with the fenced board state and its receipt.
-- Keep the shared token ledger and add the new action to its exact source checks.
DO $migration$
DECLARE definition TEXT;
        previous TEXT := $old$'"ClaimDailyReroll"'::jsonb,'"ClaimBounty"'::jsonb$old$;
        replacement TEXT := $new$'"ClaimDailyReroll"'::jsonb,'"ClaimBounty"'::jsonb,'"ClaimDailyBountyToken"'::jsonb$new$;
        signature REGPROCEDURE;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_task_board_token_source_guard()'::regprocedure,
        'game_task_board_token_receipt_guard()'::regprocedure
    ] LOOP
        SELECT pg_get_functiondef(signature) INTO definition;
        IF strpos(definition,previous)=0 THEN
            RAISE EXCEPTION 'unexpected Task Board token guard shape: %', signature;
        END IF;
        EXECUTE replace(definition,previous,replacement);
    END LOOP;
END $migration$;

-- A successful claim must bind its token and board checkpoint to the same
-- reset day as the command, even if a caller constructs a receipt directly.
CREATE FUNCTION game_task_board_daily_bounty_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind<>'command' OR NEW.committed_character_revision IS NULL
        OR (NEW.effects->>'reroll_tokens_granted')::bigint IS DISTINCT FROM 1
        OR (NEW.reply->'data'->>'daily_bounty_claim_day')::bigint IS DISTINCT FROM
            (game_task_board_reset_day(NEW.world_id)-DATE '1970-01-01')
        OR (NEW.reply->'data'->>'daily_bounty_claim_day') IS NULL THEN
        RAISE EXCEPTION 'daily bounty token requires its committed reset day and token grant' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_task_board_reset_day(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_task_board_daily_bounty_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_task_board_daily_bounty_guard() FROM PUBLIC;

CREATE CONSTRAINT TRIGGER game_task_board_daily_bounty_guard
    AFTER INSERT ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    WHEN (NEW.action='"ClaimDailyBountyToken"'::jsonb AND NEW.refusal='null'::jsonb)
    EXECUTE FUNCTION game_task_board_daily_bounty_guard();

CREATE UNIQUE INDEX game_task_board_daily_bounty_token_once
    ON game_task_board_receipts(character_id,world_id,((reply->'data'->>'daily_bounty_claim_day')::bigint))
    WHERE action='"ClaimDailyBountyToken"'::jsonb AND refusal='null'::jsonb;
