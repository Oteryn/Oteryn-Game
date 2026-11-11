-- M09: the existing native NPC programme participant proves every output,
-- retaining all previous input, source programme, quest and primary MINT arms.
DO $migration$
DECLARE definition TEXT; position INTEGER; participant TEXT := $guard$
    IF NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.transaction_id=NEW.mint_transaction_id
           AND jsonb_array_length(r.additional_plan)=jsonb_array_length(
               coalesce(NEW.plan #> '{give,additional}','[]'::JSONB))
           AND NOT EXISTS (
               SELECT 1 FROM jsonb_array_elements(r.additional_plan) AS m09_actual(value)
                WHERE NOT EXISTS (
                    SELECT 1 FROM jsonb_array_elements(coalesce(
                        NEW.plan #> '{give,additional}','[]'::JSONB))
                        WITH ORDINALITY m09_expected(item,ordinal)
                     WHERE m09_expected.ordinal=(m09_actual.value->>'ordinal')::BIGINT
                       AND m09_expected.item->>'item'=m09_actual.value->>'production_key'
                       AND (m09_expected.item->>'count')::BIGINT=(m09_actual.value->>'quantity')::BIGINT))
           AND game_reward_claim_additional_complete(r.transaction_id)) THEN
        RAISE EXCEPTION 'NPC reward lacks exact independent outputs' USING ERRCODE='23514';
    END IF;
$guard$;
BEGIN
    definition:=pg_get_functiondef('game_quest_npc_item_complete()'::regprocedure);
    position:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,1,'i');
    IF position=0 THEN RAISE EXCEPTION 'missing NPC sibling participant insertion point'; END IF;
    EXECUTE overlay(definition placing participant from position for 0);
END $migration$;
