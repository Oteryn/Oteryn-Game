-- T125: Platform holds all coins. Game journals only offers, gold custody and retry intents.
CREATE TABLE game_coin_market_offers (
 offer_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(offer_id)),
 world_id UUID NOT NULL, account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id), side SMALLINT NOT NULL CHECK(side IN(1,2)),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1 AND 64000), remaining BIGINT NOT NULL CHECK(remaining BETWEEN 0 AND amount),
 price BIGINT NOT NULL CHECK(price BETWEEN 1 AND 999999999999),
 held_gold BIGINT NOT NULL DEFAULT 0 CHECK(held_gold BETWEEN 0 AND 999999999999),
 state SMALLINT NOT NULL DEFAULT 0 CHECK(state BETWEEN 0 AND 4),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(), expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()+interval '30 days',
 CHECK((side=1 AND held_gold=remaining*price) OR (side=2 AND held_gold=0)), CHECK(amount*price<=999999999999)
);
CREATE INDEX coin_market_book ON game_coin_market_offers(world_id,state,side,price,created_at,offer_id);
CREATE INDEX coin_market_owner ON game_coin_market_offers(account_id,world_id,created_at DESC,offer_id);
CREATE INDEX coin_market_character ON game_coin_market_offers(character_id);
CREATE TABLE game_coin_operations (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL, actor UUID NOT NULL REFERENCES game_character_roots(character_id),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 seller UUID REFERENCES game_character_account_guards(account_id),buyer UUID REFERENCES game_character_account_guards(account_id),
 seller_character UUID REFERENCES game_character_roots(character_id),buyer_character UUID REFERENCES game_character_roots(character_id),
 kind TEXT NOT NULL CHECK(kind IN('reserve','reserve_take','settle','release','buy','cancel_buy','transfer')),
 buy_offer UUID REFERENCES game_coin_market_offers(offer_id),sell_offer UUID REFERENCES game_coin_market_offers(offer_id),
 reservation_id UUID, parent UUID REFERENCES game_coin_operations(operation_id),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 64000),price BIGINT NOT NULL CHECK(price BETWEEN 0 AND 999999999999),
 fee BIGINT NOT NULL DEFAULT 0 CHECK(fee BETWEEN 0 AND 1000000),
 held_gold BIGINT NOT NULL DEFAULT 0 CHECK(held_gold BETWEEN 0 AND 999999999999),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32), phase SMALLINT NOT NULL DEFAULT 0 CHECK(phase IN(0,1,2)),
 last_error TEXT NOT NULL DEFAULT '',next_attempt TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX coin_operation_pending ON game_coin_operations(world_id,phase,created_at,operation_id);
CREATE INDEX coin_operation_owner ON game_coin_operations(account_id,created_at DESC,operation_id);
CREATE INDEX coin_operation_actor ON game_coin_operations(actor);
CREATE INDEX coin_operation_seller ON game_coin_operations(seller);
CREATE INDEX coin_operation_buyer ON game_coin_operations(buyer);
CREATE INDEX coin_operation_seller_character ON game_coin_operations(seller_character);
CREATE INDEX coin_operation_buyer_character ON game_coin_operations(buyer_character);
CREATE INDEX coin_operation_buy ON game_coin_operations(buy_offer);
CREATE INDEX coin_operation_sell ON game_coin_operations(sell_offer);
CREATE INDEX coin_operation_parent ON game_coin_operations(parent);
CREATE TABLE game_coin_receipts (
 operation_id UUID PRIMARY KEY REFERENCES game_coin_operations(operation_id),
 canonical_receipt TEXT NOT NULL CHECK(octet_length(canonical_receipt)<=2048),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER coin_receipt_immutable BEFORE UPDATE OR DELETE ON game_coin_receipts FOR EACH ROW EXECUTE FUNCTION game_vip_immutable();
CREATE TABLE game_coin_gold_events (
 event_id UUID PRIMARY KEY,operation_id UUID NOT NULL REFERENCES game_coin_operations(operation_id),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),world_id UUID NOT NULL,
 bank_delta BIGINT NOT NULL,escrow_delta BIGINT NOT NULL,burn BIGINT NOT NULL CHECK(burn BETWEEN 0 AND 1000000),
 bank_entry UUID,entry JSONB,created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(bank_delta+escrow_delta+burn=0),CHECK((bank_delta=0)=(bank_entry IS NULL))
);
CREATE INDEX coin_gold_operation ON game_coin_gold_events(operation_id);
CREATE INDEX coin_gold_account ON game_coin_gold_events(account_id,world_id);
CREATE INDEX coin_gold_bank_entry ON game_coin_gold_events(bank_entry);
CREATE TRIGGER coin_gold_immutable BEFORE UPDATE OR DELETE ON game_coin_gold_events FOR EACH ROW EXECUTE FUNCTION game_vip_immutable();
ALTER TABLE game_account_bank_entries ADD COLUMN coin_event_id UUID REFERENCES game_coin_gold_events(event_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN coin_delta BIGINT;
CREATE INDEX bank_entry_coin_event ON game_account_bank_entries(coin_event_id);
DO $$ DECLARE expr TEXT; name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_account_bank_entries_one_reference','game_account_bank_entries_guild_reference','game_account_bank_entries_house_value'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint WHERE conrelid='game_account_bank_entries'::regclass AND conname=name;
  EXECUTE format('ALTER TABLE game_account_bank_entries DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT %I CHECK(CASE WHEN coin_event_id IS NOT NULL THEN num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id,guild_operation_id,market_transaction_id,house_rent_payment_id)=0 AND kind IS NULL AND coin_delta IS NOT NULL AND coin_delta<>0 AND balance_after=balance_before+coin_delta AND amount=abs(coin_delta) AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND escrow_before IS NULL AND escrow_after IS NULL ELSE coin_delta IS NULL AND (%s) END)',name,expr);
 END LOOP;
END $$;
-- Extend the established bank proof without changing any previous branch.
DO $$ DECLARE body TEXT; at INTEGER; arm TEXT:=$arm$
 IF NEW.coin_event_id IS NOT NULL THEN
  IF NEW.created_xact_id<>pg_current_xact_id()
   OR NOT EXISTS(SELECT 1 FROM game_coin_gold_events e WHERE e.event_id=NEW.coin_event_id
    AND e.account_id=NEW.account_id AND e.world_id=NEW.world_id AND e.bank_entry=NEW.entry_id
    AND e.bank_delta=NEW.coin_delta AND e.entry=to_jsonb(NEW)-'created_xact_id' AND e.created_xact_id=pg_current_xact_id())
   OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
   OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.balance_after=NEW.balance_before))
   OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=NEW.account_id AND b.world_id=NEW.world_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.previous_entry_id=NEW.entry_id)) THEN
    RAISE EXCEPTION 'coin gold requires its immutable conservation event and bank chain' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
