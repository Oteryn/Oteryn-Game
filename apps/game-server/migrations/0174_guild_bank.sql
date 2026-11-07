-- GUILD-BANK-1, GUILD0 §5. Symbolic Guild kinds do not allocate numeric Bank/event IDs.
-- Applied migrations are immutable. Rollback must retain financial chains, receipts and claims.
CREATE TABLE game_guild_bank_operations (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 guild_id UUID NOT NULL REFERENCES game_guilds,
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 account_id UUID NOT NULL REFERENCES game_character_account_guards,
 actor UUID NOT NULL REFERENCES game_character_roots,
 kind TEXT NOT NULL CHECK(kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM')),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 game_session_id UUID, command_id NUMERIC(20,0),
 guild_revision NUMERIC(20,0) NOT NULL CHECK(guild_revision BETWEEN 1 AND 18446744073709551615),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 999999999999),
 result SMALLINT NOT NULL CHECK(result BETWEEN 1 AND 7),
 bank_before BIGINT NOT NULL CHECK(bank_before BETWEEN 0 AND 9000000000000000),
 bank_after BIGINT NOT NULL CHECK(bank_after BETWEEN 0 AND 9000000000000000),
 guild_before BIGINT NOT NULL CHECK(guild_before BETWEEN 0 AND 999999999999),
 guild_after BIGINT NOT NULL CHECK(guild_after BETWEEN 0 AND 999999999999),
 claim_before BIGINT NOT NULL CHECK(claim_before BETWEEN 0 AND 999999999999),
 claim_after BIGINT NOT NULL CHECK(claim_after BETWEEN 0 AND 999999999999),
 reply BYTEA NOT NULL CHECK(octet_length(reply) BETWEEN 1 AND 16384),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK((kind='GUILD_DISBAND_PAYOUT')=(game_session_id IS NULL)),
 CHECK((game_session_id IS NULL)=(command_id IS NULL)),
 CHECK(command_id IS NULL OR command_id BETWEEN 1 AND 18446744073709551615),
 CHECK(game_session_id IS NULL OR game_character_is_uuid_v7(game_session_id)),
 CHECK(result<>1 OR CASE kind
  WHEN 'GUILD_DEPOSIT' THEN amount>0 AND bank_before-bank_after=amount AND guild_after-guild_before=amount AND claim_before=claim_after
  WHEN 'GUILD_WITHDRAW' THEN amount>0 AND bank_after-bank_before=amount AND guild_before-guild_after=amount AND claim_before=claim_after AND bank_after<=999999999999
  WHEN 'GUILD_DISBAND_PAYOUT' THEN amount=guild_before AND guild_after=0 AND claim_before=0 AND bank_after-bank_before+claim_after=amount
  ELSE amount>0 AND bank_after-bank_before=amount AND claim_before-claim_after=amount AND guild_before=guild_after END),
 CHECK(result=1 OR (bank_before=0 AND bank_after=0 AND guild_before=0 AND guild_after=0 AND claim_before=0 AND claim_after=0)),
 CHECK(kind<>'GUILD_DISBAND_PAYOUT' OR result=1)
);
CREATE UNIQUE INDEX game_guild_bank_one_payout ON game_guild_bank_operations(guild_id) WHERE kind='GUILD_DISBAND_PAYOUT';
CREATE TABLE game_guild_bank_entries (
 entry_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(entry_id)),
 operation_id UUID NOT NULL UNIQUE REFERENCES game_guild_bank_operations,
 guild_id UUID NOT NULL REFERENCES game_guilds,
 ordinal BIGINT NOT NULL CHECK(ordinal>0),
 kind TEXT NOT NULL CHECK(kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT')),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1 AND 999999999999),
 balance_before BIGINT NOT NULL CHECK(balance_before BETWEEN 0 AND 999999999999),
 balance_after BIGINT NOT NULL CHECK(balance_after BETWEEN 0 AND 999999999999),
 previous_entry_id UUID UNIQUE REFERENCES game_guild_bank_entries,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(guild_id,ordinal),
 CHECK(previous_entry_id IS DISTINCT FROM entry_id),
 CHECK(balance_after=balance_before+CASE kind WHEN 'GUILD_DEPOSIT' THEN amount ELSE -amount END)
);
CREATE TABLE game_guild_bank_balances (
 guild_id UUID PRIMARY KEY REFERENCES game_guilds,
 balance BIGINT NOT NULL CHECK(balance BETWEEN 0 AND 999999999999),
 last_entry_id UUID REFERENCES game_guild_bank_entries,
 ordinal BIGINT NOT NULL DEFAULT 0 CHECK(ordinal>=0),
 CHECK((last_entry_id IS NULL)=(ordinal=0)),
 CHECK(last_entry_id IS NOT NULL OR balance=0)
);
CREATE TABLE game_guild_disband_claims (
 guild_id UUID PRIMARY KEY REFERENCES game_guilds,
 account_id UUID NOT NULL REFERENCES game_character_account_guards,
 world_id UUID NOT NULL,
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 999999999999),
 payout_id UUID NOT NULL UNIQUE REFERENCES game_guild_bank_operations,
 last_operation_id UUID NOT NULL UNIQUE REFERENCES game_guild_bank_operations,
 created_at BIGINT NOT NULL CHECK(created_at>0)
);
CREATE INDEX game_guild_disband_claim_owner ON game_guild_disband_claims(account_id,world_id,guild_id) WHERE amount>0;
-- Native owner evidence, like Guild core's private audit proposals. Not an admitted numeric EventEnvelope.
CREATE TABLE game_guild_bank_event_outbox (
 operation_id UUID NOT NULL REFERENCES game_guild_bank_operations,
 owner TEXT NOT NULL CHECK(owner IN('BANK','GUILD')),
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 2048),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+2592000000),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(operation_id,owner)
);

