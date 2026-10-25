-- MARKET-1: runtime remains SELECT-only on Market tables. A bounded definer
-- composes the native Bank chain, CharacterDepot and CharacterInbox owners.
CREATE TABLE game_market_settlements (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 offer_id UUID NOT NULL,
 world_id UUID NOT NULL,
 actor UUID REFERENCES game_character_roots,
 account_id UUID NOT NULL REFERENCES game_character_account_guards,
 kind SMALLINT NOT NULL CHECK(kind IN(1,2,3,4,5,6)),
 fee BIGINT NOT NULL CHECK(fee BETWEEN 0 AND 1000000),
 fee_cause BYTEA,
 CHECK((fee>0)=(fee_cause IS NOT NULL)),
 escrow_delta BIGINT NOT NULL,
 held_delta BIGINT NOT NULL DEFAULT 0,
 moved_units INTEGER NOT NULL CHECK(moved_units BETWEEN 0 AND 64000),
 occurred_at_us BIGINT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(offer_id,world_id) REFERENCES game_market_offers(offer_id,world_id)
);
CREATE TABLE game_market_item_lines (
 transaction_id UUID NOT NULL REFERENCES game_market_settlements DEFERRABLE INITIALLY DEFERRED,
 source_item UUID NOT NULL REFERENCES game_item_instances,
 output_item UUID NOT NULL,
 before_item JSONB NOT NULL,
 source_location JSONB NOT NULL,
 moved INTEGER NOT NULL CHECK(moved BETWEEN 1 AND 64000),
 destination_character UUID REFERENCES game_character_roots,
 destination_offer UUID REFERENCES game_market_offers,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,source_item),
 UNIQUE(transaction_id,output_item),
 CHECK(num_nonnulls(destination_character,destination_offer)=1),
 CHECK(moved<=(before_item->>'quantity')::bigint),
 CHECK((source_item=output_item)=(moved=(before_item->>'quantity')::bigint))
);
CREATE INDEX market_item_line_source ON game_market_item_lines(source_item,created_xact_id);
-- Expected native ledger entries are private writer evidence. Runtime's old
-- Bank INSERT grant cannot append unrelated transfers to a valid Market receipt.
CREATE TABLE game_market_value_lines (
 transaction_id UUID NOT NULL REFERENCES game_market_settlements,
 entry_id UUID PRIMARY KEY REFERENCES game_account_bank_entries DEFERRABLE INITIALLY DEFERRED,
 entry JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX market_value_line_transaction ON game_market_value_lines(transaction_id);
CREATE TABLE game_market_history (
 transaction_id UUID NOT NULL REFERENCES game_market_settlements,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 offer_id UUID NOT NULL REFERENCES game_market_offers,
 amount INTEGER NOT NULL CHECK(amount BETWEEN 1 AND 64000),
 price BIGINT NOT NULL CHECK(price BETWEEN 1 AND 999999999999),
 occurred_at_us BIGINT NOT NULL,
 PRIMARY KEY(transaction_id,character_id)
);
CREATE INDEX market_history_owner ON game_market_history(character_id,occurred_at_us DESC,transaction_id DESC);
CREATE TABLE game_market_event_outbox (
 transaction_id UUID PRIMARY KEY REFERENCES game_market_settlements,
 payload JSONB NOT NULL,
 payload_sha256 BYTEA NOT NULL CHECK(octet_length(payload_sha256)=32),
 occurred_at_us BIGINT NOT NULL,
 expires_at_us BIGINT NOT NULL,
 CHECK(expires_at_us=occurred_at_us+2592000000000)
);
INSERT INTO game_character_inbox_cause_kinds VALUES('MARKET');
ALTER TABLE game_market_offers ADD COLUMN inbox_reservation INTEGER NOT NULL DEFAULT 0 CHECK(inbox_reservation BETWEEN 0 AND 64000);
ALTER TABLE game_character_inbox_counters ADD COLUMN market_reserved BIGINT NOT NULL DEFAULT 0 CHECK(market_reserved BETWEEN 0 AND 100000);
CREATE FUNCTION game_market_reservation_count() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 UPDATE game_character_inbox_counters SET market_reserved=market_reserved+NEW.inbox_reservation-OLD.inbox_reservation WHERE character_id=NEW.character_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'Market reservation needs its Inbox counter' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER market_reservation_count AFTER UPDATE OF inbox_reservation ON game_market_offers FOR EACH ROW EXECUTE FUNCTION game_market_reservation_count();
ALTER TABLE game_account_bank_entries
 ADD COLUMN market_transaction_id UUID REFERENCES game_market_settlements DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN market_kind TEXT CHECK(market_kind IN('MARKET_ESCROW','MARKET_ESCROW_RETURN','MARKET_PURCHASE','MARKET_SALE','MARKET_HELD_CREDIT','FEE_DEBIT')),
 ADD COLUMN counterparty_character_id UUID REFERENCES game_character_roots,
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_guild_reference,
 DROP CONSTRAINT game_account_bank_entries_house_value,
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id,guild_operation_id,market_transaction_id)=1),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
  (guild_operation_id IS NOT NULL)=(guild_kind IS NOT NULL)
  AND (market_transaction_id IS NOT NULL)=(market_kind IS NOT NULL)
  AND (kind IS NULL)=(guild_kind IS NOT NULL OR market_kind IS NOT NULL)),
 ADD CONSTRAINT game_account_bank_entries_house_value CHECK(
  CASE WHEN market_kind IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL
   AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
   AND balance_after=balance_before+CASE WHEN market_kind IN('MARKET_ESCROW','MARKET_PURCHASE','FEE_DEBIT') THEN -amount ELSE amount END
  WHEN guild_kind IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL
    AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
    AND balance_after=balance_before+CASE guild_kind WHEN 'GUILD_DEPOSIT' THEN -amount ELSE amount END
  WHEN kind>=7 THEN escrow_before BETWEEN 0 AND 999999999999 AND escrow_after BETWEEN 0 AND 999999999999
   AND escrow_before IS NOT NULL AND escrow_after IS NOT NULL
   AND balance_after=balance_before+CASE kind WHEN 7 THEN -amount WHEN 8 THEN amount ELSE 0 END
   AND escrow_after=escrow_before+CASE kind WHEN 7 THEN amount ELSE -amount END
  ELSE escrow_before IS NULL AND escrow_after IS NULL
   AND balance_after=balance_before+CASE WHEN kind IN(1,4) THEN amount ELSE -amount END END);
