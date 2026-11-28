-- A NULL stake preserves the terms and receipts of GUILD_WAR_V1.
ALTER TABLE game_guild_wars
 ADD COLUMN stake_gold BIGINT CHECK(stake_gold BETWEEN 0 AND 2000000000),
 ADD COLUMN stake_declarer UUID REFERENCES game_character_roots,
 ADD CONSTRAINT game_guild_war_stake_consent CHECK((stake_gold IS NULL)=(stake_declarer IS NULL)),
 DROP CONSTRAINT game_guild_wars_ended_reason_check,
 ADD CONSTRAINT game_guild_wars_ended_reason_check CHECK(ended_reason IN (1,2,3));

CREATE INDEX game_guild_war_stake_declarer ON game_guild_wars(stake_declarer) WHERE stake_declarer IS NOT NULL;

-- Keep retained V1 receipts intact. New receipts identify their physical transaction.
ALTER TABLE game_guild_war_commands ADD COLUMN created_xact_id XID8;
ALTER TABLE game_guild_war_commands ALTER COLUMN created_xact_id SET DEFAULT pg_current_xact_id();

CREATE TABLE game_guild_war_escrow (
 war_id UUID NOT NULL REFERENCES game_guild_wars,
 guild_id UUID NOT NULL REFERENCES game_guilds,
 world_id UUID NOT NULL,
 account_id UUID NOT NULL REFERENCES game_character_account_guards,
 leader UUID NOT NULL REFERENCES game_character_roots,
 stake_gold BIGINT NOT NULL CHECK(stake_gold BETWEEN 0 AND 2000000000),
 accept_session UUID NOT NULL,
 accept_command BIGINT NOT NULL,
 funded_xact XID8 NOT NULL DEFAULT pg_current_xact_id(),
 settled_xact XID8,
 settled_gold BIGINT CHECK(settled_gold BETWEEN 0 AND 2*stake_gold),
 CHECK((settled_xact IS NULL)=(settled_gold IS NULL)),
 PRIMARY KEY(war_id,guild_id),
 UNIQUE(war_id,account_id),
 FOREIGN KEY(accept_session,accept_command) REFERENCES game_guild_war_commands(game_session_id,command_id)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_guild_war_escrow_open_account ON game_guild_war_escrow(account_id,world_id)
 WHERE settled_xact IS NULL;
CREATE INDEX game_guild_war_escrow_guild ON game_guild_war_escrow(guild_id);
CREATE INDEX game_guild_war_escrow_leader ON game_guild_war_escrow(leader);
CREATE INDEX game_guild_war_escrow_account ON game_guild_war_escrow(account_id);
CREATE INDEX game_guild_war_escrow_receipt ON game_guild_war_escrow(accept_session,accept_command);
REVOKE ALL ON game_guild_war_escrow FROM PUBLIC;
GRANT SELECT ON game_guild_war_escrow TO oteryn_game_runtime;
GRANT SELECT ON game_guild_war_escrow TO oteryn_game_control;

CREATE TABLE game_guild_war_notices (
 notice_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 war_id UUID NOT NULL REFERENCES game_guild_wars,
 guild_id UUID NOT NULL REFERENCES game_guilds,
 occurred_at BIGINT NOT NULL,
 message TEXT NOT NULL CHECK(octet_length(message) BETWEEN 1 AND 255),
 UNIQUE(war_id,guild_id)
);
CREATE INDEX game_guild_war_notices_guild ON game_guild_war_notices(guild_id,notice_id);
REVOKE ALL ON game_guild_war_notices FROM PUBLIC;
GRANT SELECT,INSERT ON game_guild_war_notices TO oteryn_game_runtime;
GRANT SELECT ON game_guild_war_notices TO oteryn_game_control;

CREATE TABLE game_guild_war_notice_cursors (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 guild_id UUID NOT NULL REFERENCES game_guilds,
 notice_id BIGINT NOT NULL REFERENCES game_guild_war_notices,
 PRIMARY KEY(character_id,guild_id)
);
CREATE INDEX game_guild_war_notice_cursors_guild ON game_guild_war_notice_cursors(guild_id);
CREATE INDEX game_guild_war_notice_cursors_notice ON game_guild_war_notice_cursors(notice_id);
REVOKE ALL ON game_guild_war_notice_cursors FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_guild_war_notice_cursors TO oteryn_game_runtime;
GRANT SELECT ON game_guild_war_notice_cursors TO oteryn_game_control;

-- Guild disband removes its roster in bounded batches. Retain the exact membership
-- window so an offline recipient can still read the terminal war notice after cleanup.
CREATE TABLE game_guild_war_disband_members (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 guild_id UUID NOT NULL REFERENCES game_guilds,
 joined_at BIGINT NOT NULL,
 PRIMARY KEY(character_id,guild_id)
);
CREATE INDEX game_guild_war_disband_members_guild ON game_guild_war_disband_members(guild_id);
REVOKE ALL ON game_guild_war_disband_members FROM PUBLIC;
GRANT SELECT ON game_guild_war_disband_members TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_guild_war_keep_disband_member() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=OLD.guild_id AND state=3)
 AND (EXISTS(SELECT 1 FROM game_guild_wars WHERE state IN(1,2) AND OLD.guild_id IN(declaring_guild,opposing_guild))
      OR EXISTS(SELECT 1 FROM game_guild_war_notices WHERE guild_id=OLD.guild_id AND occurred_at>=OLD.joined_at))
 THEN
  INSERT INTO game_guild_war_disband_members(character_id,guild_id,joined_at)
  VALUES(OLD.character_id,OLD.guild_id,OLD.joined_at);
 END IF;
 RETURN OLD;
