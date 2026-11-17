-- T135: Canary 99902524 game.cpp:13124-13339; player.cpp:12947-13017.
-- Actor intents use a live session fence; final handover uses the independently
-- current World disposition authority. These are Game bank gold, never coins.
CREATE TABLE game_house_sale_offers (
 offer_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(offer_id)),
 world_id UUID NOT NULL,house_key TEXT NOT NULL,
 seller UUID NOT NULL REFERENCES game_character_roots,
 seller_account UUID NOT NULL REFERENCES game_character_account_guards,
 recipient UUID NOT NULL REFERENCES game_character_roots,
 recipient_account UUID NOT NULL REFERENCES game_character_account_guards,
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision>0),
 price BIGINT NOT NULL CHECK(price BETWEEN 0 AND 999999999999),
 rent BIGINT NOT NULL CHECK(rent BETWEEN 0 AND 999999999999 AND price+rent<=999999999999),
 due_at BIGINT NOT NULL,created_at BIGINT NOT NULL,
 state SMALLINT NOT NULL CHECK(state BETWEEN 1 AND 5),
 disposition UUID UNIQUE REFERENCES game_house_dispositions,
 last_event UUID NOT NULL,
 CHECK(seller<>recipient),CHECK(due_at-created_at BETWEEN 86400000 AND 2592000000),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties
);
CREATE UNIQUE INDEX game_house_sale_active_house ON game_house_sale_offers(world_id,house_key) WHERE state IN(1,2);
CREATE UNIQUE INDEX game_house_sale_accepted_slot ON game_house_sale_offers(recipient_account,world_id) WHERE state=2;
CREATE TABLE game_house_sale_events (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 offer_id UUID NOT NULL REFERENCES game_house_sale_offers DEFERRABLE INITIALLY DEFERRED,
 action SMALLINT NOT NULL CHECK(action BETWEEN 3 AND 7),
 before_offer JSONB,after_offer JSONB NOT NULL,
 property_before JSONB NOT NULL,property_after JSONB NOT NULL,
 game_session_id UUID,command_id NUMERIC(20,0),
 actor UUID REFERENCES game_character_roots,
 node_id UUID NOT NULL,node_revision NUMERIC(20,0) NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 occurred_at BIGINT NOT NULL DEFAULT (floor(extract(epoch FROM clock_timestamp())*1000)::bigint),
 CHECK((action=7)=(game_session_id IS NULL)),
 CHECK((game_session_id IS NULL)=(command_id IS NULL)),
 CHECK((action=7)=(actor IS NULL))
);
ALTER TABLE game_house_sale_offers ADD FOREIGN KEY(last_event) REFERENCES game_house_sale_events DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE game_house_sale_commands (
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 binding BYTEA NOT NULL CHECK(octet_length(binding)<=4096),outcome JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 operation_id UUID REFERENCES game_house_dispositions,
 PRIMARY KEY(game_session_id,command_id)
);
CREATE TABLE game_house_sale_holds (
 offer_id UUID PRIMARY KEY REFERENCES game_house_sale_offers,
 world_id UUID NOT NULL,recipient_account UUID NOT NULL,seller_account UUID NOT NULL,
 refund BIGINT NOT NULL CHECK(refund BETWEEN 0 AND 999999999999),
 incoming BIGINT NOT NULL CHECK(incoming BETWEEN 0 AND refund)
);
REVOKE ALL ON game_house_sale_offers,game_house_sale_events,game_house_sale_commands,game_house_sale_holds
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE TABLE game_house_sale_audit_outbox (
 event_id UUID PRIMARY KEY REFERENCES game_house_sale_events,
 event_type_id BIGINT NOT NULL DEFAULT 8 CHECK(event_type_id=8),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 16384),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+2592000000),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 publication_state SMALLINT NOT NULL DEFAULT 1 CHECK(publication_state IN(1,2)),
 published_at BIGINT CHECK(published_at IS NULL OR published_at>=occurred_at),
 written_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((publication_state=2)=(published_at IS NOT NULL))
);
CREATE TRIGGER audit_guard BEFORE UPDATE OR DELETE ON game_house_sale_audit_outbox FOR EACH ROW EXECUTE FUNCTION game_account_bank_audit_guard();
REVOKE ALL ON game_house_sale_audit_outbox FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,DELETE ON game_house_sale_audit_outbox TO oteryn_game_runtime;
GRANT UPDATE(publication_state,published_at) ON game_house_sale_audit_outbox TO oteryn_game_runtime;

