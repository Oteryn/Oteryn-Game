-- WU26-02: Winter Update 2026 stores at most 10 Bounty Reroll Tokens. A credit
-- records min(requested, 10 - balance); a daily claim at the cap writes a zero
-- entry (the daily plan still names its requested line), a bounty claim at the cap
-- grants nothing. Balances already above 10 are kept but receive nothing.
ALTER TABLE game_character_bounty_reroll_token_entries
    DROP CONSTRAINT game_character_bounty_reroll_token_entries_amount_check;
ALTER TABLE game_character_bounty_reroll_token_entries
    ADD CONSTRAINT game_character_bounty_reroll_token_entries_amount_check
    CHECK(amount BETWEEN -1 AND 10);
-- A zero entry is only the capped daily credit.
ALTER TABLE game_character_bounty_reroll_token_entries
    ADD CONSTRAINT game_character_bounty_reroll_token_entries_cap_check
    CHECK(amount<>0 OR balance_before>=10);

-- Daily source: the entry credits the daily line's quantity clamped to the cap.
DO $migration$
DECLARE definition TEXT;
    previous TEXT := $old$AND l.quantity=NEW.amount AND l.status='DELIVERED')$old$;
    replacement TEXT := $new$AND NEW.amount=least(l.quantity,greatest(0,10-NEW.balance_before)) AND l.status='DELIVERED')$new$;
BEGIN
    SELECT pg_get_functiondef('game_bounty_reroll_token_guard()'::regprocedure) INTO definition;
    IF strpos(definition,previous)=0 THEN RAISE EXCEPTION 'unexpected bounty token guard shape'; END IF;
    EXECUTE replace(definition,previous,replacement);
END $migration$;

-- Task Board source: a successful bounty claim grants one token below the cap and
-- none at or above it (the reply carries the committed balance).
DO $migration$
DECLARE definition TEXT;
    previous TEXT := $old$AND delta<>1 THEN$old$;
    replacement TEXT := $new$AND delta<>CASE WHEN (NEW.reply->>'reroll_tokens')::bigint-delta>=10 THEN 0 ELSE 1 END THEN$new$;
BEGIN
    SELECT pg_get_functiondef('game_task_board_token_receipt_guard()'::regprocedure) INTO definition;
    IF strpos(definition,previous)=0 THEN RAISE EXCEPTION 'unexpected task board token receipt guard shape'; END IF;
    EXECUTE replace(definition,previous,replacement);
END $migration$;
