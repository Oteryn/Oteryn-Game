-- Current TibiaWiki Book of Wisdom (51977): one per character per 300 seconds.
-- Forward rollback disables the literal USE adapter; earned items/receipts remain.
CREATE TABLE game_character_viola_book_cycles (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 cycle BIGINT NOT NULL CHECK(cycle>0),
 claimed_at BIGINT NOT NULL,
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_reward_claim_mint_receipts(transaction_id),
 PRIMARY KEY(character_id,cycle)
);
REVOKE ALL ON game_character_viola_book_cycles FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_viola_book_cycles TO oteryn_game_runtime;
GRANT SELECT ON game_character_viola_book_cycles TO oteryn_game_control;
CREATE FUNCTION game_viola_book_now_ms() RETURNS BIGINT LANGUAGE SQL VOLATILE AS $$
 SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint
$$;
CREATE FUNCTION game_viola_book_cycle_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous_cycle BIGINT; previous_time BIGINT;
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 SELECT cycle,claimed_at INTO previous_cycle,previous_time
 FROM game_character_viola_book_cycles WHERE character_id=NEW.character_id ORDER BY cycle DESC LIMIT 1;
 IF NEW.cycle<>COALESCE(previous_cycle,0)+1
    OR NEW.claimed_at NOT BETWEEN game_viola_book_now_ms()-10000 AND game_viola_book_now_ms()
    OR (previous_time IS NOT NULL AND NEW.claimed_at::numeric<previous_time::numeric+300000)
    OR NOT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
      WHERE r.transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id
       AND r.claim_family='RewardClaim'
       AND r.claim_production_key='oteryn:reward-claim.tutorial.viola-book:cycle:'||NEW.cycle::text
       AND r.claim_revision_ref='reward-claim-r1' AND r.quantity=1
       AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i51977'
       AND i.definition_revision_ref='definition-r1'
       AND r.occurred_at=NEW.claimed_at AND r.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Book of Wisdom requires its atomic repeat-eligible mint' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_viola_book_cycle_guard BEFORE INSERT ON game_character_viola_book_cycles
 FOR EACH ROW EXECUTE FUNCTION game_viola_book_cycle_guard();
CREATE TRIGGER game_viola_book_cycle_immutable BEFORE UPDATE OR DELETE ON game_character_viola_book_cycles
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_viola_book_cycle_no_truncate BEFORE TRUNCATE ON game_character_viola_book_cycles
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE FUNCTION game_viola_book_receipt_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF starts_with(NEW.claim_production_key,'oteryn:reward-claim.tutorial.viola-book') AND NOT EXISTS(
   SELECT 1 FROM game_character_viola_book_cycles c WHERE c.transaction_id=NEW.transaction_id
    AND c.character_id=NEW.character_id AND c.claimed_at=NEW.occurred_at
    AND NEW.claim_family='RewardClaim'
    AND NEW.claim_production_key='oteryn:reward-claim.tutorial.viola-book:cycle:'||c.cycle::text)
 THEN RAISE EXCEPTION 'Book of Wisdom mint lacks its repeat receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_book_receipt_guard AFTER INSERT ON game_reward_claim_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_book_receipt_guard();
DO $$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_viola_book_now_ms()','game_viola_book_cycle_guard()','game_viola_book_receipt_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_viola_book_now_ms(),game_viola_book_cycle_guard(),game_viola_book_receipt_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_viola_book_now_ms(),game_viola_book_cycle_guard(),game_viola_book_receipt_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_viola_book_now_ms() TO oteryn_game_control;