CREATE FUNCTION game_house_sale_command_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.operation_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_house_disposition_audit_outbox
  WHERE operation_id=NEW.operation_id AND written_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'house sale notice or cancellation lacks its physical disposition event' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_sale_command_proven AFTER INSERT ON game_house_sale_commands
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_command_proven();
CREATE FUNCTION game_house_sale_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'house sale evidence is immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON game_house_sale_events FOR EACH ROW EXECUTE FUNCTION game_house_sale_immutable();
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON game_house_sale_commands FOR EACH ROW EXECUTE FUNCTION game_house_sale_immutable();
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_house_sale_events','game_house_sale_commands','game_house_sale_offers','game_house_sale_holds'] LOOP
  EXECUTE format('CREATE TRIGGER no_truncate BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate()',t);
 END LOOP;
END $$;

ALTER TABLE game_account_bank_entries ADD COLUMN house_sale_event UUID REFERENCES game_house_sale_events DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN house_sale_kind TEXT CHECK(house_sale_kind IN('RESERVE','REFUND','PAYMENT')),
 ADD CONSTRAINT game_account_bank_entries_sale_reference CHECK((house_sale_event IS NULL)=(house_sale_kind IS NULL));
CREATE UNIQUE INDEX game_house_sale_bank_event_kind ON game_account_bank_entries(house_sale_event,house_sale_kind) WHERE house_sale_event IS NOT NULL;
DO $$ DECLARE body TEXT;n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_account_bank_entries_one_reference','game_account_bank_entries_guild_reference','game_account_bank_entries_house_value'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT body FROM pg_constraint WHERE conrelid='game_account_bank_entries'::regclass AND conname=n;
  EXECUTE format('ALTER TABLE game_account_bank_entries DROP CONSTRAINT %I',n);
  IF n='game_account_bank_entries_one_reference' THEN
   IF strpos(body,'num_nonnulls(')=0 THEN RAISE EXCEPTION 'missing house sale bank reference seam'; END IF;
   body:=replace(body,'num_nonnulls(','num_nonnulls(house_sale_event,');
  ELSIF n='game_account_bank_entries_guild_reference' THEN
   body:='CASE WHEN house_sale_event IS NOT NULL THEN kind IS NULL AND guild_kind IS NULL AND market_kind IS NULL AND house_rent_kind IS NULL AND bazaar_kind IS NULL ELSE ('||body||') END';
  ELSE
   body:='CASE WHEN house_sale_event IS NOT NULL THEN kind IS NULL AND escrow_before IS NULL AND escrow_after IS NULL AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND balance_after=balance_before+CASE house_sale_kind WHEN ''RESERVE'' THEN -amount ELSE amount END ELSE ('||body||') END';
  END IF;
  EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT %I CHECK(%s)',n,body);
 END LOOP;
END $$;

CREATE FUNCTION game_house_sale_liability(a UUID,w UUID) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT coalesce(sum(CASE WHEN recipient_account=a THEN refund ELSE incoming END),0)::bigint
 FROM game_house_sale_holds WHERE world_id=w AND (recipient_account=a OR seller_account=a)
