-- QMECH/qready2 extends the additional-item owner installed by main 1550.
-- Reuse its columns, indexes, complete metadata proof and guard arms; add the
-- ordinary-fluid restriction and the exact quest-plan sibling correspondence.
CREATE FUNCTION game_reward_claim_siblings_valid(p_plan JSONB, p_capacity SMALLINT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE part JSONB;
BEGIN
    IF NOT game_reward_claim_contents_plan_valid(p_plan,p_capacity) THEN RETURN FALSE; END IF;
    IF p_capacity IS NULL THEN RETURN p_plan='[]'::jsonb; END IF;
    IF jsonb_array_length(p_plan)>=p_capacity THEN RETURN FALSE; END IF;
    FOR part IN SELECT value FROM jsonb_array_elements(p_plan) LOOP
        -- Main's complete participant proves authored key/text metadata. A
        -- variable-fluid sibling has no participant in this ordinary owner.
        IF part ? 'fluid_subtype' THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END $$;

ALTER TABLE game_reward_claim_mint_reservations
 ADD CONSTRAINT game_reward_claim_reserved_siblings CHECK (
  game_reward_claim_siblings_valid(additional_plan,additional_capacity));
ALTER TABLE game_reward_claim_mint_receipts
 ADD CONSTRAINT game_reward_claim_committed_siblings CHECK (
  game_reward_claim_siblings_valid(additional_plan,additional_capacity));

DO $migration$
DECLARE definition TEXT; original TEXT; replacement TEXT; position INTEGER;
BEGIN
    definition:=pg_get_functiondef('game_reward_claim_additional_complete(UUID)'::regprocedure);
    original:='game_reward_claim_contents_plan_valid(r.additional_plan,r.additional_capacity)';
    replacement:='game_reward_claim_siblings_valid(r.additional_plan,r.additional_capacity)';
    IF (length(definition)-length(replace(definition,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed additional reward shape proof'; END IF;
    definition:=replace(definition,original,replacement);
    original:='OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=child)';
    replacement:=original || E'\n   OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=child)';
    IF (length(definition)-length(replace(definition,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed additional reward item proof'; END IF;
    EXECUTE replace(definition,original,replacement);

    definition:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    position:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,1,'i');
    IF position=0 THEN RAISE EXCEPTION 'missing quest sibling guard insertion point'; END IF;
    EXECUTE overlay(definition placing $guard$
    IF NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.transaction_id=NEW.mint_transaction_id
           AND jsonb_array_length(r.additional_plan)=jsonb_array_length(
               coalesce(NEW.plan #> '{give,additional}','[]'::JSONB))
           AND NOT EXISTS (
               SELECT 1 FROM jsonb_array_elements(r.additional_plan) actual
                WHERE NOT EXISTS (
                    SELECT 1 FROM jsonb_array_elements(coalesce(
                        NEW.plan #> '{give,additional}','[]'::JSONB))
                        WITH ORDINALITY expected(item,ordinal)
                     WHERE expected.ordinal=(actual->>'ordinal')::BIGINT
                       AND expected.item->>'item'=actual->>'production_key'
                       AND (expected.item->>'count')::BIGINT=(actual->>'quantity')::BIGINT))
           AND game_reward_claim_additional_complete(r.transaction_id)) THEN
        RAISE EXCEPTION 'quest reward lacks exact independent outputs' USING ERRCODE='23514';
    END IF;
$guard$ from position for 0);
    EXECUTE format('ALTER FUNCTION game_reward_claim_additional_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_siblings_valid(JSONB,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_reward_claim_siblings_valid(JSONB,SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_reward_claim_siblings_valid(JSONB,SMALLINT)
 TO oteryn_game_runtime,oteryn_game_control;