$arm$; BEGIN
 body:=pg_get_functiondef('game_account_bank_entry_consistency_guard()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');IF at=0 THEN RAISE EXCEPTION 'unexpected bank proof';END IF;
 EXECUTE overlay(body placing 'BEGIN'||arm from at for 5);
END $$;
CREATE FUNCTION game_coin_gold_move(e UUID,op UUID,a UUID,c UUID,w UUID,d BIGINT,h BIGINT,f BIGINT) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b game_account_bank_balances; v_entry UUID; raw JSONB;
BEGIN
 IF d+h+f<>0 OR f<0 OR NOT EXISTS(SELECT 1 FROM game_coin_operations o WHERE o.operation_id=op AND o.world_id=w AND a IN(o.account_id,o.seller,o.buyer)) THEN RAISE EXCEPTION 'coin gold operation mismatch' USING ERRCODE='23514'; END IF;
 INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(a,w,0) ON CONFLICT DO NOTHING;
 SELECT * INTO STRICT b FROM game_account_bank_balances WHERE account_id=a AND world_id=w FOR UPDATE;
 IF d<>0 THEN
  v_entry:=game_character_uuid_v7();
  INSERT INTO game_account_bank_entries(entry_id,coin_event_id,coin_delta,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
   VALUES(v_entry,e,d,a,w,NULL,abs(d),b.balance,b.balance+d,b.last_entry_id,c);
  SELECT to_jsonb(x)-'created_xact_id' INTO raw FROM game_account_bank_entries x WHERE x.entry_id=v_entry;
  INSERT INTO game_coin_gold_events(event_id,operation_id,account_id,world_id,bank_delta,escrow_delta,burn,bank_entry,entry)
   VALUES(e,op,a,w,d,h,f,v_entry,raw);
  UPDATE game_account_bank_balances SET balance=b.balance+d,last_entry_id=v_entry WHERE account_id=a AND world_id=w;
 ELSE
  INSERT INTO game_coin_gold_events(event_id,operation_id,account_id,world_id,bank_delta,escrow_delta,burn) VALUES(e,op,a,w,d,h,f);
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_coin_gold_move(UUID,UUID,UUID,UUID,UUID,BIGINT,BIGINT,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_coin_gold_move(UUID,UUID,UUID,UUID,UUID,BIGINT,BIGINT,BIGINT) TO oteryn_game_runtime;
REVOKE ALL ON game_coin_market_offers,game_coin_operations,game_coin_receipts,game_coin_gold_events FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_coin_market_offers,game_coin_operations TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_coin_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_coin_gold_events TO oteryn_game_runtime;
GRANT SELECT ON game_coin_market_offers,game_coin_operations,game_coin_receipts,game_coin_gold_events TO oteryn_game_control;
-- Terms survive retries, character changes and worker restarts unchanged.
CREATE FUNCTION game_coin_operation_terms() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' OR (to_jsonb(NEW)-ARRAY['phase','last_error','next_attempt','held_gold']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['phase','last_error','next_attempt','held_gold'])
  OR NEW.held_gold>OLD.held_gold OR (OLD.phase=1 AND NEW.phase<>1) OR (OLD.phase=2 AND NEW.phase NOT IN(1,2)) THEN
  RAISE EXCEPTION 'immutable coin intent terms and monotonic progress' USING ERRCODE='23514';
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER coin_operation_terms BEFORE UPDATE OR DELETE ON game_coin_operations FOR EACH ROW EXECUTE FUNCTION game_coin_operation_terms();
CREATE FUNCTION game_coin_offer_terms() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' OR (to_jsonb(NEW)-ARRAY['state','remaining','held_gold','created_at','expires_at']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['state','remaining','held_gold','created_at','expires_at']) OR NEW.remaining>OLD.remaining THEN
  RAISE EXCEPTION 'immutable coin offer terms' USING ERRCODE='23514';
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER coin_offer_terms BEFORE UPDATE OR DELETE ON game_coin_market_offers FOR EACH ROW EXECUTE FUNCTION game_coin_offer_terms();
