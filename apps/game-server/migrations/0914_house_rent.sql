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
 rent_due_at TIMESTAMPTZ NOT NULL,
 delivered_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 UNIQUE(world_id,house_key,warning,rent_due_at),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
INSERT INTO game_character_inbox_cause_kinds VALUES('HOUSE_RENT_WARNING');
CREATE TABLE game_house_rent_letter_mints (
 letter_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(letter_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 world_id UUID NOT NULL,
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_house_rent_letter_mint_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'house rent letter mint is immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_house_rent_letter_mint_guard BEFORE UPDATE OR DELETE ON game_house_rent_letter_mints
 FOR EACH ROW EXECUTE FUNCTION game_house_rent_letter_mint_guard();
REVOKE ALL ON FUNCTION game_house_rent_letter_mint_guard() FROM PUBLIC;
-- Extend the central deferred ItemInstance proof with this closed, transaction-local mint.
-- The guard body is amended in place, as 0165-0762 do, so every earlier branch survives;
-- replacing it with a copied body would drop the branches added after the copied migration.
DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_item_mint_consistency_guard()'::regprocedure);
 body:=regexp_replace(body,'BEGIN',E'BEGIN\n    IF EXISTS (SELECT 1 FROM game_house_rent_letter_mints m JOIN game_item_character_inbox_locations l ON l.item_instance_id=m.letter_id WHERE m.letter_id=NEW.item_instance_id AND m.transaction_id=NEW.minted_transaction_id AND m.world_id=NEW.world_id AND l.character_id=m.owner_character_id AND m.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;');
 EXECUTE body;
END $$;

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
DECLARE p game_house_properties; s game_house_rent_state; b game_account_bank_balances; entry UUID; payment UUID; letter UUID; letter_tx UUID; due_cycle TIMESTAMPTZ; now_ms BIGINT; period INTERVAL;
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
  SELECT world_id,house_key,greatest(paid_until,statement_timestamp()) FROM game_house_properties
   WHERE world_id=w AND owner_character_id IS NOT NULL ON CONFLICT DO NOTHING;
 SELECT hp.* INTO p FROM game_house_properties hp JOIN game_house_rent_state rs USING(world_id,house_key)
  WHERE hp.world_id=w AND hp.owner_character_id IS NOT NULL AND hp.paid_until<=statement_timestamp() AND rs.next_rent_at<=statement_timestamp()
  ORDER BY rs.next_rent_at,hp.house_key LIMIT 1 FOR UPDATE OF hp,rs SKIP LOCKED;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO s FROM game_house_rent_state WHERE world_id=w AND house_key=p.house_key FOR UPDATE;
 due_cycle:=s.next_rent_at;
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
  UPDATE game_house_properties SET paid_until=greatest(paid_until,statement_timestamp())+period WHERE world_id=w AND house_key=p.house_key;
  UPDATE game_house_rent_state SET warnings=0,next_rent_at=(SELECT paid_until FROM game_house_properties WHERE world_id=w AND house_key=p.house_key) WHERE world_id=w AND house_key=p.house_key;
  RETURN TRUE;
 END IF;
 IF s.warnings<7 THEN
  UPDATE game_house_rent_state SET warnings=warnings+1,next_rent_at=next_rent_at+period WHERE world_id=w AND house_key=p.house_key RETURNING * INTO s;
  letter:=game_character_uuid_v7();letter_tx:=game_character_uuid_v7();
  INSERT INTO game_house_rent_inbox_letters(letter_id,world_id,house_key,owner_character_id,warning,rent_gold,rent_due_at)
   VALUES(letter,w,p.house_key,p.owner_character_id,s.warnings,p.rent_gold,due_cycle);
  INSERT INTO game_house_rent_letter_mints VALUES(letter,letter_tx,w,p.owner_character_id);
  INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
   VALUES(letter,w,'Item','oteryn:item.tibia.i3506','house-rent-warning',1,1,letter_tx);
  PERFORM game_character_inbox_deliver(letter,p.owner_character_id,w,'HOUSE_RENT_WARNING',letter::text);
 ELSE
  UPDATE game_house_rent_state SET next_rent_at=next_rent_at+period WHERE world_id=w AND house_key=p.house_key;
 END IF;
 IF s.warnings=7 AND NOT EXISTS(
  SELECT 1 FROM game_house_dispositions d WHERE d.world_id=w AND d.house_key=p.house_key
   AND d.cause='EVICTION' AND d.state<>4) THEN
  now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  BEGIN
   INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause)
    SELECT w,p.house_key,o.owner_character_id,o.ownership_revision,1,now_ms,now_ms,'EVICTION' FROM game_house_ownership o
     WHERE o.world_id=w AND o.house_key=p.house_key;
   UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=w AND runtime_scope_kind=2
    AND runtime_scope_house_key=p.house_key AND session_state IN(1,2);
  EXCEPTION WHEN unique_violation THEN
   -- A competing disposition belongs to this house only. Keep its warning
   -- transition committed so the world worker can advance the next house.
   NULL;
  END;
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

REVOKE ALL ON game_house_rent_config,game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_letter_mints,game_house_rent_payments FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_house_rent_state TO oteryn_game_runtime;
GRANT SELECT ON game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_runtime;
GRANT SELECT,UPDATE(rent_period) ON game_house_rent_config TO oteryn_game_control;
GRANT SELECT ON game_house_rent_config,game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_runtime;
GRANT SELECT ON game_house_rent_state,game_house_rent_inbox_letters,game_house_rent_payments TO oteryn_game_control;
-- game_item_mint_consistency_guard runs as the minting role and reads this proof table.
GRANT SELECT ON game_house_rent_letter_mints TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_house_disposition_is_personal_move_out(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_disposition_is_personal_move_out(UUID) TO oteryn_game_runtime,oteryn_game_control;