END $$;
CREATE TRIGGER game_guild_war_keep_disband_member AFTER DELETE ON game_guild_members
 FOR EACH ROW EXECUTE FUNCTION game_guild_war_keep_disband_member();

ALTER TABLE game_account_bank_entries
 ADD COLUMN war_escrow_id UUID,
 ADD COLUMN war_escrow_guild UUID,
 ADD COLUMN war_escrow_phase TEXT CHECK(war_escrow_phase IN('FUND','SETTLE')),
 ADD FOREIGN KEY(war_escrow_id,war_escrow_guild) REFERENCES game_guild_war_escrow,
 ADD CONSTRAINT game_account_bank_entries_war_reference
  CHECK(num_nonnulls(war_escrow_id,war_escrow_guild,war_escrow_phase) IN (0,3));
CREATE UNIQUE INDEX game_guild_war_bank_entry ON game_account_bank_entries(war_escrow_id,war_escrow_guild,war_escrow_phase)
 WHERE war_escrow_id IS NOT NULL;

-- Extend the current closed cause vocabulary without relaxing another cause's proof.
DO $$ DECLARE body TEXT;n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_account_bank_entries_one_reference','game_account_bank_entries_guild_reference','game_account_bank_entries_house_value'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT body FROM pg_constraint
   WHERE conrelid='game_account_bank_entries'::regclass AND conname=n;
  EXECUTE format('ALTER TABLE game_account_bank_entries DROP CONSTRAINT %I',n);
  IF n='game_account_bank_entries_one_reference' THEN
   IF strpos(body,'num_nonnulls(')=0 THEN RAISE EXCEPTION 'missing war bank reference seam'; END IF;
   body:=replace(body,'num_nonnulls(','num_nonnulls(war_escrow_id,');
  ELSIF n='game_account_bank_entries_guild_reference' THEN
   body:='CASE WHEN war_escrow_id IS NOT NULL THEN kind IS NULL AND guild_kind IS NULL AND market_kind IS NULL AND house_rent_kind IS NULL AND bazaar_kind IS NULL ELSE ('||body||') END';
  ELSE
   body:='CASE WHEN war_escrow_id IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND balance_after=balance_before+CASE war_escrow_phase WHEN ''FUND'' THEN -amount ELSE amount END ELSE ('||body||') END';
  END IF;
  EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT %I CHECK(%s)',n,body);
 END LOOP;
END $$;

CREATE FUNCTION game_guild_war_liability(a UUID,w UUID) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT coalesce(sum(2*stake_gold),0)::bigint FROM game_guild_war_escrow
 WHERE account_id=a AND world_id=w AND settled_xact IS NULL
$$;
ALTER FUNCTION game_account_bank_credit_headroom(UUID,UUID) RENAME TO game_account_bank_credit_headroom_before_war;
-- 0994's soft ceiling applies to accounts on either side of a Bazaar hold.
CREATE FUNCTION game_guild_war_bazaar_ceiling(a UUID,w UUID) RETURNS BIGINT
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT 999999999999-coalesce(sum(incoming_net) FILTER(WHERE seller_account_id=a),0)::bigint
 FROM game_bazaar_gold_holds WHERE world_id=w
 AND ((buyer_account_id=a AND amount>0) OR (seller_account_id=a AND incoming_net>0))
 HAVING count(*)>0
