-- Optional NPC completion is proven by the existing owning Quest receipt/state.
-- Older plans omit the field and retain their original false completion value.
CREATE FUNCTION game_quest_reward_completion_exact() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_receipt game_character_quest_receipts%ROWTYPE; v_completes BOOLEAN;
BEGIN
    IF NEW.plan ? 'completes' AND jsonb_typeof(NEW.plan->'completes')<>'boolean' THEN
        RAISE EXCEPTION 'quest reward completion must be boolean' USING ERRCODE='23514';
    END IF;
    v_completes:=coalesce((NEW.plan->>'completes')::boolean,false);
    SELECT * INTO v_receipt FROM game_character_quest_receipts
        WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id
            AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR v_receipt.created_xact_id<>pg_current_xact_id()
        OR NEW.created_xact_id<>pg_current_xact_id()
        OR v_receipt.quest_key IS DISTINCT FROM NEW.plan->>'quest'
        OR v_receipt.completes IS DISTINCT FROM v_completes
        OR (v_completes AND NOT EXISTS(SELECT 1 FROM game_character_quest_states s
            WHERE s.character_id=NEW.character_id AND s.quest_key=v_receipt.quest_key
                AND s.completed_character_revision=v_receipt.committed_character_revision)) THEN
        RAISE EXCEPTION 'quest reward completion lacks exact owning receipt/state' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_completion_exact AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_completion_exact();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_completion_exact() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_completion_exact() FROM PUBLIC;
