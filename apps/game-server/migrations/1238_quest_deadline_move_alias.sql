-- The loop trigger already declares r as its move-header record. Keep the exact
-- deadline/custody proof while giving the joined reward source its own SQL alias.
DO $migration$
DECLARE body TEXT; original TEXT; corrected TEXT;
BEGIN
    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    original := $query$JOIN game_quest_reward_sources r
            ON r.mint_transaction_id=p.source_transaction_id
        JOIN game_reward_claim_mint_receipts m ON m.transaction_id=r.mint_transaction_id
        WHERE p.item_instance_id=NEW.item_instance_id AND m.item_instance_id=p.item_instance_id
            AND jsonb_typeof(r.plan #> '{give,decay}')='object'$query$;
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1
       OR position('deadline_reward' IN body) <> 0 THEN
        RAISE EXCEPTION 'unexpected quest deadline movement predecessor';
    END IF;
    corrected := replace(original,'game_quest_reward_sources r','game_quest_reward_sources deadline_reward');
    corrected := replace(corrected,'r.mint_transaction_id','deadline_reward.mint_transaction_id');
    corrected := replace(corrected,'r.plan #>','deadline_reward.plan #>');
    EXECUTE replace(body,original,corrected);
END $migration$;
