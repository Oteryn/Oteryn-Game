-- Current TibiaWiki Lost Wisdom reward, retaining Crystal's mission/yes flow.
-- Forward rollback: disable Viola's adapter; retain earned state and receipts.
CREATE TABLE game_character_viola (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 stage SMALLINT NOT NULL CHECK(stage IN (1,2))
);
CREATE TABLE game_viola_receipts (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 action SMALLINT NOT NULL CHECK(action IN (1,2)),
 reply SMALLINT NOT NULL CHECK(reply BETWEEN 0 AND 6),
 craft_transaction_id UUID UNIQUE REFERENCES game_item_craft_records(transaction_id),
 xp_occurrence_id UUID UNIQUE REFERENCES game_character_xp_receipts(reward_occurrence_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,game_session_id,command_id),
 CHECK((reply=3)=(craft_transaction_id IS NOT NULL AND xp_occurrence_id IS NOT NULL)),
 CHECK(reply=3 OR (craft_transaction_id IS NULL AND xp_occurrence_id IS NULL))
);
CREATE UNIQUE INDEX game_viola_once ON game_viola_receipts(character_id) WHERE reply=3;
CREATE INDEX game_viola_receipts_session ON game_viola_receipts(game_session_id);
REVOKE ALL ON game_character_viola,game_viola_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_viola TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_viola_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_viola,game_viola_receipts TO oteryn_game_control;
CREATE FUNCTION game_viola_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; x game_character_xp_receipts%ROWTYPE;
BEGIN
 IF NEW.reply=3 THEN
  SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=NEW.craft_transaction_id;
  SELECT * INTO x FROM game_character_xp_receipts WHERE reward_occurrence_id=NEW.xp_occurrence_id;
  IF r.created_xact_id IS DISTINCT FROM pg_current_xact_id() OR NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id()
    OR r.character_id IS DISTINCT FROM NEW.character_id
    OR r.game_session_id IS DISTINCT FROM NEW.game_session_id OR r.command_id IS DISTINCT FROM NEW.command_id
    OR r.recipe IS DISTINCT FROM 'oteryn:npc-handoff.viola'
    OR r.before_item->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i51977'
    OR jsonb_array_length(r.outputs) IS DISTINCT FROM 1
    OR r.outputs->0->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i3572'
    OR (r.outputs->0->>'count')::int IS DISTINCT FROM 1
    OR NOT EXISTS(SELECT 1 FROM game_character_xp_receipts WHERE reward_occurrence_id=NEW.xp_occurrence_id AND xmin=pg_current_xact_id()::xid)
    OR x.character_id IS DISTINCT FROM NEW.character_id OR x.experience_awarded IS DISTINCT FROM 800
    OR NOT EXISTS(SELECT 1 FROM game_character_viola WHERE character_id=NEW.character_id AND stage=2)
  THEN RAISE EXCEPTION 'Viola requires atomic book hand-in, scarf and 800 XP' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_complete AFTER INSERT ON game_viola_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_complete();
CREATE TRIGGER game_viola_receipt_immutable BEFORE UPDATE OR DELETE ON game_viola_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_viola_receipt_no_truncate BEFORE TRUNCATE ON game_viola_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Do not let the general item exchange owner acquire Viola completion authority alone.
CREATE FUNCTION game_viola_handoff_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.recipe='oteryn:npc-handoff.viola' AND NOT EXISTS(
   SELECT 1 FROM game_viola_receipts WHERE craft_transaction_id=NEW.transaction_id
    AND reply=3 AND created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Viola handoff lacks its atomic completion receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_handoff_proven AFTER INSERT ON game_item_craft_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_handoff_proven();
CREATE FUNCTION game_viola_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (TG_OP='INSERT' AND NEW.stage<>1) OR
    (TG_OP='UPDATE' AND (NEW.character_id IS DISTINCT FROM OLD.character_id OR OLD.stage<>1 OR NEW.stage<>2))
 THEN RAISE EXCEPTION 'invalid Viola quest successor' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_viola_state_guard BEFORE INSERT OR UPDATE ON game_character_viola FOR EACH ROW EXECUTE FUNCTION game_viola_state_guard();
CREATE FUNCTION game_viola_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_viola_receipts WHERE character_id=NEW.character_id
   AND created_xact_id=pg_current_xact_id() AND reply=CASE NEW.stage WHEN 1 THEN 1 ELSE 3 END)
 THEN RAISE EXCEPTION 'Viola quest state requires its same-transaction receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_state_proven AFTER INSERT OR UPDATE ON game_character_viola DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_state_proven();
CREATE TRIGGER game_viola_state_no_truncate BEFORE TRUNCATE ON game_character_viola FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Reuse the carried-tree proof of the consume/mint owner for a nested book.
-- Preserve its existing W19 constraint and add only this exact NPC handoff.
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
  WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_event_source_location_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (recipe=''oteryn:npc-handoff.viola'' AND consume_source AND before_item->>''definition_production_key''=''oteryn:item.tibia.i51977'' AND jsonb_typeof(event_source_location)=''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',previous);
END $migration$;

DO $$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_viola_complete()','game_viola_handoff_proven()','game_viola_state_guard()','game_viola_state_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_viola_complete(),game_viola_handoff_proven(),game_viola_state_guard(),game_viola_state_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_viola_complete(),game_viola_handoff_proven(),game_viola_state_guard(),game_viola_state_proven() TO oteryn_game_runtime;

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