CREATE INDEX market_held_jobs ON game_market_offers(world_id,expires_at_us,offer_id) WHERE state=5;
CREATE UNIQUE INDEX market_bank_line_kind ON game_account_bank_entries(market_transaction_id,account_id,market_kind) WHERE market_transaction_id IS NOT NULL;

ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
 ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(cause_kind IN(1,2,3,4,5,6));
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=6 AND charm_key IS NULL AND purchase_service_key=''oteryn:market.offer'' AND fee_gold_units BETWEEN 20 AND 1000000 AND line_count=0))';
END $$;

-- Terms are immutable after INSERT; re-locking the maker's Character root
-- from an accept would invert root -> book ordering against the maker's command.
DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_market_offer_guard()'::regprocedure);
 body:=replace(body,'FOR UPDATE;','FOR NO KEY UPDATE;');
 body:=replace(body, E'    SELECT world_id,account_id INTO v_world,v_account', E'    IF TG_OP = ''UPDATE'' THEN RETURN NEW; END IF;\n    SELECT world_id,account_id INTO v_world,v_account');
 EXECUTE body;
END $$;

-- Derive planned split/entry identities deterministically from the receipt.
CREATE FUNCTION game_market_identity(t UUID,label TEXT) RETURNS UUID LANGUAGE sql IMMUTABLE AS $$
 SELECT (substr(h,1,8)||'-'||substr(h,9,4)||'-7'||substr(h,14,3)||'-8'||substr(h,18,3)||'-'||substr(h,21,12))::uuid
 FROM (SELECT encode(sha256(uuid_send(t)||convert_to(label,'UTF8')),'hex') h) s
$$;
CREATE FUNCTION game_market_complete(t UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_market_settlements%ROWTYPE; l game_market_item_lines%ROWTYPE; i game_item_instances%ROWTYPE; n game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_market_settlements WHERE transaction_id=t;
 IF r.created_xact_id<>pg_current_xact_id()
  OR COALESCE((SELECT sum(balance_after-balance_before) FROM game_account_bank_entries WHERE market_transaction_id=t),0)+r.escrow_delta+r.held_delta+r.fee<>0
  OR NOT EXISTS(SELECT 1 FROM game_market_operations WHERE transaction_id=t AND world_id=r.world_id)
  OR NOT EXISTS(SELECT 1 FROM game_market_event_outbox WHERE transaction_id=t
       AND payload=to_jsonb(r)-'created_xact_id' AND payload_sha256=sha256(convert_to(payload::text,'UTF8'))
       AND occurred_at_us=r.occurred_at_us)
  OR (SELECT count(*) FROM game_market_value_lines WHERE transaction_id=t)>3
  OR (SELECT count(*) FROM game_account_bank_entries WHERE market_transaction_id=t)<>(SELECT count(*) FROM game_market_value_lines WHERE transaction_id=t)
  OR EXISTS(SELECT 1 FROM game_market_value_lines v LEFT JOIN game_account_bank_entries e USING(entry_id)
       WHERE v.transaction_id=t AND (e.entry_id IS NULL OR v.created_xact_id<>pg_current_xact_id() OR v.entry IS DISTINCT FROM to_jsonb(e)-'created_xact_id'))
  OR (SELECT count(*) FROM game_market_item_lines WHERE transaction_id=t)>100
  OR COALESCE((SELECT sum(moved) FROM game_market_item_lines WHERE transaction_id=t),0)<>r.moved_units THEN
  RAISE EXCEPTION 'Market conservation/receipt/event mismatch' USING ERRCODE='23514';
 END IF;
 FOR l IN SELECT * FROM game_market_item_lines WHERE transaction_id=t ORDER BY source_item LIMIT 100 LOOP
  SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=l.source_item;
  IF i.last_transaction_id IS DISTINCT FROM t OR i.lifecycle<>1
    OR (to_jsonb(i)-ARRAY['quantity','last_transaction_id']) IS DISTINCT FROM (l.before_item-ARRAY['quantity','last_transaction_id'])
    OR i.quantity<>((l.before_item->>'quantity')::bigint-(CASE WHEN l.source_item=l.output_item THEN 0 ELSE l.moved END)) THEN
   RAISE EXCEPTION 'Market source state or units mismatch' USING ERRCODE='23514'; END IF;
  SELECT * INTO STRICT n FROM game_item_instances WHERE item_instance_id=l.output_item;
  IF n.quantity<>l.moved OR n.lifecycle<>1 OR n.world_id<>r.world_id
    OR (n.definition_family,n.definition_production_key,n.definition_revision_ref) IS DISTINCT FROM
       (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    OR (l.source_item<>l.output_item AND (n.minted_transaction_id<>t OR n.last_transaction_id IS NOT NULL OR n.state_revision<>1))
    OR (l.destination_offer IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_market_escrow_locations WHERE item_instance_id=l.output_item AND offer_id=l.destination_offer AND world_id=r.world_id))
    OR (l.destination_character IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=l.output_item AND character_id=l.destination_character AND world_id=r.world_id)) THEN
   RAISE EXCEPTION 'Market output/custody mismatch' USING ERRCODE='23514'; END IF;
  IF l.source_item<>l.output_item THEN
   IF l.source_location ? 'offer_id' THEN
    IF NOT EXISTS(SELECT 1 FROM game_item_market_escrow_locations WHERE item_instance_id=l.source_item AND to_jsonb(game_item_market_escrow_locations)=l.source_location) THEN
     RAISE EXCEPTION 'Market split source moved' USING ERRCODE='23514'; END IF;
   ELSE
    IF NOT EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=l.source_item AND to_jsonb(game_item_character_depot_locations)=l.source_location) THEN
     RAISE EXCEPTION 'Market split source moved' USING ERRCODE='23514'; END IF;
   END IF;
  END IF;
  PERFORM game_item_location_exclusive(l.source_item);
  PERFORM game_item_location_exclusive(l.output_item);
 END LOOP;
END $$;
CREATE FUNCTION game_market_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_market_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER market_settlement_consistent AFTER INSERT ON game_market_settlements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_market_consistent();
CREATE CONSTRAINT TRIGGER market_line_consistent AFTER INSERT ON game_market_item_lines
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_market_consistent();