-- Keep the complete Bank/Fee/NPC and House guards; select their old arms by their old numeric kind.
ALTER TABLE game_account_bank_entries
 ALTER COLUMN kind DROP NOT NULL,
 ADD COLUMN guild_operation_id UUID UNIQUE REFERENCES game_guild_bank_operations DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN guild_kind TEXT CHECK(guild_kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM')),
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_house_value,
 DROP CONSTRAINT game_account_bank_entries_balance_before_check,
 DROP CONSTRAINT game_account_bank_entries_balance_after_check,
 ADD CONSTRAINT game_account_bank_entries_balance_before_check CHECK(balance_before BETWEEN 0 AND 9000000000000000),
 ADD CONSTRAINT game_account_bank_entries_balance_after_check CHECK(balance_after BETWEEN 0 AND 9000000000000000),
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id,guild_operation_id)=1),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
  (guild_operation_id IS NOT NULL)=(guild_kind IS NOT NULL) AND (kind IS NULL)=(guild_kind IS NOT NULL)),
 ADD CONSTRAINT game_account_bank_entries_house_value CHECK(
  CASE WHEN guild_kind IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL
    AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
    AND balance_after=balance_before+CASE guild_kind WHEN 'GUILD_DEPOSIT' THEN -amount ELSE amount END
  WHEN kind>=7 THEN escrow_before BETWEEN 0 AND 999999999999 AND escrow_after BETWEEN 0 AND 999999999999
   AND escrow_before IS NOT NULL AND escrow_after IS NOT NULL
   AND balance_after=balance_before+CASE kind WHEN 7 THEN -amount WHEN 8 THEN amount ELSE 0 END
   AND escrow_after=escrow_before+CASE kind WHEN 7 THEN amount ELSE -amount END
  ELSE escrow_before IS NULL AND escrow_after IS NULL
   AND balance_after=balance_before+CASE WHEN kind IN(1,4) THEN amount ELSE -amount END END);
ALTER TABLE game_account_bank_balances DROP CONSTRAINT game_account_bank_balances_balance_check,
 ADD CONSTRAINT game_account_bank_balances_balance_check CHECK(balance BETWEEN 0 AND 9000000000000000);

CREATE INDEX game_market_account_credit_custody ON game_market_offers(account_id,world_id) WHERE state=1 OR held_credit_gold>0;

-- One hard headroom source for owned-value returns. Ordinary credits retain their soft cap in L3.
CREATE OR REPLACE FUNCTION game_account_bank_credit_headroom(p_account UUID,p_world UUID)
RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,9000000000000000-coalesce((SELECT balance FROM game_account_bank_balances
  WHERE account_id=p_account AND world_id=p_world),0)-coalesce((SELECT sum(escrow_gold)
  FROM game_house_bids WHERE account_id=p_account AND world_id=p_world AND state=1),0)
  -coalesce((SELECT sum(escrow_gold+held_credit_gold) FROM game_market_offers WHERE account_id=p_account AND world_id=p_world AND (state=1 OR held_credit_gold>0)),0))::bigint
