-- Quest rewards reuse the existing immutable absolute Item deadline owner.
-- Every new admission arm requires the original same-transaction reward MINT.
DO $migration$
DECLARE body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal schedule proof'; END IF;
    arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_quest_reward_sources r
        JOIN game_reward_claim_mint_receipts m ON m.transaction_id=r.mint_transaction_id
        JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
        WHERE r.mint_transaction_id=NEW.source_transaction_id AND NEW.source_ordinal=1
            AND m.item_instance_id=NEW.item_instance_id AND i.minted_transaction_id=m.transaction_id
            AND m.character_id=r.character_id AND m.game_session_id=r.cause_id AND m.command_id=r.cause_ordinal
            AND m.quantity=1 AND i.quantity=1 AND i.lifecycle=1
            AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
            AND r.plan #>> '{give,item}'=NEW.definition_production_key
            AND (r.plan #>> '{give,count}')::bigint=1
            AND m.claim_revision_ref='sha256:'||encode(NEW.content_digest,'hex')
            AND NEW.expires_at_unix_ms=m.occurred_at+NEW.duration_millis
            AND r.plan #> '{give,decay}'=jsonb_build_object('duration_millis',NEW.duration_millis,
                'target',CASE WHEN NEW.target_family IS NULL THEN NULL ELSE
                    jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,
                        'revision',NEW.target_revision) END)
            AND NEW.created_xact_id=pg_current_xact_id()
            AND r.created_xact_id=NEW.created_xact_id AND m.created_xact_id=NEW.created_xact_id)
    THEN RETURN NULL; END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF regexp_count(body,'RETURN NULL;')<>1 THEN
        RAISE EXCEPTION 'missing exact quest reward completion predecessor';
    END IF;
    arm:=$arm$
    IF NEW.plan #> '{give,decay}' IS NOT NULL THEN
        IF jsonb_typeof(NEW.plan #> '{give,decay}') IS DISTINCT FROM 'object'
            OR NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
                JOIN game_reward_claim_mint_receipts m ON m.transaction_id=p.source_transaction_id
                WHERE m.transaction_id=NEW.mint_transaction_id AND m.item_instance_id=p.item_instance_id
                    AND p.source_ordinal=1 AND p.created_xact_id=pg_current_xact_id()
                    AND m.created_xact_id=p.created_xact_id
                    AND p.expires_at_unix_ms=m.occurred_at+p.duration_millis
                    AND NEW.plan #> '{give,decay}'=jsonb_build_object('duration_millis',p.duration_millis,
                        'target',CASE WHEN p.target_family IS NULL THEN NULL ELSE
                            jsonb_build_object('family',p.target_family,'key',p.target_production_key,
                                'revision',p.target_revision) END)) THEN
            RAISE EXCEPTION 'quest reward requires its exact original absolute deadline' USING ERRCODE='23514';
        END IF;
    END IF;
$arm$;
    EXECUTE replace(body,'RETURN NULL;',arm||E'\nRETURN NULL;');
END $migration$;

-- Ordinary carried moves retain the immutable schedule and its original time.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    original:='EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)';
    IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing absolute timer movement predecessor'; END IF;
    EXECUTE replace(body,original,'('||original||' AND NOT EXISTS(
        SELECT 1 FROM game_spell_item_temporal_schedules p JOIN game_quest_reward_sources r
            ON r.mint_transaction_id=p.source_transaction_id
        JOIN game_reward_claim_mint_receipts m ON m.transaction_id=r.mint_transaction_id
        WHERE p.item_instance_id=NEW.item_instance_id AND m.item_instance_id=p.item_instance_id
            AND jsonb_typeof(r.plan #> ''{give,decay}'')=''object''))');
END $migration$;
