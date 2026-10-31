-- P04: reuse DAILY's bounty-token balance and ledger, adding the bounded Task
-- Board source. Existing daily grants retain their original proof requirements.
CREATE TABLE game_task_board_token_sources (
    claim_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(claim_id)),
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal>0),
    amount SMALLINT NOT NULL CHECK(amount BETWEEN -1 AND 10 AND amount<>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(character_id,cause_id,cause_ordinal)
        REFERENCES game_task_board_receipts(character_id,cause_id,cause_ordinal)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_task_board_token_source_command ON game_task_board_token_sources(character_id,cause_id,cause_ordinal);

DO $$
DECLARE constraint_name TEXT;
BEGIN
    FOR constraint_name IN SELECT conname FROM pg_constraint
        WHERE conrelid='game_character_bounty_reroll_token_entries'::regclass
          AND confrelid='game_character_daily_reward_claims'::regclass LOOP
        EXECUTE format('ALTER TABLE game_character_bounty_reroll_token_entries DROP CONSTRAINT %I',constraint_name);
    END LOOP;
END $$;
ALTER TABLE game_character_bounty_reroll_token_entries
    DROP CONSTRAINT game_character_bounty_reroll_token_entries_amount_check;
ALTER TABLE game_character_bounty_reroll_token_entries
    ADD CONSTRAINT game_character_bounty_reroll_token_entries_amount_check
    CHECK(amount BETWEEN -1 AND 10 AND amount<>0);

CREATE FUNCTION game_task_board_token_source_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
        SELECT 1 FROM game_task_board_receipts r
        JOIN game_character_bounty_reroll_token_entries e ON e.claim_id=NEW.claim_id
        WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id
          AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
          AND r.cause_kind='command' AND r.refusal='null'::jsonb
          AND r.committed_character_revision IS NOT NULL
          AND r.created_xact_id=pg_current_xact_id()
          AND (r.effects->>'reroll_tokens_granted')::int-(r.effects->>'reroll_tokens_spent')::int=NEW.amount
          AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND e.amount=NEW.amount
          AND e.balance_after=(r.reply->>'reroll_tokens')::bigint
          AND ((NEW.amount=-1 AND r.action='"RerollBounty"'::jsonb)
            OR (NEW.amount=1 AND r.action IN('"ClaimDailyReroll"'::jsonb,'"ClaimBounty"'::jsonb)))) THEN
        RAISE EXCEPTION 'bounty token change requires atomic Task Board command' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_task_board_token_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE delta BIGINT;
BEGIN
    delta:=coalesce((NEW.effects->>'reroll_tokens_granted')::bigint,0)
        -coalesce((NEW.effects->>'reroll_tokens_spent')::bigint,0);
    IF NEW.refusal='null'::jsonb AND NEW.action IN('"ClaimDailyReroll"'::jsonb,'"ClaimBounty"'::jsonb)
        AND delta<>1 THEN
        RAISE EXCEPTION 'Task Board successful claim must grant one reroll token' USING ERRCODE='23514';
    END IF;
    IF delta<>0 AND (NEW.refusal<>'null'::jsonb OR NEW.committed_character_revision IS NULL
        OR NOT EXISTS(SELECT 1 FROM game_task_board_token_sources s
            WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id
              AND s.cause_id=NEW.cause_id AND s.cause_ordinal=NEW.cause_ordinal
              AND s.amount=delta AND s.created_xact_id=pg_current_xact_id())) THEN
        RAISE EXCEPTION 'Task Board token effect requires its existing ledger participant' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_token_receipt_guard AFTER INSERT
    ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_token_receipt_guard();

-- Keep the existing ledger continuity/final-balance checks, adding only the new
-- exact same-transaction source alongside the original DAILY source predicate.
DO $migration$
DECLARE definition TEXT; previous TEXT; replacement TEXT;
BEGIN
    SELECT pg_get_functiondef('game_bounty_reroll_token_guard()'::regprocedure) INTO definition;
    previous := $old$AND l.quantity=NEW.amount AND l.status='DELIVERED') THEN$old$;
    replacement := $new$AND l.quantity=NEW.amount AND l.status='DELIVERED')
        AND NOT EXISTS(SELECT 1 FROM game_task_board_token_sources s
            WHERE s.claim_id=NEW.claim_id AND s.character_id=NEW.character_id
              AND s.world_id=NEW.world_id AND s.amount=NEW.amount
              AND s.created_xact_id=pg_current_xact_id()) THEN$new$;
    IF strpos(definition,previous)=0 THEN RAISE EXCEPTION 'unexpected bounty token guard shape'; END IF;
    EXECUTE replace(definition,previous,replacement);
END $migration$;

CREATE CONSTRAINT TRIGGER game_task_board_token_source_guard AFTER INSERT
    ON game_task_board_token_sources DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_token_source_guard();
CREATE TRIGGER game_task_board_token_sources_immutable BEFORE UPDATE OR DELETE
    ON game_task_board_token_sources FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_task_board_token_sources_no_truncate BEFORE TRUNCATE
    ON game_task_board_token_sources FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_task_board_token_source_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_task_board_token_receipt_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_bounty_reroll_token_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_task_board_token_source_guard(),game_task_board_token_receipt_guard(),game_bounty_reroll_token_guard() FROM PUBLIC;
REVOKE ALL ON game_task_board_token_sources FROM PUBLIC;
GRANT SELECT,INSERT ON game_task_board_token_sources TO oteryn_game_runtime;