$$;
-- House custody remains reserved even when a disband raises the liquid balance above the soft ceiling.
CREATE OR REPLACE FUNCTION game_house_headroom_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1)
  AND EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=NEW.account_id AND world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'House bid and personal slot are mutually exclusive' USING ERRCODE='23514';END IF;
 IF coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=NEW.account_id AND world_id=NEW.world_id),0)
  +coalesce((SELECT sum(escrow_gold) FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1),0)
  +coalesce((SELECT sum(escrow_gold+held_credit_gold) FROM game_market_offers WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND (state=1 OR held_credit_gold>0)),0)>9000000000000000 THEN
  RAISE EXCEPTION 'HOUSE_BANK_HEADROOM' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;

CREATE FUNCTION game_guild_bank_payload(o game_guild_bank_operations,own TEXT) RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$
 SELECT jsonb_build_object('version',1,'class','TRANSFER','kind',o.kind,'owner',own,
  'guild',o.guild_id,'account',o.account_id,'world',o.world_id,'actor',o.actor,
  'bank_before',o.bank_before,'bank_after',o.bank_after,'guild_before',o.guild_before,'guild_after',o.guild_after,
  'claim_before',o.claim_before,'claim_after',o.claim_after,'amount',o.amount)
$$;
CREATE FUNCTION game_guild_bank_balance_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.balance<>0 OR NEW.last_entry_id IS NOT NULL OR NEW.ordinal<>0 THEN
   RAISE EXCEPTION 'Guild balance starts at zero' USING ERRCODE='23514';END IF;
 ELSE
  IF NEW.guild_id<>OLD.guild_id OR NEW.ordinal<>OLD.ordinal+1 OR NOT EXISTS(
   SELECT 1 FROM game_guild_bank_entries e WHERE e.entry_id=NEW.last_entry_id AND e.guild_id=NEW.guild_id
    AND e.balance_before=OLD.balance AND e.balance_after=NEW.balance AND e.ordinal=NEW.ordinal
    AND e.previous_entry_id IS NOT DISTINCT FROM OLD.last_entry_id AND e.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Guild balance requires exact same-transaction ledger successor' USING ERRCODE='23514';END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_bank_balance_proven AFTER INSERT OR UPDATE ON game_guild_bank_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_balance_proven();
CREATE FUNCTION game_guild_bank_entry_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations%ROWTYPE;
BEGIN
 SELECT * INTO o FROM game_guild_bank_operations WHERE operation_id=NEW.operation_id;
 IF o.operation_id IS NULL OR o.result<>1 OR o.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
  OR (NEW.guild_id,NEW.kind,NEW.amount,NEW.balance_before,NEW.balance_after) IS DISTINCT FROM
     (o.guild_id,o.kind,o.amount,o.guild_before,o.guild_after)
  OR (NEW.previous_entry_id IS NULL AND (NEW.ordinal<>1 OR NEW.balance_before<>0))
  OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries e
   WHERE e.entry_id=NEW.previous_entry_id AND e.guild_id=NEW.guild_id AND e.balance_after=NEW.balance_before AND e.ordinal=NEW.ordinal-1))
  OR NOT (EXISTS(SELECT 1 FROM game_guild_bank_balances b WHERE b.guild_id=NEW.guild_id AND b.last_entry_id=NEW.entry_id)
   OR EXISTS(SELECT 1 FROM game_guild_bank_entries e WHERE e.previous_entry_id=NEW.entry_id)) THEN
  RAISE EXCEPTION 'Guild entry lacks its operation and chain' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_bank_entry_proven AFTER INSERT ON game_guild_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_entry_proven();