-- Preserve all existing proof arms. Fresh same-transaction immutable Market
-- evidence is the only additional admission; runtime cannot insert that evidence.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_mint_consistency_guard','game_character_depot_remove','game_item_fee_burn_consistency_guard','game_account_bank_entry_consistency_guard'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'unexpected proof body %',name; END IF;
  IF name='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_market_item_lines l WHERE l.source_item=OLD.item_instance_id AND l.transaction_id=NEW.last_transaction_id AND l.created_xact_id=pg_current_xact_id() AND l.before_item=to_jsonb(OLD)) THEN
    IF NEW.lifecycle<>1 OR (to_jsonb(NEW)-ARRAY['quantity','last_transaction_id']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['quantity','last_transaction_id'])
      OR NOT EXISTS(SELECT 1 FROM game_market_item_lines l WHERE l.source_item=OLD.item_instance_id AND l.transaction_id=NEW.last_transaction_id
        AND NEW.quantity=OLD.quantity-CASE WHEN l.source_item=l.output_item THEN 0 ELSE l.moved END) THEN
     RAISE EXCEPTION 'Market preserves item state and conserves split units' USING ERRCODE='23514'; END IF;
    RETURN NEW;
   END IF;
$arm$;
  ELSIF name='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_market_item_lines WHERE source_item=NEW.item_instance_id AND transaction_id=NEW.last_transaction_id AND created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_market_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF name='game_item_mint_consistency_guard' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_market_item_lines WHERE output_item=NEW.item_instance_id AND source_item<>output_item AND transaction_id=NEW.minted_transaction_id AND created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_market_complete(NEW.minted_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF name='game_character_depot_remove' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_market_item_lines WHERE source_item=OLD.item_instance_id AND source_item=output_item AND source_location=to_jsonb(OLD) AND created_xact_id=pg_current_xact_id()) THEN
    UPDATE game_character_depot_box_counters SET committed=committed-1 WHERE character_id=OLD.character_id AND town_id=OLD.town_id AND box_number=OLD.box_number;
    RETURN OLD; END IF;
$arm$;
  ELSIF name='game_item_fee_burn_consistency_guard' THEN arm:=$arm$
   IF NEW.cause_kind=6 AND NOT EXISTS(SELECT 1 FROM game_market_settlements WHERE transaction_id=NEW.transaction_id
       AND kind=1 AND fee=NEW.fee_gold_units AND created_xact_id=pg_current_xact_id()) THEN
    RAISE EXCEPTION 'Market fee requires its atomic offer settlement' USING ERRCODE='23514'; END IF;
$arm$;
  ELSE arm:=$arm$
   IF NEW.market_transaction_id IS NOT NULL THEN
    IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS(SELECT 1 FROM game_market_value_lines WHERE entry_id=NEW.entry_id AND transaction_id=NEW.market_transaction_id AND entry=to_jsonb(NEW)-'created_xact_id' AND created_xact_id=pg_current_xact_id())
      OR NOT EXISTS(SELECT 1 FROM game_market_settlements WHERE transaction_id=NEW.market_transaction_id AND world_id=NEW.world_id AND created_xact_id=pg_current_xact_id())
      OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
      OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries WHERE entry_id=NEW.previous_entry_id AND account_id=NEW.account_id AND world_id=NEW.world_id AND balance_after=NEW.balance_before))
      OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND last_entry_id=NEW.entry_id)
        OR EXISTS(SELECT 1 FROM game_account_bank_entries WHERE previous_entry_id=NEW.entry_id)) THEN
     RAISE EXCEPTION 'Market bank entry requires its receipt and chain' USING ERRCODE='23514'; END IF;
    RETURN NULL;
   END IF;
$arm$;
  END IF;
  BEGIN
   EXECUTE overlay(body placing 'BEGIN'||arm from at for 5);
  EXCEPTION WHEN OTHERS THEN RAISE EXCEPTION 'Market proof extension %: %',name,SQLERRM;
  END;
 END LOOP;
 body:=pg_get_functiondef('game_item_location_exclusive(uuid)'::regprocedure);
 IF position('v_locations :=' IN body)=0 THEN RAISE EXCEPTION 'unexpected location proof'; END IF;
 body:=replace(body,'v_locations :=','v_locations := (SELECT count(*) FROM game_item_market_escrow_locations WHERE item_instance_id=p_item) +');
 EXECUTE body;
END $migration$;
CREATE FUNCTION game_market_escrow_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE item UUID;
BEGIN
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'immutable Market custody' USING ERRCODE='23514'; END IF;
 IF TG_OP='DELETE' THEN item:=OLD.item_instance_id; ELSE item:=NEW.item_instance_id; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_market_item_lines WHERE created_xact_id=pg_current_xact_id()
    AND ((TG_OP='INSERT' AND output_item=item) OR (TG_OP='DELETE' AND source_item=item AND source_item=output_item AND source_location=to_jsonb(OLD)))) THEN
  RAISE EXCEPTION 'Market custody requires current transfer evidence' USING ERRCODE='23514'; END IF;
 RETURN COALESCE(NEW,OLD);
END $$;
CREATE TRIGGER market_escrow_guard BEFORE INSERT OR UPDATE OR DELETE ON game_item_market_escrow_locations FOR EACH ROW EXECUTE FUNCTION game_market_escrow_guard();
CREATE FUNCTION game_market_escrow_exclusive() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_item_location_exclusive(COALESCE(NEW.item_instance_id,OLD.item_instance_id)); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER market_escrow_exclusive AFTER INSERT OR DELETE ON game_item_market_escrow_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_market_escrow_exclusive();

-- Private helper, never executable by runtime. Every balance change uses the
-- existing immutable Account Bank ledger and its unchanged balance proof.
CREATE FUNCTION game_market_bank(t UUID,a UUID,w UUID,c UUID,other UUID,k TEXT,delta BIGINT) RETURNS void LANGUAGE plpgsql AS $$
DECLARE b game_account_bank_balances%ROWTYPE; entry UUID;
BEGIN
 IF delta=0 THEN RETURN; END IF;
 SELECT * INTO STRICT b FROM game_account_bank_balances WHERE account_id=a AND world_id=w FOR UPDATE;
 entry:=game_market_identity(t,a::text||k);
 INSERT INTO game_account_bank_entries(entry_id,market_transaction_id,market_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id,counterparty_character_id)
 VALUES(entry,t,k,a,w,NULL,abs(delta),b.balance,b.balance+delta,b.last_entry_id,c,other);
 INSERT INTO game_market_value_lines SELECT t,entry_id,to_jsonb(e)-'created_xact_id',pg_current_xact_id() FROM game_account_bank_entries e WHERE entry_id=entry;
 UPDATE game_account_bank_balances SET balance=balance+delta,last_entry_id=entry WHERE account_id=a AND world_id=w;