$$;
ALTER FUNCTION game_account_bank_credit_headroom(UUID,UUID) RENAME TO game_account_bank_credit_headroom_before_t135;
CREATE FUNCTION game_account_bank_credit_headroom(a UUID,w UUID) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,game_account_bank_credit_headroom_before_t135(a,w)-game_house_sale_liability(a,w))
$$;
CREATE FUNCTION game_house_sale_reserves_slot(a UUID,w UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_house_sale_offers WHERE world_id=w AND recipient_account=a AND state=2)
$$;
CREATE FUNCTION game_house_sale_headroom() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE a UUID;w UUID;
BEGIN
 IF TG_TABLE_NAME='game_account_bank_balances' THEN
  IF game_house_sale_liability(NEW.account_id,NEW.world_id)>game_account_bank_credit_headroom_before_t135(NEW.account_id,NEW.world_id)
  THEN RAISE EXCEPTION 'house sale return or payment headroom is reserved' USING ERRCODE='23514'; END IF;
 ELSE
  FOR a,w IN SELECT recipient_account,world_id FROM game_house_sale_holds WHERE offer_id=coalesce(NEW.offer_id,OLD.offer_id)
   UNION SELECT seller_account,world_id FROM game_house_sale_holds WHERE offer_id=coalesce(NEW.offer_id,OLD.offer_id) LOOP
   IF game_house_sale_liability(a,w)>game_account_bank_credit_headroom_before_t135(a,w)
   THEN RAISE EXCEPTION 'house sale return or payment headroom is reserved' USING ERRCODE='23514'; END IF;
  END LOOP;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_sale_bank_headroom AFTER INSERT OR UPDATE ON game_account_bank_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_headroom();
CREATE CONSTRAINT TRIGGER game_house_sale_hold_headroom AFTER INSERT OR UPDATE OR DELETE ON game_house_sale_holds
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_headroom();
CREATE FUNCTION game_house_sale_slot() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_TABLE_NAME='game_house_bids' THEN
  IF NEW.state=1 AND game_house_sale_reserves_slot(NEW.account_id,NEW.world_id)
  THEN RAISE EXCEPTION 'accepted house transfer reserves the account slot' USING ERRCODE='23514'; END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_house_sale_offers WHERE recipient_account=NEW.account_id AND world_id=NEW.world_id AND state=2 AND house_key<>NEW.house_key)
  THEN RAISE EXCEPTION 'accepted house transfer reserves another account slot' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_sale_bid_slot AFTER INSERT OR UPDATE ON game_house_bids
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_slot();
CREATE CONSTRAINT TRIGGER game_house_sale_owned_slot AFTER INSERT OR UPDATE ON game_account_housing_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_slot();

