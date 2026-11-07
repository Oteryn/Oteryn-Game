-- Consume QENGINE2 1004 unchanged; extend its fixed-width occurrence identity
-- to positive-cost exchanges and world gifts earned through a new state cycle.
DO $migration$
DECLARE body TEXT; original TEXT:='SELECT CASE WHEN jsonb_array_length(v_plan->''storage'')=0';
BEGIN
    body:=pg_get_functiondef('game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB)'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed quest occurrence key';
    END IF;
    EXECUTE replace(body,original,
        'SELECT CASE WHEN coalesce((v_plan->>''repeatable'')::boolean,false) OR jsonb_array_length(v_plan->''storage'')=0');
END $migration$;

CREATE FUNCTION game_quest_earned_repeat_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT (NEW.plan ? 'repeatable') THEN RETURN NULL; END IF;
    IF jsonb_typeof(NEW.plan->'repeatable') IS DISTINCT FROM 'boolean' THEN
        RAISE EXCEPTION 'quest repeat flag must be boolean' USING ERRCODE='23514';
    END IF;
    IF NOT (NEW.plan->>'repeatable')::boolean THEN RETURN NULL; END IF;
    IF jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'
        OR NEW.mint_transaction_id IS NULL
        OR coalesce(NEW.plan->'outfit','null'::jsonb)<>'null'::jsonb
        OR coalesce(NEW.plan->'achievement','null'::jsonb)<>'null'::jsonb
        OR coalesce(NEW.plan->'blessing','null'::jsonb)<>'null'::jsonb THEN
        RAISE EXCEPTION 'repeatable quest lacks exact item reward' USING ERRCODE='23514';
    END IF;
    -- The unchanged source-completion proof independently checks every positive
    -- authored input against same-transaction custody and consumption records.
    IF jsonb_array_length(NEW.plan->'take') BETWEEN 1 AND 8
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
            WHERE CASE WHEN e->>'count' ~ '^[0-9]{1,5}$'
                THEN (e->>'count')::bigint<=0 ELSE true END) THEN
        RETURN NULL;
    END IF;
    -- A free world reward must leave its eligible state range, so another
    -- producer must earn that state again before a fresh occurrence can mint.
    IF NEW.plan #>> '{world_callback,repeatable_reward}'='true'
        AND NEW.plan #>> '{world_callback,trigger}'='use'
        AND NEW.plan #> '{world_callback,storage}'=NEW.plan->'storage'
        AND NEW.plan #> '{world_callback,effects,item_reward}'=NEW.plan->'give'
        AND jsonb_array_length(NEW.plan->'take')=0
        AND EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'storage') s
            WHERE NOT coalesce((s->>'preserve')::boolean,false)
                AND (s->>'set')::bigint NOT BETWEEN (s->>'minimum')::bigint AND (s->>'maximum')::bigint) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'repeatable quest lacks positive inputs or an earned state cycle' USING ERRCODE='23514';
END $$;
CREATE CONSTRAINT TRIGGER game_quest_earned_repeat_complete
    AFTER INSERT ON game_quest_reward_sources DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_earned_repeat_complete();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_earned_repeat_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_earned_repeat_complete() FROM PUBLIC;