END $$;

CREATE FUNCTION game_market_write(w UUID,c UUID,s UUID,g NUMERIC,lease NUMERIC,cmd NUMERIC,t UUID,k SMALLINT,o UUID,
 key TEXT,rev TEXT,side SMALLINT,units INTEGER,price BIGINT,anonymous BOOLEAN,town INTEGER,binding BYTEA,fee_cause BYTEA)
RETURNS BYTEA LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a UUID; maker game_market_offers%ROWTYPE; now_us BIGINT; v_occurrence BYTEA; previous game_market_operations%ROWTYPE;
 fee BIGINT:=0; total BIGINT:=0; gold BIGINT; maker_gold BIGINT; escrow_change BIGINT:=0;
 source_character UUID; destination_character UUID; destination_offer UUID; item RECORD;
 selected JSONB:='[]'; entry JSONB; n INTEGER; left_units INTEGER; output UUID; ordinal INTEGER:=0;
 result BYTEA; buyer UUID; seller UUID; held BIGINT:=0; v_state SMALLINT; reservations BIGINT;
BEGIN
 IF k NOT IN(1,2,3,4) OR octet_length(binding)<>32 OR NOT game_character_is_uuid_v7(t) OR NOT game_character_is_uuid_v7(o) THEN
  RAISE EXCEPTION 'invalid Market request' USING ERRCODE='22023'; END IF;
 now_us:=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint;
 IF k=4 THEN
  IF c IS NOT NULL OR s IS NOT NULL THEN RAISE EXCEPTION 'expiry has no session actor' USING ERRCODE='22023'; END IF;
  v_occurrence:=decode('0102','hex')||uuid_send(o)||decode('01','hex')||int8send(1);
 ELSE
  IF cmd NOT BETWEEN 1 AND 18446744073709551615 THEN RAISE EXCEPTION 'invalid command' USING ERRCODE='22023'; END IF;
  -- encode numeric u64 without signed overflow.
  v_occurrence:=decode('0101','hex')||uuid_send(s)||decode(lpad(to_hex(floor(cmd/4294967296)::bigint),8,'0')||lpad(to_hex(mod(cmd,4294967296)::bigint),8,'0'),'hex');
 END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('market:'||encode(v_occurrence,'hex'),0));
 SELECT * INTO previous FROM game_market_operations WHERE game_market_operations.occurrence=v_occurrence;
 IF FOUND THEN
  IF previous.world_id<>w OR previous.actor IS DISTINCT FROM c OR previous.kind<>k OR previous.request_binding<>binding THEN
   RAISE EXCEPTION 'conflicting Market command' USING ERRCODE='23514'; END IF;
  RETURN previous.outcome;
 END IF;
 IF k<>4 THEN
  SELECT r.account_id INTO a FROM game_durability_reconnect_sessions r
   JOIN game_character_roots root ON root.character_id=r.character_id AND root.world_id=r.world_id AND root.account_id=r.account_id AND root.lifecycle=1
   JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id AND cg.account_id=r.account_id
   JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
   WHERE r.game_session_id=s AND r.character_id=c AND r.world_id=w AND r.current_generation=g
    AND r.character_lease_generation=lease AND r.session_state IN(1,2)
    AND cg.eligible AND cg.lease_generation=lease AND cg.holder_game_session_id=s
    AND ag.presence_character_id=c AND ag.holder_game_session_id=s FOR SHARE OF r;
  IF NOT FOUND THEN RAISE EXCEPTION 'stale Market session generation' USING ERRCODE='23514'; END IF;
  PERFORM 1 FROM game_character_roots WHERE character_id=c FOR NO KEY UPDATE;
 END IF;
 -- Refusals roll back this inner block, including lazily created books/balances.
 BEGIN
  IF k=1 THEN
   IF side NOT IN(1,2) OR units NOT BETWEEN 1 AND 64000 OR price NOT BETWEEN 1 AND 999999999999
     OR price::numeric*units>999999999999 OR octet_length(key) NOT BETWEEN 1 AND 512 OR octet_length(rev) NOT BETWEEN 1 AND 512 THEN
    RAISE EXCEPTION USING ERRCODE='OM009'; END IF;
   IF (SELECT count(*) FROM (SELECT 1 FROM game_market_offers WHERE character_id=c AND game_market_offers.state=1 LIMIT 100) offers)>=100 THEN RAISE EXCEPTION USING ERRCODE='OM005'; END IF;
   INSERT INTO game_market_books VALUES(w,key) ON CONFLICT DO NOTHING;
   PERFORM 1 FROM game_market_books WHERE world_id=w AND definition_key=key FOR UPDATE;
   total:=units*price; fee:=least(1000000,greatest(20,total/50));
   source_character:=CASE WHEN side=2 THEN c END;
   destination_offer:=CASE WHEN side=2 THEN o END;
   buyer:=CASE WHEN side=1 THEN c END; seller:=CASE WHEN side=2 THEN c END;
   escrow_change:=CASE WHEN side=1 THEN total ELSE 0 END;
  ELSE
   SELECT * INTO maker FROM game_market_offers WHERE offer_id=o AND world_id=w;
   IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE='OM006'; END IF;
   PERFORM 1 FROM game_market_books WHERE world_id=w AND definition_key=maker.definition_key FOR UPDATE;
   SELECT * INTO STRICT maker FROM game_market_offers WHERE offer_id=o FOR UPDATE;
   IF maker.state<>1 OR (k<>4 AND maker.expires_at_us<=now_us) OR (k=4 AND maker.expires_at_us>now_us) THEN RAISE EXCEPTION USING ERRCODE='OM006'; END IF;
   IF k=2 AND (key IS DISTINCT FROM maker.definition_key OR rev IS DISTINCT FROM maker.definition_revision) THEN RAISE EXCEPTION USING ERRCODE='OM009'; END IF;
   IF k IN(3,4) THEN
    IF k=3 AND maker.character_id<>c THEN RAISE EXCEPTION USING ERRCODE='OM009'; END IF;
    units:=maker.remaining; total:=maker.escrow_gold; a:=maker.account_id;
    destination_character:=CASE WHEN maker.side=2 THEN maker.character_id END;
    escrow_change:=-total;
   ELSE
    IF maker.account_id=a THEN RAISE EXCEPTION USING ERRCODE='OM007'; END IF;
    IF EXISTS(SELECT 1 FROM game_market_offers WHERE world_id=w AND definition_key=maker.definition_key AND game_market_offers.state=1 AND matching_pending) THEN RAISE EXCEPTION USING ERRCODE='OM011'; END IF;
    IF units NOT BETWEEN 1 AND maker.remaining THEN RAISE EXCEPTION USING ERRCODE='OM006'; END IF;
    total:=units*maker.piece_price;
    IF maker.side=2 THEN buyer:=c; seller:=maker.character_id; destination_character:=c;
    ELSE buyer:=maker.character_id; seller:=c; source_character:=c; destination_character:=buyer; escrow_change:=-total; END IF;
   END IF;
   key:=maker.definition_key; rev:=maker.definition_revision;
  END IF;
  -- Lock recipient counters before Account balance rows, always in Character order.
  FOR item IN SELECT character_id FROM game_character_roots WHERE character_id IN(c,maker.character_id,destination_character) ORDER BY character_id LOOP
   INSERT INTO game_character_inbox_counters(character_id) VALUES(item.character_id) ON CONFLICT DO NOTHING;
   PERFORM 1 FROM game_character_inbox_counters WHERE character_id=item.character_id FOR UPDATE;
  END LOOP;
  -- Select plain native depot/escrow inputs in deterministic order; only the
  -- last touched item splits. The complete selection is bounded by 101 rows.
  IF source_character IS NOT NULL OR (k<>1 AND maker.side=2) THEN
   left_units:=units;
   FOR item IN SELECT i.*,to_jsonb(d) location,d.box_number sort_box,d.ordinal sort_ordinal FROM game_item_instances i
    JOIN game_item_character_depot_locations d USING(item_instance_id,world_id)
    WHERE source_character IS NOT NULL AND d.character_id=source_character AND d.world_id=w AND d.town_id=town
     AND i.definition_production_key=key AND i.definition_revision_ref=rev AND i.lifecycle=1
    UNION ALL SELECT i.*,to_jsonb(e) location,0 sort_box,e.ordinal sort_ordinal FROM game_item_instances i JOIN game_item_market_escrow_locations e USING(item_instance_id,world_id)
    WHERE source_character IS NULL AND e.offer_id=o AND e.world_id=w AND i.lifecycle=1
    ORDER BY sort_box,sort_ordinal,item_instance_id LIMIT 101 LOOP
    EXIT WHEN left_units=0;
    PERFORM 1 FROM game_item_instances WHERE item_instance_id=item.item_instance_id FOR UPDATE;
    IF EXISTS(SELECT 1 FROM game_item_bindings WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=item.item_instance_id AND tier<>0)
      OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=item.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=item.item_instance_id) THEN CONTINUE; END IF;
    IF jsonb_array_length(selected)=100 THEN RAISE EXCEPTION USING ERRCODE='OM008'; END IF;
    n:=least(left_units,item.quantity); left_units:=left_units-n;
    selected:=selected||jsonb_build_object('item',to_jsonb(item)-ARRAY['location','sort_box','sort_ordinal'],'location',item.location,'moved',n);
   END LOOP;
   IF left_units<>0 THEN RAISE EXCEPTION USING ERRCODE='OM003'; END IF;
  END IF;
  IF k=1 OR k=2 THEN
   SELECT market_reserved INTO reservations FROM game_character_inbox_counters WHERE character_id=coalesce(destination_character,c);
   n:=CASE WHEN k=1 AND side=1 THEN units ELSE jsonb_array_length(selected) END;
   IF k=2 AND maker.side=1 THEN reservations:=reservations-units; END IF;
   IF coalesce((SELECT committed FROM game_character_inbox_counters WHERE character_id=coalesce(destination_character,c)),0)+reservations+n>100000 THEN RAISE EXCEPTION USING ERRCODE='OM004'; END IF;
  END IF;
  FOR item IN SELECT account_id FROM game_character_roots WHERE character_id IN(c,maker.character_id) GROUP BY account_id ORDER BY account_id LOOP
   INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(item.account_id,w,0) ON CONFLICT DO NOTHING;
   PERFORM 1 FROM game_account_bank_balances WHERE account_id=item.account_id AND world_id=w FOR UPDATE;
  END LOOP;
  SELECT balance INTO gold FROM game_account_bank_balances WHERE account_id=a AND world_id=w;
  IF (k=1 AND gold<fee+escrow_change) OR (k=2 AND maker.side=2 AND gold<total) THEN RAISE EXCEPTION USING ERRCODE='OM002'; END IF;
  IF k=2 THEN
   SELECT balance INTO maker_gold FROM game_account_bank_balances WHERE account_id=maker.account_id AND world_id=w;
   IF maker.side=1 AND gold+total>9000000000000000 THEN RAISE EXCEPTION USING ERRCODE='OM010'; END IF;
   IF maker.side=2 AND maker_gold+total>9000000000000000 THEN held:=total; END IF;
  END IF;
  IF k IN(3,4) AND gold+total>9000000000000000 THEN held:=total; END IF;
  IF k=1 THEN
   INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,anonymous,created_at_us,expires_at_us,state,escrow_gold,held_credit_gold)
   VALUES(o,w,key,rev,side,a,c,price,units,units,anonymous,now_us,now_us+2592000000000,1,escrow_change,0);
   UPDATE game_market_offers SET matching_pending=EXISTS(SELECT 1 FROM game_market_offers x WHERE x.world_id=w AND x.definition_key=key AND x.definition_revision=rev
      AND x.side<>game_market_write.side AND x.account_id<>a AND x.state=1 AND x.expires_at_us>now_us
      AND CASE WHEN game_market_write.side=1 THEN x.piece_price<=price ELSE x.piece_price>=price END),inbox_reservation=CASE WHEN game_market_write.side=1 THEN units ELSE jsonb_array_length(selected) END WHERE offer_id=o;
  END IF;
  IF k=1 AND fee_cause IS DISTINCT FROM decode('32240a10','hex')||uuid_send(o)||decode('1210','hex')||uuid_send(t) THEN
   RAISE EXCEPTION 'Market fee requires its typed offer cause' USING ERRCODE='23514'; END IF;
  INSERT INTO game_market_settlements VALUES(t,o,w,c,a,k,fee,fee_cause,escrow_change,held,
    coalesce((SELECT sum((value->>'moved')::int) FROM jsonb_array_elements(selected)),0),now_us,pg_current_xact_id());
  FOR entry IN SELECT value FROM jsonb_array_elements(selected) LOOP
   ordinal:=ordinal+1; n:=(entry->>'moved')::int;
   output:=(entry->'item'->>'item_instance_id')::uuid;
   IF n<(entry->'item'->>'quantity')::bigint THEN output:=game_market_identity(t,output::text); END IF;
   INSERT INTO game_market_item_lines VALUES(t,(entry->'item'->>'item_instance_id')::uuid,output,entry->'item',entry->'location',n,destination_character,destination_offer,pg_current_xact_id());
   IF output=(entry->'item'->>'item_instance_id')::uuid THEN
    IF entry->'location' ? 'offer_id' THEN DELETE FROM game_item_market_escrow_locations WHERE item_instance_id=output;
    ELSE DELETE FROM game_item_character_depot_locations WHERE item_instance_id=output; END IF;
   ELSE
    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
    VALUES(output,w,'Item',key,rev,n,1,t);
   END IF;
   UPDATE game_item_instances SET quantity=quantity-CASE WHEN item_instance_id=output THEN 0 ELSE n END,last_transaction_id=t WHERE item_instance_id=(entry->'item'->>'item_instance_id')::uuid;
   IF destination_offer IS NOT NULL THEN INSERT INTO game_item_market_escrow_locations VALUES(output,w,destination_offer,ordinal);
   ELSE PERFORM game_character_inbox_deliver(output,destination_character,w,'MARKET',t::text); END IF;
  END LOOP;
  IF k=1 THEN
   PERFORM game_market_bank(t,a,w,c,NULL,'FEE_DEBIT',-fee);
   PERFORM game_market_bank(t,a,w,c,NULL,'MARKET_ESCROW',-escrow_change);
  ELSIF k=2 THEN
   IF maker.side=2 THEN
    PERFORM game_market_bank(t,a,w,c,seller,'MARKET_PURCHASE',-total);
    IF held=0 THEN PERFORM game_market_bank(t,maker.account_id,w,seller,buyer,'MARKET_SALE',total); END IF;
   ELSE PERFORM game_market_bank(t,a,w,seller,buyer,'MARKET_SALE',total); END IF;
   INSERT INTO game_market_history VALUES(t,buyer,o,units,maker.piece_price,now_us),(t,seller,o,units,maker.piece_price,now_us);
  ELSE
   IF held=0 THEN PERFORM game_market_bank(t,a,w,maker.character_id,NULL,'MARKET_ESCROW_RETURN',total); END IF;
   INSERT INTO game_market_history VALUES(t,maker.character_id,o,units,maker.piece_price,now_us);
  END IF;
  IF k<>1 THEN
   v_state:=CASE WHEN k=2 AND maker.remaining>units AND held=0 THEN 1 WHEN k=2 AND maker.remaining=units THEN 2 WHEN k=2 THEN 3 WHEN k=3 THEN 3 ELSE 4 END;
   -- Held sale removes the offer from the book; remaining items remain explicit
   -- escrow custody for the return job, rather than becoming spendable twice.
   UPDATE game_market_offers SET remaining=CASE WHEN k=2 THEN remaining-units ELSE remaining END,
    state=CASE WHEN held>0 THEN 5 ELSE v_state END,
    matching_pending=false,
    ended_as=CASE WHEN held>0 THEN v_state END,
    escrow_gold=escrow_gold+escrow_change,held_credit_gold=held,
    inbox_reservation=CASE WHEN maker.side=1 AND k=2 THEN maker.remaining-units WHEN maker.side=2 AND k=2 THEN (SELECT count(*) FROM game_item_market_escrow_locations WHERE offer_id=o) ELSE 0 END,
    ended_at_us=CASE WHEN v_state<>1 THEN now_us END,
    end_sequence=CASE WHEN v_state<>1 THEN nextval('game_market_end_sequence') END
   WHERE offer_id=o;
  END IF;
  INSERT INTO game_market_event_outbox SELECT t,to_jsonb(r)-'created_xact_id',sha256(convert_to((to_jsonb(r)-'created_xact_id')::text,'UTF8')),now_us,now_us+2592000000000 FROM game_market_settlements r WHERE transaction_id=t;
  result:=decode('01','hex')||uuid_send(o);
 EXCEPTION WHEN SQLSTATE 'OM002' THEN result:=decode('02','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM003' THEN result:=decode('03','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM004' THEN result:=decode('04','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM005' THEN result:=decode('05','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM006' THEN result:=decode('06','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM007' THEN result:=decode('07','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM008' THEN result:=decode('08','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM009' THEN result:=decode('09','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM010' THEN result:=decode('0a','hex')||uuid_send(o);
 WHEN SQLSTATE 'OM011' THEN result:=decode('0f','hex')||uuid_send(o);
 END;
 INSERT INTO game_market_operations VALUES(w,v_occurrence,t,k,c,binding,result);
 RETURN result;
END $$;

-- Restart-safe held credit and remaining sell escrow delivery. One occurrence per
-- offer and UTC day; no empty event is emitted while the balance cannot receive.
CREATE FUNCTION game_market_release(w UUID,o UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_market_offers%ROWTYPE; now_us BIGINT; day BIGINT; t UUID; v_occurrence BYTEA;
 b BIGINT; credit BIGINT; moved INTEGER:=0; item RECORD; payload BYTEA;
BEGIN
 now_us:=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint;
 day:=now_us/86400000000;
 v_occurrence:=decode('0102','hex')||uuid_send(o)||decode('03','hex')||int8send(day);
 t:=game_market_identity(o,'held:'||day::text);
 PERFORM pg_advisory_xact_lock(hashtextextended('market:'||encode(v_occurrence,'hex'),0));
 IF EXISTS(SELECT 1 FROM game_market_operations WHERE game_market_operations.occurrence=v_occurrence) THEN RETURN; END IF;
 SELECT * INTO r FROM game_market_offers WHERE offer_id=o AND world_id=w;
 IF NOT FOUND OR r.state<>5 THEN RETURN; END IF;
 PERFORM 1 FROM game_market_books WHERE world_id=w AND definition_key=r.definition_key FOR UPDATE;
 SELECT * INTO STRICT r FROM game_market_offers WHERE offer_id=o FOR UPDATE;
 IF r.state<>5 THEN RETURN; END IF;
 PERFORM 1 FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_market_escrow_locations WHERE offer_id=o LIMIT 100) ORDER BY item_instance_id FOR UPDATE;
 INSERT INTO game_character_inbox_counters(character_id) VALUES(r.character_id) ON CONFLICT DO NOTHING;
 PERFORM 1 FROM game_character_inbox_counters WHERE character_id=r.character_id FOR UPDATE;
 PERFORM 1 FROM game_account_bank_balances WHERE account_id=r.account_id AND world_id=w FOR UPDATE;
 SELECT balance INTO STRICT b FROM game_account_bank_balances WHERE account_id=r.account_id AND world_id=w;
 credit:=CASE WHEN b+r.held_credit_gold<=9000000000000000 THEN r.held_credit_gold ELSE 0 END;
 IF credit=0 AND NOT EXISTS(SELECT 1 FROM game_item_market_escrow_locations WHERE offer_id=o) THEN RETURN; END IF;
 SELECT coalesce(sum(i.quantity),0) INTO moved FROM game_item_market_escrow_locations e JOIN game_item_instances i USING(item_instance_id) WHERE e.offer_id=o;
 INSERT INTO game_market_settlements VALUES(t,o,w,NULL,r.account_id,6,0,NULL,0,-credit,moved,now_us,pg_current_xact_id());
 FOR item IN SELECT i.*,to_jsonb(e) location FROM game_item_market_escrow_locations e JOIN game_item_instances i USING(item_instance_id) WHERE e.offer_id=o ORDER BY i.item_instance_id LIMIT 100 LOOP
  INSERT INTO game_market_item_lines VALUES(t,item.item_instance_id,item.item_instance_id,to_jsonb(item)-'location',item.location,item.quantity,r.character_id,NULL,pg_current_xact_id());
  DELETE FROM game_item_market_escrow_locations WHERE item_instance_id=item.item_instance_id;
  UPDATE game_item_instances SET last_transaction_id=t WHERE item_instance_id=item.item_instance_id;
  PERFORM game_character_inbox_deliver(item.item_instance_id,r.character_id,w,'MARKET',t::text);
 END LOOP;
 IF credit>0 THEN PERFORM game_market_bank(t,r.account_id,w,r.character_id,NULL,'MARKET_HELD_CREDIT',credit); END IF;
 UPDATE game_market_offers SET held_credit_gold=held_credit_gold-credit,inbox_reservation=0,
  state=CASE WHEN credit>0 THEN ended_as ELSE state END,ended_as=CASE WHEN credit>0 THEN NULL ELSE ended_as END WHERE offer_id=o;
 INSERT INTO game_market_event_outbox SELECT t,to_jsonb(s)-'created_xact_id',sha256(convert_to((to_jsonb(s)-'created_xact_id')::text,'UTF8')),now_us,now_us+2592000000000 FROM game_market_settlements s WHERE transaction_id=t;
 payload:=decode('01','hex')||uuid_send(o);
 INSERT INTO game_market_operations VALUES(w,v_occurrence,t,6,NULL,sha256(uuid_send(o)),payload);
END $$;

-- One asynchronous crossing step, serialized by the same ware book. Both
-- sides retain native escrow; settlement uses the older offer's price.
CREATE FUNCTION game_market_match(w UUID,o UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE pending game_market_offers%ROWTYPE; counter game_market_offers%ROWTYPE;
 buy game_market_offers%ROWTYPE; sell game_market_offers%ROWTYPE; i RECORD; e JSONB;
 selected JSONB:='[]'; units INTEGER:=0; maximum INTEGER; n INTEGER; output UUID;
 now_us BIGINT; price BIGINT; debit BIGINT; sale BIGINT; refund BIGINT; fold BIGINT:=0;
 seller_held BIGINT:=0; buyer_held BIGINT:=0; t UUID; v_occurrence BYTEA; b BIGINT;
BEGIN
 SELECT * INTO pending FROM game_market_offers WHERE offer_id=o AND world_id=w;
 IF NOT FOUND OR pending.state<>1 OR NOT pending.matching_pending THEN RETURN; END IF;
 PERFORM 1 FROM game_market_books WHERE world_id=w AND definition_key=pending.definition_key FOR UPDATE;
 SELECT * INTO STRICT pending FROM game_market_offers WHERE offer_id=o;
 now_us:=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint;
 IF pending.state<>1 OR NOT pending.matching_pending OR pending.expires_at_us<=now_us THEN RETURN; END IF;
 SELECT * INTO counter FROM game_market_offers x WHERE x.world_id=w AND x.definition_key=pending.definition_key
   AND x.definition_revision=pending.definition_revision AND x.side<>pending.side AND x.state=1
   AND x.account_id<>pending.account_id AND x.expires_at_us>now_us
   AND CASE WHEN pending.side=1 THEN x.piece_price<=pending.piece_price ELSE x.piece_price>=pending.piece_price END
   ORDER BY CASE WHEN pending.side=1 THEN x.piece_price END ASC,CASE WHEN pending.side=2 THEN x.piece_price END DESC,x.created_at_us,x.offer_id LIMIT 1;
 IF NOT FOUND THEN UPDATE game_market_offers SET matching_pending=false WHERE offer_id=o; RETURN; END IF;
 PERFORM 1 FROM game_market_offers WHERE offer_id IN(o,counter.offer_id) ORDER BY offer_id FOR UPDATE;
 IF pending.side=1 THEN buy:=pending; sell:=counter; ELSE buy:=counter; sell:=pending; END IF;
 maximum:=least(buy.remaining,sell.remaining);
 price:=CASE WHEN (buy.created_at_us,buy.offer_id)<(sell.created_at_us,sell.offer_id) THEN buy.piece_price ELSE sell.piece_price END;
 FOR i IN SELECT item.*,to_jsonb(location) source_location FROM game_item_market_escrow_locations location JOIN game_item_instances item USING(item_instance_id)
   WHERE location.offer_id=sell.offer_id ORDER BY item.item_instance_id LIMIT 100 LOOP
  EXIT WHEN units=maximum;
  PERFORM 1 FROM game_item_instances WHERE item_instance_id=i.item_instance_id FOR UPDATE;
  n:=least(maximum-units,i.quantity); units:=units+n;
  selected:=selected||jsonb_build_object('item',to_jsonb(i)-'source_location','location',i.source_location,'moved',n);
 END LOOP;
 IF units=0 THEN RAISE EXCEPTION 'Market sell offer lost its escrow' USING ERRCODE='23514'; END IF;
 t:=game_market_identity(o,'match:'||pending.remaining::text);
 v_occurrence:=decode('0102','hex')||uuid_send(o)||decode('02','hex')||int8send(pending.remaining);
 IF EXISTS(SELECT 1 FROM game_market_operations WHERE game_market_operations.occurrence=v_occurrence) THEN RETURN; END IF;
 PERFORM 1 FROM game_character_inbox_counters WHERE character_id IN(buy.character_id,sell.character_id) ORDER BY character_id FOR UPDATE;
 PERFORM 1 FROM game_account_bank_balances WHERE world_id=w AND account_id IN(buy.account_id,sell.account_id) ORDER BY account_id FOR UPDATE;
 debit:=units*buy.piece_price; sale:=units*price; refund:=debit-sale;
 SELECT balance INTO STRICT b FROM game_account_bank_balances WHERE account_id=sell.account_id AND world_id=w;
 IF b+sale>9000000000000000 THEN seller_held:=sale; END IF;
 SELECT balance INTO STRICT b FROM game_account_bank_balances WHERE account_id=buy.account_id AND world_id=w;
 IF b+refund>9000000000000000 THEN
  fold:=(buy.remaining-units)*buy.piece_price; buyer_held:=refund+fold;
 END IF;
 INSERT INTO game_market_settlements VALUES(t,o,w,NULL,pending.account_id,5,0,NULL,-debit-fold,seller_held+buyer_held,units,now_us,pg_current_xact_id());
 FOR e IN SELECT value FROM jsonb_array_elements(selected) LOOP
  n:=(e->>'moved')::integer; output:=(e->'item'->>'item_instance_id')::uuid;
  IF n<(e->'item'->>'quantity')::bigint THEN output:=game_market_identity(t,output::text); END IF;
  INSERT INTO game_market_item_lines VALUES(t,(e->'item'->>'item_instance_id')::uuid,output,e->'item',e->'location',n,buy.character_id,NULL,pg_current_xact_id());
  IF output=(e->'item'->>'item_instance_id')::uuid THEN DELETE FROM game_item_market_escrow_locations WHERE item_instance_id=output;
  ELSE INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
    VALUES(output,w,'Item',sell.definition_key,sell.definition_revision,n,1,t); END IF;
  UPDATE game_item_instances SET quantity=quantity-CASE WHEN item_instance_id=output THEN 0 ELSE n END,last_transaction_id=t WHERE item_instance_id=(e->'item'->>'item_instance_id')::uuid;
  PERFORM game_character_inbox_deliver(output,buy.character_id,w,'MARKET',t::text);
 END LOOP;
 IF seller_held=0 THEN PERFORM game_market_bank(t,sell.account_id,w,sell.character_id,buy.character_id,'MARKET_SALE',sale); END IF;
 IF buyer_held=0 THEN PERFORM game_market_bank(t,buy.account_id,w,buy.character_id,sell.character_id,'MARKET_ESCROW_RETURN',refund); END IF;
 UPDATE game_market_offers SET remaining=remaining-units,
   escrow_gold=CASE WHEN buyer_held>0 THEN 0 ELSE escrow_gold-debit END,held_credit_gold=buyer_held,
   inbox_reservation=CASE WHEN buyer_held>0 THEN 0 ELSE remaining-units END,
   state=CASE WHEN buyer_held>0 THEN 5 WHEN remaining=units THEN 2 ELSE 1 END,
   ended_as=CASE WHEN buyer_held>0 THEN CASE WHEN remaining=units THEN 2 ELSE 3 END END,
   ended_at_us=CASE WHEN buyer_held>0 OR remaining=units THEN now_us END,
   end_sequence=CASE WHEN buyer_held>0 OR remaining=units THEN nextval('game_market_end_sequence') END,
   matching_pending=matching_pending AND remaining>units AND buyer_held=0
   WHERE offer_id=buy.offer_id;
 UPDATE game_market_offers SET remaining=remaining-units,held_credit_gold=seller_held,
   inbox_reservation=(SELECT count(*) FROM game_item_market_escrow_locations WHERE offer_id=sell.offer_id),
   state=CASE WHEN seller_held>0 THEN 5 WHEN remaining=units THEN 2 ELSE 1 END,
   ended_as=CASE WHEN seller_held>0 THEN CASE WHEN remaining=units THEN 2 ELSE 3 END END,
   ended_at_us=CASE WHEN seller_held>0 OR remaining=units THEN now_us END,
   end_sequence=CASE WHEN seller_held>0 OR remaining=units THEN nextval('game_market_end_sequence') END,
   matching_pending=matching_pending AND remaining>units AND seller_held=0
   WHERE offer_id=sell.offer_id;
 INSERT INTO game_market_history VALUES(t,buy.character_id,buy.offer_id,units,price,now_us),(t,sell.character_id,sell.offer_id,units,price,now_us);
 INSERT INTO game_market_event_outbox SELECT t,to_jsonb(s)-'created_xact_id',sha256(convert_to((to_jsonb(s)-'created_xact_id')::text,'UTF8')),now_us,now_us+2592000000000 FROM game_market_settlements s WHERE transaction_id=t;
 INSERT INTO game_market_operations VALUES(w,v_occurrence,t,5,NULL,sha256(uuid_send(buy.offer_id)||uuid_send(sell.offer_id)),decode('01','hex')||uuid_send(o));
END $$;

-- Every helper is schema-bound and private. The command seam validates the
-- live session; job seams are invoked only by the recovery/node-fenced owner.
DO $$ DECLARE f RECORD; tab TEXT; BEGIN
 FOR f IN SELECT p.oid::regprocedure signature FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
  WHERE n.nspname=current_schema() AND p.proname LIKE 'game_market_%' LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f.signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.signature);
 END LOOP;
 FOREACH tab IN ARRAY ARRAY['game_market_settlements','game_market_item_lines','game_market_value_lines','game_market_history','game_market_event_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER market_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_market_operation_immutable()',tab);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',tab);
  EXECUTE format('GRANT SELECT ON %I TO oteryn_game_runtime,oteryn_game_control',tab);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_market_write(UUID,UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,SMALLINT,UUID,TEXT,TEXT,SMALLINT,INTEGER,BIGINT,BOOLEAN,INTEGER,BYTEA,BYTEA) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_market_release(UUID,UUID) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_market_complete(UUID) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_market_match(UUID,UUID) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_market_identity(UUID,TEXT) TO oteryn_game_runtime;
