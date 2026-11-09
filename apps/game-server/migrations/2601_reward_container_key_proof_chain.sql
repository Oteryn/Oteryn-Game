-- M35: M21's replacement key guard retained map/inbox/root MINT but lost
-- W15b's complete frozen-container proof. Restore that arm without copying
-- or replacing any installed owner branch, privileges or search path.
DO $migration$
DECLARE
    body TEXT;
    insert_at INTEGER;
    arm TEXT := $arm$
    IF TG_OP = 'INSERT' AND NEW.created_xact_id = pg_current_xact_id()
       AND game_reward_claim_container_item_proven(
           NEW.item_instance_id, NEW.minted_transaction_id) THEN
        RETURN NULL;
    END IF;
$arm$;
BEGIN
    body := pg_get_functiondef('game_item_key_proven()'::regprocedure);
    IF position(arm IN body) > 0 THEN RETURN; END IF;
    insert_at := regexp_instr(body, E'\\mBEGIN\\M', 1, 1, 1, 'i');
    IF insert_at = 0 THEN
        RAISE EXCEPTION 'unexpected key proof guard structure';
    END IF;
    EXECUTE overlay(body placing arm from insert_at for 0);
END
$migration$;