$$;
CREATE FUNCTION game_guild_war_credit_room(a UUID,w UUID,liquid BIGINT) RETURNS BIGINT
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,least(game_account_bank_credit_headroom_before_war(a,w),
     coalesce(game_guild_war_bazaar_ceiling(a,w)-liquid,9000000000000000)))
$$;
CREATE FUNCTION game_account_bank_credit_headroom(a UUID,w UUID) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,game_guild_war_credit_room(a,w,coalesce(
     (SELECT balance FROM game_account_bank_balances WHERE account_id=a AND world_id=w),0))
     -game_guild_war_liability(a,w))
$$;
CREATE FUNCTION game_guild_war_headroom_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE liquid BIGINT;
BEGIN
 SELECT balance INTO STRICT liquid FROM game_account_bank_balances
  WHERE account_id=NEW.account_id AND world_id=NEW.world_id;
 IF game_guild_war_liability(NEW.account_id,NEW.world_id)>game_guild_war_credit_room(NEW.account_id,NEW.world_id,liquid)
 THEN RAISE EXCEPTION 'guild war settlement headroom is reserved' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_war_bank_headroom AFTER INSERT OR UPDATE ON game_account_bank_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_war_headroom_guard();

-- Both domains serialize on the account bank rows. Bazaar adds reservations without
-- changing the balance, so its own hold write must enforce the same shared ceiling.
CREATE FUNCTION game_guild_war_bazaar_hold_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM 1 FROM game_account_bank_balances WHERE world_id=NEW.world_id
  AND account_id IN(NEW.buyer_account_id,NEW.seller_account_id) ORDER BY account_id FOR UPDATE;
 IF EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.world_id=NEW.world_id
   AND b.account_id IN(NEW.buyer_account_id,NEW.seller_account_id)
   AND game_guild_war_liability(b.account_id,b.world_id)>game_guild_war_credit_room(b.account_id,b.world_id,b.balance))
 OR (NEW.incoming_net>0 AND game_guild_war_liability(NEW.seller_account_id,NEW.world_id)>0
   AND 9000000000000000-game_account_bank_credit_headroom_before_bazaar(NEW.seller_account_id,NEW.world_id)
       +game_guild_war_liability(NEW.seller_account_id,NEW.world_id)
       >game_guild_war_bazaar_ceiling(NEW.seller_account_id,NEW.world_id))
 THEN RAISE EXCEPTION 'Bazaar hold would consume guild war settlement headroom'
  USING ERRCODE='23514', CONSTRAINT='game_guild_war_bazaar_hold_guard';
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_guild_war_bazaar_hold_guard AFTER INSERT OR UPDATE ON game_bazaar_gold_holds
 FOR EACH ROW EXECUTE FUNCTION game_guild_war_bazaar_hold_guard();