CREATE FUNCTION game_house_sale_bank_write(e UUID,k TEXT,a UUID,w UUID,c UUID,amount BIGINT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE b game_account_bank_balances;entry UUID;
BEGIN
 IF amount=0 THEN RETURN; END IF;
 INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(a,w,0) ON CONFLICT DO NOTHING;
 SELECT * INTO STRICT b FROM game_account_bank_balances WHERE account_id=a AND world_id=w FOR UPDATE;
 IF (k='RESERVE' AND b.balance<amount) OR (k<>'RESERVE' AND game_account_bank_credit_headroom(a,w)<amount)
 THEN RAISE EXCEPTION 'house sale bank balance or headroom changed' USING ERRCODE='23514'; END IF;
 entry:=game_character_uuid_v7();
 INSERT INTO game_account_bank_entries(entry_id,house_sale_event,house_sale_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
 VALUES(entry,e,k,a,w,NULL,amount,b.balance,b.balance+CASE k WHEN 'RESERVE' THEN -amount ELSE amount END,b.last_entry_id,c);
 UPDATE game_account_bank_balances SET balance=b.balance+CASE k WHEN 'RESERVE' THEN -amount ELSE amount END,last_entry_id=entry WHERE account_id=a AND world_id=w;
END $$;

CREATE FUNCTION game_house_sale_event_proven(id UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE e game_house_sale_events;o game_house_sale_offers;p game_house_properties;old_state INT;
 bank_kind TEXT;bank_account UUID;bank_actor UUID;bank_amount BIGINT:=0;total BIGINT;
BEGIN
 SELECT * INTO e FROM game_house_sale_events WHERE event_id=id;
 SELECT * INTO o FROM game_house_sale_offers WHERE offer_id=e.offer_id;
 SELECT * INTO p FROM game_house_properties WHERE world_id=o.world_id AND house_key=o.house_key;
 old_state:=(e.before_offer->>'state')::int;total:=o.price+o.rent;
 IF NOT EXISTS(SELECT 1 FROM game_house_sale_audit_outbox a WHERE a.event_id=id AND a.occurred_at=e.occurred_at AND a.written_xact_id=pg_current_xact_id())
  OR e.event_id IS NULL OR e.created_xact_id<>pg_current_xact_id() OR o.last_event<>id
  OR to_jsonb(o) IS DISTINCT FROM e.after_offer OR to_jsonb(p) IS DISTINCT FROM e.property_after
  OR (e.before_offer IS NOT NULL AND (e.after_offer-ARRAY['state','last_event','disposition']) IS DISTINCT FROM (e.before_offer-ARRAY['state','last_event','disposition']))
  OR (e.action<>7 AND e.property_after IS DISTINCT FROM e.property_before||jsonb_build_object('revision',(e.property_before->>'revision')::numeric+1))
  OR (e.action<>7 AND NOT EXISTS(SELECT 1 FROM game_house_sale_commands c WHERE c.game_session_id=e.game_session_id
    AND c.command_id=e.command_id AND c.created_xact_id=pg_current_xact_id()
    AND (convert_from(c.binding,'UTF8')::jsonb->>'character')::uuid=e.actor))
 THEN RAISE EXCEPTION 'house sale event lacks its exact physical successor and command' USING ERRCODE='23514'; END IF;
 IF e.action=3 THEN
  IF e.before_offer IS NOT NULL OR o.state<>1 OR o.disposition IS NOT NULL OR e.actor<>o.seller
  THEN RAISE EXCEPTION 'invalid house offer event' USING ERRCODE='23514'; END IF;
 ELSIF e.action=5 THEN
  IF old_state<>1 OR o.state<>2 OR e.actor<>o.recipient OR o.disposition IS NULL
   OR NOT EXISTS(SELECT 1 FROM game_house_dispositions d WHERE d.operation_id=o.disposition AND d.world_id=o.world_id
    AND d.house_key=o.house_key AND d.owner_character_id=o.seller AND d.ownership_revision=o.ownership_revision
    AND d.state=1 AND d.due_at=o.due_at AND d.cause='MOVE_OUT' AND d.owner_guild_id IS NULL)
  THEN RAISE EXCEPTION 'invalid house acceptance event' USING ERRCODE='23514'; END IF;
  bank_kind:='RESERVE';bank_account:=o.recipient_account;bank_actor:=o.recipient;bank_amount:=total;
 ELSIF e.action IN(4,6) THEN
  IF old_state NOT IN(1,2) OR o.state<>(CASE e.action WHEN 4 THEN 3 ELSE 4 END)
   OR e.actor<>(CASE e.action WHEN 4 THEN o.seller ELSE o.recipient END)
   OR (old_state=2 AND NOT EXISTS(SELECT 1 FROM game_house_dispositions d WHERE d.operation_id=o.disposition AND d.state=4))
  THEN RAISE EXCEPTION 'invalid house cancellation or rejection event' USING ERRCODE='23514'; END IF;
  IF old_state=2 THEN bank_kind:='REFUND';bank_account:=o.recipient_account;bank_actor:=o.recipient;bank_amount:=total; END IF;
 ELSE
  IF old_state<>2 OR o.state<>5 OR e.actor IS NOT NULL OR e.game_session_id IS NOT NULL
   OR p.owner_character_id<>o.recipient OR p.owner_account_id<>o.recipient_account
   OR p.ownership_generation<>o.ownership_revision+2 OR p.acquisition_receipt_ref<>id
   OR p.revision<>(e.property_before->>'revision')::numeric+2
   OR NOT EXISTS(SELECT 1 FROM game_house_dispositions d JOIN game_house_disposition_audit_outbox a USING(operation_id)
    WHERE d.operation_id=o.disposition AND d.state=3 AND d.released_at IS NOT NULL
     AND d.written_xact_id=pg_current_xact_id() AND a.written_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_house_disposition_container_sources(o.world_id,o.house_key))
  THEN RAISE EXCEPTION 'house handover requires empty, audited disposition' USING ERRCODE='23514'; END IF;
  bank_kind:='PAYMENT';bank_account:=o.seller_account;bank_actor:=o.seller;bank_amount:=o.price;
 END IF;
 IF (o.state=2) IS DISTINCT FROM EXISTS(SELECT 1 FROM game_house_sale_holds h WHERE h.offer_id=o.offer_id)
  OR (o.state=2 AND NOT EXISTS(SELECT 1 FROM game_house_sale_holds h WHERE h.offer_id=o.offer_id
   AND h.world_id=o.world_id AND h.recipient_account=o.recipient_account AND h.seller_account=o.seller_account
   AND h.refund=total AND h.incoming=o.price))
  OR (SELECT count(*) FROM game_account_bank_entries WHERE house_sale_event=id)<>(bank_amount>0)::int
  OR (bank_amount>0 AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries b WHERE b.house_sale_event=id
    AND b.house_sale_kind=bank_kind AND b.account_id=bank_account AND b.world_id=o.world_id
    AND b.acting_character_id=bank_actor AND b.amount=bank_amount AND b.created_xact_id=pg_current_xact_id()))
 THEN RAISE EXCEPTION 'house sale escrow, refund, payment or rent does not conserve gold' USING ERRCODE='23514'; END IF;
 -- Every new bank family retains the native account chain and actual balance proof.
 IF EXISTS(SELECT 1 FROM game_account_bank_entries b WHERE b.house_sale_event=id AND (
  (b.previous_entry_id IS NULL AND b.balance_before<>0)
  OR (b.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries prev
   WHERE prev.entry_id=b.previous_entry_id AND prev.account_id=b.account_id AND prev.world_id=b.world_id AND prev.balance_after=b.balance_before))
  OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances balance WHERE balance.account_id=b.account_id AND balance.world_id=b.world_id AND balance.last_entry_id=b.entry_id)
   OR EXISTS(SELECT 1 FROM game_account_bank_entries next WHERE next.previous_entry_id=b.entry_id))
  OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=b.acting_character_id AND r.account_id=b.account_id AND r.world_id=b.world_id AND r.lifecycle=1)))
 THEN RAISE EXCEPTION 'house sale bank ledger has no current account chain' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_house_sale_offer_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='DELETE' OR NOT EXISTS(SELECT 1 FROM game_house_sale_events e WHERE e.event_id=NEW.last_event
  AND e.offer_id=NEW.offer_id AND e.created_xact_id=pg_current_xact_id() AND e.after_offer=to_jsonb(NEW)
  AND ((TG_OP='INSERT' AND e.before_offer IS NULL) OR (TG_OP='UPDATE' AND e.before_offer=to_jsonb(OLD))))
 THEN RAISE EXCEPTION 'house offer requires its exact immutable transition' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER transition BEFORE INSERT OR UPDATE OR DELETE ON game_house_sale_offers
 FOR EACH ROW EXECUTE FUNCTION game_house_sale_offer_guard();
