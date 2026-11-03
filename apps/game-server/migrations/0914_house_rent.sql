-- S05. Crystal donor: src/map/house/house.cpp and houses.cpp, payHouses;
-- config.lua houseRentPeriod (commit 201189f871d23597b82caf88fd611454a2ddefb8).
-- One failed due-period creates one Inbox warning; the seventh failure evicts.
CREATE TABLE game_house_rent_state (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 warnings SMALLINT NOT NULL DEFAULT 0 CHECK(warnings BETWEEN 0 AND 7),
 next_rent_at TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE TABLE game_house_rent_config (
 singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton),
 rent_period TEXT NOT NULL CHECK(rent_period IN('daily','weekly','monthly','yearly','never'))
);
INSERT INTO game_house_rent_config VALUES(TRUE,'monthly');
CREATE TABLE game_house_rent_inbox_letters (
 letter_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(letter_id)),
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 warning SMALLINT NOT NULL CHECK(warning BETWEEN 1 AND 7),
 rent_gold BIGINT NOT NULL CHECK(rent_gold BETWEEN 0 AND 999999999999),
 delivered_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 UNIQUE(world_id,house_key,warning),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE TABLE game_house_rent_payments (
 payment_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(payment_id)),
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 999999999999),
 bank_entry_id UUID NOT NULL UNIQUE,
 paid_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);

ALTER TABLE game_account_bank_entries
 ADD COLUMN house_rent_payment_id UUID UNIQUE REFERENCES game_house_rent_payments(payment_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN house_rent_kind TEXT CHECK(house_rent_kind='HOUSE_RENT'),
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_guild_reference,
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id,guild_operation_id,market_transaction_id,house_rent_payment_id)=1),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
  (guild_operation_id IS NOT NULL)=(guild_kind IS NOT NULL)
  AND (market_transaction_id IS NOT NULL)=(market_kind IS NOT NULL)
  AND (house_rent_payment_id IS NOT NULL)=(house_rent_kind IS NOT NULL)
  AND (kind IS NULL)=(guild_kind IS NOT NULL OR market_kind IS NOT NULL OR house_rent_kind IS NOT NULL));
DO $$ DECLARE expr TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint
  WHERE conrelid='game_account_bank_entries'::regclass AND conname='game_account_bank_entries_house_value';
 ALTER TABLE game_account_bank_entries DROP CONSTRAINT game_account_bank_entries_house_value;
 EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT game_account_bank_entries_house_value CHECK(CASE WHEN house_rent_kind IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND balance_after=balance_before-amount ELSE (%s) END)',expr);
END $$;

CREATE FUNCTION game_house_rent_tick(w UUID,n UUID,nr NUMERIC,secret BYTEA) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_properties; s game_house_rent_state; b game_account_bank_balances; entry UUID; payment UUID; now_ms BIGINT; period INTERVAL;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1 AND state=1
   AND holder_node_id=n AND holder_registration_revision=nr) THEN
  RAISE EXCEPTION 'house rent ingress refused' USING ERRCODE='23514';
 END IF;
 SELECT CASE rent_period WHEN 'daily' THEN interval '1 day' WHEN 'weekly' THEN interval '7 days'
  WHEN 'monthly' THEN interval '30 days' WHEN 'yearly' THEN interval '365 days' END INTO period FROM game_house_rent_config;
 IF period IS NULL THEN RETURN FALSE; END IF;
 INSERT INTO game_house_rent_state(world_id,house_key,next_rent_at)
  SELECT world_id,house_key,statement_timestamp()+period FROM game_house_properties
   WHERE world_id=w AND owner_character_id IS NOT NULL ON CONFLICT DO NOTHING;
 SELECT hp.* INTO p FROM game_house_properties hp JOIN game_house_rent_state rs USING(world_id,house_key)
  WHERE hp.world_id=w AND hp.owner_character_id IS NOT NULL AND rs.next_rent_at<=statement_timestamp()
  ORDER BY rs.next_rent_at,hp.house_key LIMIT 1 FOR UPDATE OF hp,rs SKIP LOCKED;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO s FROM game_house_rent_state WHERE world_id=w AND house_key=p.house_key FOR UPDATE;
 SELECT * INTO b FROM game_account_bank_balances WHERE account_id=p.owner_account_id AND world_id=w FOR UPDATE;
 IF p.rent_gold=0 OR (FOUND AND b.balance>=p.rent_gold) THEN
  IF p.rent_gold>0 THEN
   payment:=game_character_uuid_v7();entry:=game_character_uuid_v7();
   INSERT INTO game_house_rent_payments(payment_id,world_id,house_key,account_id,owner_character_id,amount,bank_entry_id)
    VALUES(payment,w,p.house_key,p.owner_account_id,p.owner_character_id,p.rent_gold,entry);
   INSERT INTO game_account_bank_entries(entry_id,house_rent_payment_id,house_rent_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
    VALUES(entry,payment,'HOUSE_RENT',p.owner_account_id,w,NULL,p.rent_gold,b.balance,b.balance-p.rent_gold,b.last_entry_id,p.owner_character_id);
   UPDATE game_account_bank_balances SET balance=b.balance-p.rent_gold,last_entry_id=entry WHERE account_id=p.owner_account_id AND world_id=w;
  END IF;
  UPDATE game_house_rent_state SET warnings=0,next_rent_at=next_rent_at+period WHERE world_id=w AND house_key=p.house_key;
  RETURN TRUE;
 END IF;
 UPDATE game_house_rent_state SET warnings=warnings+1,next_rent_at=next_rent_at+period WHERE world_id=w AND house_key=p.house_key RETURNING * INTO s;
 INSERT INTO game_house_rent_inbox_letters(letter_id,world_id,house_key,owner_character_id,warning,rent_gold)
  VALUES(game_character_uuid_v7(),w,p.house_key,p.owner_character_id,s.warnings,p.rent_gold);
 IF s.warnings=7 THEN
  now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause)
   SELECT w,p.house_key,o.owner_character_id,o.ownership_revision,1,now_ms,now_ms,'EVICTION' FROM game_house_ownership o
    WHERE o.world_id=w AND o.house_key=p.house_key;
  UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=w AND runtime_scope_kind=2
   AND runtime_scope_house_key=p.house_key AND session_state IN(1,2);
 END IF;
 RETURN TRUE;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- The established terminal disposition path already moves every item, clears all
-- ACL lists and releases ownership atomically. Admit personal rent eviction to it.
DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_house_move_out_release(uuid)'::regprocedure);
 body:=replace(body,'d.cause<>''MOVE_OUT''','d.cause NOT IN(''MOVE_OUT'',''EVICTION'')');
 EXECUTE body;
END $$;
CREATE OR REPLACE FUNCTION game_house_disposition_is_personal_move_out(op UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_house_dispositions%ROWTYPE;
BEGIN
 SELECT * INTO d FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'house disposition release operation missing' USING ERRCODE='23514'; END IF;
 RETURN d.cause IN('MOVE_OUT','EVICTION') AND d.owner_guild_id IS NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_move_out_release(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_disposition_is_personal_move_out(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

REVOKE ALL ON game_house_rent_config,game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_payments FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_house_rent_state TO oteryn_game_runtime;
GRANT SELECT ON game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_runtime;
GRANT SELECT,UPDATE(rent_period) ON game_house_rent_config TO oteryn_game_control;
GRANT SELECT ON game_house_rent_config,game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_runtime;
GRANT SELECT ON game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