CREATE FUNCTION game_guild_war_bank_write(e game_guild_war_escrow,phase TEXT,delta BIGINT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE b game_account_bank_balances; entry UUID;
BEGIN
 IF delta=0 THEN RETURN; END IF;
 SELECT * INTO STRICT b FROM game_account_bank_balances
  WHERE account_id=e.account_id AND world_id=e.world_id FOR UPDATE;
 IF delta>0 AND game_account_bank_credit_headroom(e.account_id,e.world_id)<delta
 THEN RAISE EXCEPTION 'guild war settlement exceeds bank headroom' USING ERRCODE='23514'; END IF;
 entry:=game_character_uuid_v7();
 INSERT INTO game_account_bank_entries(entry_id,war_escrow_id,war_escrow_guild,war_escrow_phase,
  account_id,world_id,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
 VALUES(entry,e.war_id,e.guild_id,phase,e.account_id,e.world_id,abs(delta),b.balance,b.balance+delta,b.last_entry_id,e.leader);
 UPDATE game_account_bank_balances SET balance=b.balance+delta,last_entry_id=entry
 WHERE account_id=e.account_id AND world_id=e.world_id;
END $$;

-- These entry points use the war row already locked by the fenced Guild transaction.
CREATE FUNCTION game_guild_war_fund(id UUID,accepter UUID,session_id UUID,command BIGINT) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w game_guild_wars; e game_guild_war_escrow; g UUID; accounts UUID[]:=ARRAY[]::uuid[];
        a UUID; debit BIGINT; liquid BIGINT; held BIGINT; liability BIGINT; ceiling BIGINT;
BEGIN
 SELECT * INTO STRICT w FROM game_guild_wars WHERE war_id=id AND state=1 AND stake_gold IS NOT NULL FOR UPDATE;
 debit:=w.stake_gold+1000;
 -- Resolve both current leader accounts after locking both guilds in key order.
 FOR g IN SELECT guild_id FROM game_guilds WHERE guild_id IN(w.declaring_guild,w.opposing_guild)
  AND world_id=w.world_id AND state IN(1,2) ORDER BY guild_id FOR UPDATE LOOP
  SELECT r.account_id INTO a FROM game_guild_members m JOIN game_character_roots r USING(character_id)
   WHERE m.guild_id=g AND m.rank_level=1 AND r.world_id=w.world_id AND r.lifecycle=1
     AND m.character_id=CASE WHEN g=w.opposing_guild THEN accepter ELSE w.stake_declarer END;
  IF a IS NULL OR a=ANY(accounts) THEN RETURN FALSE; END IF;
  accounts:=array_append(accounts,a);
 END LOOP;
 IF cardinality(accounts)<>2 THEN RETURN FALSE; END IF;
 PERFORM 1 FROM game_character_account_guards WHERE account_id=ANY(accounts) ORDER BY account_id FOR UPDATE;
 FOR a IN SELECT unnest(accounts) ORDER BY 1 LOOP
  IF game_account_closure_closed(a) THEN RETURN FALSE; END IF;
  SELECT balance INTO liquid FROM game_account_bank_balances WHERE account_id=a AND world_id=w.world_id FOR UPDATE;
  SELECT coalesce(sum(amount),0)::bigint INTO held FROM game_bazaar_gold_holds
   WHERE buyer_account_id=a AND world_id=w.world_id AND amount>0;
  liability:=game_guild_war_liability(a,w.world_id)+2*w.stake_gold;
  ceiling:=game_guild_war_bazaar_ceiling(a,w.world_id);
  IF liquid IS NULL OR liquid-debit<held
   OR game_account_bank_credit_headroom_before_war(a,w.world_id)+debit<liability
   OR liquid-debit+liability>ceiling
   OR (EXISTS(SELECT 1 FROM game_bazaar_gold_holds WHERE seller_account_id=a AND world_id=w.world_id AND incoming_net>0)
       AND 9000000000000000-game_account_bank_credit_headroom_before_bazaar(a,w.world_id)-debit+liability>ceiling)
  THEN RETURN FALSE; END IF;
 END LOOP;
 FOR e IN INSERT INTO game_guild_war_escrow(war_id,guild_id,world_id,account_id,leader,stake_gold,accept_session,accept_command)
  SELECT id,m.guild_id,w.world_id,r.account_id,m.character_id,w.stake_gold,session_id,command
  FROM game_guild_members m JOIN game_character_roots r USING(character_id)
  WHERE m.guild_id IN(w.declaring_guild,w.opposing_guild) AND m.rank_level=1
  RETURNING * LOOP
  PERFORM game_guild_war_bank_write(e,'FUND',-debit);
 END LOOP;
 RETURN TRUE;
END $$;

CREATE FUNCTION game_guild_war_settle(id UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w game_guild_wars; e game_guild_war_escrow; winner UUID; transfer BIGINT:=0; credit BIGINT;
BEGIN
 SELECT * INTO STRICT w FROM game_guild_wars WHERE war_id=id AND state=3 AND stake_gold IS NOT NULL FOR UPDATE;
 IF w.ended_reason<>3 AND w.declaring_frags<>w.opposing_frags THEN
  winner:=CASE WHEN w.declaring_frags>w.opposing_frags THEN w.declaring_guild ELSE w.opposing_guild END;
  transfer:=CASE WHEN w.ended_reason=1 THEN w.stake_gold
   ELSE w.stake_gold*abs(w.declaring_frags::bigint-w.opposing_frags)/w.frag_limit END;
 END IF;
 PERFORM 1 FROM game_character_account_guards a JOIN game_guild_war_escrow escrow USING(account_id)
  WHERE escrow.war_id=id AND escrow.settled_xact IS NULL ORDER BY a.account_id FOR UPDATE OF a;
 FOR e IN SELECT * FROM game_guild_war_escrow WHERE war_id=id AND settled_xact IS NULL ORDER BY account_id FOR UPDATE LOOP
  PERFORM 1 FROM game_account_bank_balances WHERE account_id=e.account_id AND world_id=e.world_id FOR UPDATE;
  credit:=e.stake_gold+CASE WHEN winner=e.guild_id THEN transfer ELSE -transfer END;
  UPDATE game_guild_war_escrow SET settled_xact=pg_current_xact_id(),settled_gold=credit
   WHERE war_id=id AND guild_id=e.guild_id RETURNING * INTO e;
  PERFORM game_guild_war_bank_write(e,'SETTLE',credit);
 END LOOP;
END $$;

CREATE FUNCTION game_guild_war_ledger_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE e game_guild_war_escrow; b game_account_bank_entries:=NEW;
BEGIN
 SELECT * INTO STRICT e FROM game_guild_war_escrow WHERE war_id=b.war_escrow_id AND guild_id=b.war_escrow_guild;
 IF (b.account_id,b.world_id,b.acting_character_id) IS DISTINCT FROM (e.account_id,e.world_id,e.leader)
  OR b.created_xact_id<>pg_current_xact_id()
  OR (CASE b.war_escrow_phase WHEN 'FUND' THEN e.funded_xact ELSE e.settled_xact END) IS DISTINCT FROM pg_current_xact_id()
  OR (CASE b.war_escrow_phase WHEN 'FUND' THEN e.stake_gold+1000 ELSE e.settled_gold END) IS DISTINCT FROM b.amount
  OR (b.previous_entry_id IS NULL AND b.balance_before<>0)
  OR (b.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p
   WHERE p.entry_id=b.previous_entry_id AND p.account_id=b.account_id AND p.world_id=b.world_id AND p.balance_after=b.balance_before))
  OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances p WHERE p.account_id=b.account_id AND p.world_id=b.world_id AND p.last_entry_id=b.entry_id)
   OR EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.previous_entry_id=b.entry_id))
 THEN RAISE EXCEPTION 'guild war bank entry lacks its exact escrow cause or balance chain' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_war_ledger_proven AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.war_escrow_id IS NOT NULL)
 EXECUTE FUNCTION game_guild_war_ledger_proven();