CREATE FUNCTION game_house_sale_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_house_sale_event_proven(NEW.event_id);RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER proven AFTER INSERT ON game_house_sale_events
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_proven();
CREATE FUNCTION game_house_sale_hold_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_sale_offers;
BEGIN
 SELECT * INTO STRICT o FROM game_house_sale_offers WHERE offer_id=coalesce(NEW.offer_id,OLD.offer_id);
 PERFORM game_house_sale_event_proven(o.last_event);RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER proven AFTER INSERT OR UPDATE OR DELETE ON game_house_sale_holds
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_sale_hold_proven();
DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_account_bank_entry_consistency_guard()'::regprocedure);
 EXECUTE overlay(body placing E'BEGIN\n IF NEW.house_sale_event IS NOT NULL THEN PERFORM game_house_sale_event_proven(NEW.house_sale_event); RETURN NULL; END IF;'
 FROM strpos(body,'BEGIN') FOR 5);
END $$;
DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT oid::regprocedure FROM pg_proc WHERE pronamespace=current_schema()::regnamespace
  AND (proname LIKE 'game_house_sale_%' OR proname IN('game_account_bank_credit_headroom','game_account_bank_credit_headroom_before_t135')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_account_bank_credit_headroom(UUID,UUID),game_house_sale_reserves_slot(UUID,UUID) TO oteryn_game_runtime;