CREATE FUNCTION game_guild_bank_account_entry_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations%ROWTYPE;
BEGIN
 SELECT * INTO o FROM game_guild_bank_operations WHERE operation_id=NEW.guild_operation_id;
 IF o.operation_id IS NULL OR o.result<>1 OR o.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
  OR (NEW.account_id,NEW.world_id,NEW.acting_character_id,NEW.guild_kind,NEW.amount,NEW.balance_before,NEW.balance_after) IS DISTINCT FROM
    (o.account_id,o.world_id,o.actor,o.kind,abs(o.bank_after-o.bank_before),o.bank_before,o.bank_after)
  OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
  OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e
   WHERE e.entry_id=NEW.previous_entry_id AND (e.account_id,e.world_id,e.balance_after)=(NEW.account_id,NEW.world_id,NEW.balance_before)))
  OR NOT (EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE (b.account_id,b.world_id,b.last_entry_id)=(NEW.account_id,NEW.world_id,NEW.entry_id))
   OR EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.previous_entry_id=NEW.entry_id)) THEN
  RAISE EXCEPTION 'Guild Account entry lacks paired operation/chain' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_bank_account_entry_proven AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.guild_operation_id IS NOT NULL)
 EXECUTE FUNCTION game_guild_bank_account_entry_proven();
CREATE FUNCTION game_guild_disband_claim_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations%ROWTYPE;
BEGIN
 SELECT * INTO o FROM game_guild_bank_operations WHERE operation_id=NEW.last_operation_id;
 IF o.result IS DISTINCT FROM 1 OR o.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR (NEW.guild_id,NEW.account_id,NEW.world_id,NEW.amount) IS DISTINCT FROM (o.guild_id,o.account_id,o.world_id,o.claim_after) THEN
  RAISE EXCEPTION 'Claim requires a same-transaction bound operation' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' THEN
  IF o.kind<>'GUILD_DISBAND_PAYOUT' OR NEW.payout_id<>o.operation_id OR NEW.created_at<>o.occurred_at THEN
   RAISE EXCEPTION 'Only payout creates retained custody' USING ERRCODE='23514';END IF;
 ELSE
  IF (NEW.guild_id,NEW.account_id,NEW.world_id,NEW.payout_id,NEW.created_at) IS DISTINCT FROM
      (OLD.guild_id,OLD.account_id,OLD.world_id,OLD.payout_id,OLD.created_at)
   OR o.kind<>'GUILD_DISBAND_CLAIM' OR o.claim_before<>OLD.amount OR NEW.amount>=OLD.amount THEN
   RAISE EXCEPTION 'Claim binding is immutable and amount only decreases' USING ERRCODE='23514';END IF;
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER game_guild_disband_claim_transition BEFORE INSERT OR UPDATE ON game_guild_disband_claims
 FOR EACH ROW EXECUTE FUNCTION game_guild_disband_claim_transition();

-- No Guild property can exist in the current private-House schema. Unknown future Hall/House
-- participants must supply their real release proof before this predicate can be extended.
CREATE FUNCTION game_guild_bank_release_ready(g UUID,w UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=g AND world_id=w AND state IN(3,4))
 AND NOT EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
  WHERE n.nspname=current_schema() AND c.relkind IN('r','p') AND
   (c.relname LIKE 'game_guildhall_%' OR c.relname LIKE 'game_house_guild_%'))
 AND NOT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema()
  AND table_name='game_house_properties' AND column_name='owner_guild_id')