CREATE FUNCTION game_guild_war_escrow_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w game_guild_wars; e game_guild_war_escrow;
BEGIN
 SELECT * INTO STRICT w FROM game_guild_wars WHERE war_id=NEW.war_id;
 IF TG_TABLE_NAME='game_guild_wars' THEN
  IF w.stake_gold IS NULL THEN RETURN NULL; END IF;
  IF w.state=2 AND (SELECT count(*) FROM game_guild_war_escrow WHERE war_id=w.war_id AND funded_xact=pg_current_xact_id())<>2
  THEN RAISE EXCEPTION 'guild war activation requires both bank debits' USING ERRCODE='23514'; END IF;
  IF w.state=3 AND w.started_at IS NOT NULL AND (SELECT count(*) FROM game_guild_war_escrow WHERE war_id=w.war_id AND settled_xact=pg_current_xact_id())<>2
  THEN RAISE EXCEPTION 'guild war end requires both stakes settled' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 e:=NEW;
 IF e.guild_id NOT IN(w.declaring_guild,w.opposing_guild) OR e.world_id<>w.world_id OR e.stake_gold IS DISTINCT FROM w.stake_gold
  OR (e.settled_xact IS NULL AND (w.state<>2 OR NOT EXISTS(SELECT 1 FROM game_account_bank_entries
   WHERE war_escrow_id=e.war_id AND war_escrow_guild=e.guild_id AND war_escrow_phase='FUND')
   OR NOT EXISTS(SELECT 1 FROM game_guild_war_commands c WHERE c.game_session_id=e.accept_session
    AND c.command_id=e.accept_command AND c.war_id=e.war_id AND c.result=1 AND c.created_xact_id=e.funded_xact)))
  OR (e.settled_xact IS NOT NULL AND (w.state<>3 OR (e.settled_gold>0) IS DISTINCT FROM EXISTS(SELECT 1 FROM game_account_bank_entries
   WHERE war_escrow_id=e.war_id AND war_escrow_guild=e.guild_id AND war_escrow_phase='SETTLE')))
 THEN RAISE EXCEPTION 'guild war escrow lacks its physical transition or ledger entry' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_guild_war_terms_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF (NEW.stake_gold,NEW.stake_declarer,NEW.world_id,NEW.declaring_guild,NEW.opposing_guild,NEW.frag_limit,NEW.duration_days)
    IS DISTINCT FROM (OLD.stake_gold,OLD.stake_declarer,OLD.world_id,OLD.declaring_guild,OLD.opposing_guild,OLD.frag_limit,OLD.duration_days)
 THEN RAISE EXCEPTION 'guild war terms cannot change' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_war_terms_immutable BEFORE UPDATE OF stake_gold,stake_declarer,world_id,declaring_guild,opposing_guild,frag_limit,duration_days
 ON game_guild_wars FOR EACH ROW EXECUTE FUNCTION game_guild_war_terms_immutable();

CREATE CONSTRAINT TRIGGER game_guild_war_insert AFTER INSERT ON game_guild_wars
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.stake_gold IS NOT NULL AND NEW.state<>1)
 EXECUTE FUNCTION game_guild_war_escrow_proven();
CREATE CONSTRAINT TRIGGER game_guild_war_activation_and_end AFTER UPDATE ON game_guild_wars
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (OLD.state IS DISTINCT FROM NEW.state)
 EXECUTE FUNCTION game_guild_war_escrow_proven();
CREATE CONSTRAINT TRIGGER game_guild_war_escrow_proven AFTER INSERT OR UPDATE ON game_guild_war_escrow
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_war_escrow_proven();

-- Preserve the Market branch in the existing closure guard. The only additional
-- admitted write is a credit from an already funded war ending in this transaction.
DO $$ DECLARE body TEXT; anchor TEXT:=' -- Market remains SELECT-only to runtime.';
 entry_check TEXT:=$entry$closed IS NOT NULL AND TG_TABLE_NAME='game_account_bank_entries' AND TG_OP='INSERT'$entry$;
 balance_check TEXT:=$balance$closed IS NOT NULL AND TG_TABLE_NAME='game_account_bank_balances' AND TG_OP='UPDATE'$balance$;
 allowed TEXT:=$state$(closed IS NOT NULL OR EXISTS(SELECT 1 FROM public.game_account_closures WHERE account_id=a AND state='CANCELLING'))$state$;
BEGIN
 SELECT pg_get_functiondef('game_account_closure_bank_guard()'::regprocedure) INTO STRICT body;
 IF strpos(body,anchor)=0 OR strpos(body,entry_check)=0 OR strpos(body,balance_check)=0
 THEN RAISE EXCEPTION 'missing closing bank settlement seam'; END IF;
 body:=replace(body,entry_check,replace(entry_check,'closed IS NOT NULL',allowed));
 body:=replace(body,balance_check,replace(balance_check,'closed IS NOT NULL',allowed));
 EXECUTE replace(body,anchor,$war$
 IF e.account_id=a AND e.world_id=(j->>'world_id')::uuid AND e.created_xact_id=pg_current_xact_id()
 AND e.war_escrow_phase='SETTLE' AND e.amount>0 AND e.balance_after=e.balance_before+e.amount
 AND EXISTS(SELECT 1 FROM public.game_guild_war_escrow h JOIN public.game_guild_wars w USING(war_id)
  WHERE h.war_id=e.war_escrow_id AND h.guild_id=e.war_escrow_guild AND h.account_id=a
   AND h.world_id=e.world_id AND h.leader=e.acting_character_id AND h.funded_xact<>pg_current_xact_id()
   AND h.settled_xact=pg_current_xact_id() AND h.settled_gold=e.amount AND w.state=3)
 THEN RETURN NEW; END IF;
 IF closed IS NULL THEN RAISE EXCEPTION 'new bank work on closing account' USING ERRCODE='55000'; END IF;
$war$||anchor);
END $$;

DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT oid::regprocedure FROM pg_proc WHERE pronamespace=current_schema()::regnamespace
  AND (proname LIKE 'game_guild_war_%' OR proname IN('game_account_bank_credit_headroom','game_account_bank_credit_headroom_before_war')) LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_account_bank_credit_headroom(UUID,UUID),game_guild_war_fund(UUID,UUID,UUID,BIGINT),game_guild_war_settle(UUID)
 TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_guild_war_liability(UUID,UUID) TO oteryn_game_runtime;