$$;
CREATE FUNCTION game_guild_bank_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations%ROWTYPE;oid UUID; own TEXT;
BEGIN
 IF TG_TABLE_NAME='game_guild_bank_operations' THEN oid:=NEW.operation_id;
 ELSIF TG_TABLE_NAME='game_account_bank_entries' THEN oid:=NEW.guild_operation_id;
 ELSE oid:=NEW.operation_id;END IF;
 SELECT * INTO o FROM game_guild_bank_operations WHERE operation_id=oid;
 IF o.operation_id IS NULL OR o.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=o.actor AND account_id=o.account_id AND world_id=o.world_id)
  OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id)
  OR ((SELECT count(*) FROM game_account_bank_entries WHERE guild_operation_id=oid)<>CASE WHEN o.result=1 AND o.bank_before<>o.bank_after THEN 1 ELSE 0 END)
  OR ((SELECT count(*) FROM game_guild_bank_entries WHERE operation_id=oid)<>CASE WHEN o.result=1 AND o.guild_before<>o.guild_after THEN 1 ELSE 0 END)
  OR ((SELECT count(*) FROM game_guild_bank_event_outbox WHERE operation_id=oid)<>CASE WHEN o.result<>1 THEN 0 WHEN o.kind='GUILD_DISBAND_CLAIM' THEN 1 ELSE 2 END) THEN
  RAISE EXCEPTION 'Guild bank operation must commit its complete paired ledger/audits' USING ERRCODE='23514';END IF;
 IF o.result=1 THEN
  FOREACH own IN ARRAY CASE WHEN o.kind='GUILD_DISBAND_CLAIM' THEN ARRAY['BANK'] ELSE ARRAY['BANK','GUILD'] END LOOP
   IF NOT EXISTS(SELECT 1 FROM game_guild_bank_event_outbox e WHERE e.operation_id=oid AND e.owner=own
    AND e.created_xact_id=pg_current_xact_id() AND e.occurred_at=o.occurred_at
    AND convert_from(e.payload,'UTF8')::jsonb=game_guild_bank_payload(o,own)) THEN
    RAISE EXCEPTION 'Guild bank audit does not describe conserved operation' USING ERRCODE='23514';END IF;
  END LOOP;
  IF o.kind='GUILD_DISBAND_PAYOUT' THEN
   IF NOT game_guild_bank_release_ready(o.guild_id,o.world_id)
    OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state=3 AND revision=o.guild_revision)
    OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=1 AND character_id=o.actor)
    OR NOT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=o.guild_id AND world_id=o.world_id AND step=1)
    OR NOT EXISTS(SELECT 1 FROM game_guild_disband_claims WHERE guild_id=o.guild_id AND payout_id=oid AND amount=o.claim_after)
    OR NOT EXISTS(SELECT 1 FROM game_guild_bank_balances WHERE guild_id=o.guild_id AND balance=0) THEN
    RAISE EXCEPTION 'Payout lacks genuine pending Guild and released property/custody' USING ERRCODE='23514';END IF;
  ELSIF o.kind='GUILD_DISBAND_CLAIM' AND NOT EXISTS(SELECT 1 FROM game_guild_disband_claims
    WHERE guild_id=o.guild_id AND account_id=o.account_id AND world_id=o.world_id AND last_operation_id=oid AND amount=o.claim_after) THEN
   RAISE EXCEPTION 'Claim debit missing' USING ERRCODE='23514';
  END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_bank_operation_proven AFTER INSERT ON game_guild_bank_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_operation_proven();
CREATE CONSTRAINT TRIGGER game_guild_bank_event_proven AFTER INSERT ON game_guild_bank_event_outbox
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_operation_proven();
CREATE CONSTRAINT TRIGGER game_guild_bank_account_operation_proven AFTER INSERT ON game_account_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.guild_operation_id IS NOT NULL) EXECUTE FUNCTION game_guild_bank_operation_proven();
CREATE CONSTRAINT TRIGGER game_guild_bank_entry_operation_proven AFTER INSERT ON game_guild_bank_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_operation_proven();

CREATE OR REPLACE FUNCTION game_guild_core_assert_released(g UUID,w UUID) RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_guild_bank_release_ready(g,w) OR NOT EXISTS(
  SELECT 1 FROM game_guild_bank_operations o JOIN game_guild_bank_balances b USING(guild_id)
   JOIN game_guild_disband_claims c ON c.payout_id=o.operation_id AND c.guild_id=o.guild_id
   WHERE o.guild_id=g AND o.world_id=w AND o.kind='GUILD_DISBAND_PAYOUT' AND o.result=1
    AND o.account_id=c.account_id AND o.world_id=c.world_id AND b.balance=0) THEN
  RAISE EXCEPTION 'Guild cleanup requires released property and conserved retained payout' USING ERRCODE='23514';END IF;
END $$;

DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guild_bank_operations','game_guild_bank_entries','game_guild_bank_event_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_immutable',tab);
 END LOOP;
 FOREACH tab IN ARRAY ARRAY['game_guild_bank_operations','game_guild_bank_entries','game_guild_bank_balances','game_guild_disband_claims','game_guild_bank_event_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_no_truncate',tab);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC',tab);
 END LOOP;
END $$;
CREATE TRIGGER game_guild_bank_balance_no_delete BEFORE DELETE ON game_guild_bank_balances FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_disband_claim_no_delete BEFORE DELETE ON game_guild_disband_claims FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
GRANT SELECT,INSERT ON game_guild_bank_operations,game_guild_bank_entries,game_guild_bank_event_outbox TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_guild_bank_balances,game_guild_disband_claims TO oteryn_game_runtime;

-- Bind the two SECURITY DEFINER helpers to their installed schema, never caller search_path.
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_account_bank_credit_headroom(UUID,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_headroom_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
