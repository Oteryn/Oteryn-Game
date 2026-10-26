-- GUILDHALL-1: retain the qualified catalogue class and extend the existing
-- House/Guild Bank owners with Guild custody, without a surrogate personal owner.
ALTER TABLE game_house_properties DROP CONSTRAINT game_house_properties_kind_check;
ALTER TABLE game_house_properties
 ADD CONSTRAINT game_house_properties_kind_check
  CHECK(kind IN ('private_house','shop','guildhall')),
 ADD CONSTRAINT game_house_guildhall_lifecycle_absent CHECK(
  kind<>'guildhall' OR (state=1 AND owner_character_id IS NULL AND owner_account_id IS NULL
   AND paid_until IS NULL AND acquisition_receipt_ref IS NULL));

COMMENT ON CONSTRAINT game_house_guildhall_lifecycle_absent ON game_house_properties IS
 'GUILDHALL-1 classification only: delegates leader eligibility, Guild custody, ACL, escrow, rent and release to their accepted owners';

-- GUILDHALL-1: Guild custody extends the existing House spine, never a personal slot.
ALTER TABLE game_house_properties DROP CONSTRAINT game_house_guildhall_lifecycle_absent,
 ADD COLUMN owner_guild_id UUID REFERENCES game_guilds,
 ADD COLUMN rent_grace_until TIMESTAMPTZ,
 ADD COLUMN last_guildhall_operation UUID,
 DROP CONSTRAINT game_house_properties_check1,
 ADD CONSTRAINT game_house_property_owner_kind CHECK(
  CASE WHEN kind='guildhall' THEN owner_character_id IS NULL AND owner_account_id IS NULL
   AND ((state IN(3,4,5))=(owner_guild_id IS NOT NULL))
  ELSE owner_guild_id IS NULL AND ((state IN(3,4,5))=(owner_character_id IS NOT NULL)) END),
 ADD CONSTRAINT game_house_guild_paid CHECK(owner_guild_id IS NULL OR (paid_until IS NOT NULL AND acquisition_receipt_ref IS NOT NULL));
CREATE UNIQUE INDEX game_house_one_guild_owner ON game_house_properties(owner_guild_id) WHERE owner_guild_id IS NOT NULL;
ALTER TABLE game_house_ownership ADD COLUMN owner_guild_id UUID REFERENCES game_guilds,
 ADD CONSTRAINT game_house_anchor_owner_kind CHECK(num_nonnulls(owner_character_id,owner_guild_id)<=1);
ALTER TABLE game_house_bids ADD COLUMN guild_id UUID REFERENCES game_guilds,
 ADD COLUMN escrow_guild_gold BIGINT NOT NULL DEFAULT 0 CHECK(escrow_guild_gold BETWEEN 0 AND 999999999999),
 ADD CONSTRAINT game_house_split_escrow CHECK(escrow_guild_gold<=escrow_gold AND (guild_id IS NOT NULL OR escrow_guild_gold=0));
DROP INDEX game_house_one_held_bid;
CREATE UNIQUE INDEX game_house_one_held_bid ON game_house_bids(account_id,world_id) WHERE state=1 AND guild_id IS NULL;
CREATE UNIQUE INDEX game_house_one_held_guild_bid ON game_house_bids(guild_id) WHERE state=1 AND guild_id IS NOT NULL;
-- A former leader may fund a retained bid for one Guild and later lead another.
-- Contributor identity cannot also be the Guild bid's unique identity.
ALTER TABLE game_house_bids DROP CONSTRAINT game_house_bids_pkey,
 ADD COLUMN bid_id UUID NOT NULL DEFAULT game_character_uuid_v7(),
 ADD PRIMARY KEY(bid_id);
CREATE UNIQUE INDEX game_house_personal_auction_bid ON game_house_bids(auction_id,account_id) WHERE guild_id IS NULL;
CREATE UNIQUE INDEX game_house_guild_auction_bid ON game_house_bids(auction_id,guild_id) WHERE guild_id IS NOT NULL;
DO $$ DECLARE definition TEXT; BEGIN
 SELECT pg_get_functiondef('game_house_value_apply(jsonb,uuid,numeric,bytea,bytea)'::regprocedure) INTO definition;
 definition:=replace(definition,'ON CONFLICT(auction_id,account_id) DO UPDATE',
  'ON CONFLICT(auction_id,account_id) WHERE guild_id IS NULL DO UPDATE');
 EXECUTE definition;
END $$;
CREATE TABLE game_guild_house_bans(guild_id UUID PRIMARY KEY REFERENCES game_guilds, world_id UUID NOT NULL, banned_until TIMESTAMPTZ NOT NULL);
REVOKE ALL ON game_guild_house_bans FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_guild_house_bans TO oteryn_game_runtime;

-- The original Bank checks remain intact in their ordinary arms.
ALTER TABLE game_guild_bank_operations
 ADD COLUMN hall_house TEXT,
 ADD COLUMN hall_auction UUID REFERENCES game_house_auctions,
 ADD COLUMN hall_guild_before BIGINT NOT NULL DEFAULT 0,
 ADD COLUMN hall_guild_after BIGINT NOT NULL DEFAULT 0,
 ADD COLUMN hall_account_before BIGINT NOT NULL DEFAULT 0,
 ADD COLUMN hall_account_after BIGINT NOT NULL DEFAULT 0,
 ADD COLUMN hall_burn BIGINT NOT NULL DEFAULT 0;
DO $$ DECLARE c RECORD; expr TEXT; BEGIN
 FOR c IN SELECT conname,pg_get_expr(conbin,conrelid) AS expr FROM pg_constraint
  WHERE conrelid='game_guild_bank_operations'::regclass AND contype='c' LOOP
  expr:=c.expr;
  IF expr LIKE '%kind%GUILD_DEPOSIT%' AND expr NOT LIKE '%result%' THEN
   EXECUTE format('ALTER TABLE game_guild_bank_operations DROP CONSTRAINT %I',c.conname);
   EXECUTE format('ALTER TABLE game_guild_bank_operations ADD CONSTRAINT %I CHECK((%s) OR kind IN(''GUILDHALL_BID_RESERVE'',''GUILDHALL_BID_REFUND'',''GUILDHALL_PAYMENT''))',c.conname,expr);
  ELSIF expr LIKE '%result%GUILD_DEPOSIT%' THEN
   EXECUTE format('ALTER TABLE game_guild_bank_operations DROP CONSTRAINT %I',c.conname);
   EXECUTE format('ALTER TABLE game_guild_bank_operations ADD CONSTRAINT %I CHECK(CASE WHEN kind LIKE ''GUILDHALL_%%'' THEN result=1 ELSE (%s) END)',c.conname,expr);
  ELSIF expr LIKE '%GUILD_DISBAND_PAYOUT%game_session_id%' OR expr LIKE '%game_session_id%GUILD_DISBAND_PAYOUT%' THEN
   EXECUTE format('ALTER TABLE game_guild_bank_operations DROP CONSTRAINT %I',c.conname);
   EXECUTE format('ALTER TABLE game_guild_bank_operations ADD CONSTRAINT %I CHECK(kind LIKE ''GUILDHALL_%%'' OR (%s))',c.conname,expr);
  END IF;
 END LOOP;
END $$;
ALTER TABLE game_guild_bank_operations ADD CONSTRAINT game_guild_bank_hall_conservation CHECK(
 CASE WHEN kind LIKE 'GUILDHALL_%' THEN hall_house IS NOT NULL AND hall_auction IS NOT NULL
  AND hall_guild_before BETWEEN 0 AND 999999999999 AND hall_guild_after BETWEEN 0 AND 999999999999
  AND hall_account_before BETWEEN 0 AND 999999999999 AND hall_account_after BETWEEN 0 AND 999999999999
  AND hall_burn BETWEEN 0 AND 999999999999
  AND guild_before+bank_before+hall_guild_before+hall_account_before=
      guild_after+bank_after+hall_guild_after+hall_account_after+hall_burn
  AND amount=abs(guild_after-guild_before) AND claim_before=0 AND claim_after=0
  AND CASE kind WHEN 'GUILDHALL_BID_RESERVE' THEN hall_burn=0 AND
    guild_before-guild_after=hall_guild_after-hall_guild_before AND bank_before-bank_after=hall_account_after-hall_account_before
    AND guild_after<=guild_before AND bank_after<=bank_before
   WHEN 'GUILDHALL_BID_REFUND' THEN hall_burn=0 AND
    guild_after-guild_before=hall_guild_before-hall_guild_after AND bank_after-bank_before=hall_account_before-hall_account_after
    AND guild_after>=guild_before AND bank_after>=bank_before
   ELSE guild_before+hall_guild_before>=guild_after+hall_guild_after AND bank_before+hall_account_before>=bank_after+hall_account_after END
 ELSE hall_house IS NULL AND hall_auction IS NULL AND hall_guild_before=0 AND hall_guild_after=0
  AND hall_account_before=0 AND hall_account_after=0 AND hall_burn=0 END);
DO $$ DECLARE expr TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint WHERE conrelid='game_guild_bank_entries'::regclass AND conname='game_guild_bank_entries_kind_check';
 ALTER TABLE game_guild_bank_entries DROP CONSTRAINT game_guild_bank_entries_kind_check;
 EXECUTE format('ALTER TABLE game_guild_bank_entries ADD CONSTRAINT game_guild_bank_entries_kind_check CHECK((kind IN(''GUILDHALL_BID_RESERVE'',''GUILDHALL_BID_REFUND'',''GUILDHALL_PAYMENT'')) OR (%s))',expr);
END $$;
DO $$ DECLARE expr TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint WHERE conrelid='game_account_bank_entries'::regclass AND conname='game_account_bank_entries_guild_kind_check';
 ALTER TABLE game_account_bank_entries DROP CONSTRAINT game_account_bank_entries_guild_kind_check;
 EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT game_account_bank_entries_guild_kind_check CHECK((guild_kind IN(''GUILDHALL_BID_RESERVE'',''GUILDHALL_BID_REFUND'',''GUILDHALL_PAYMENT'')) OR (%s))',expr);
END $$;
DO $$ DECLARE expr TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint WHERE conrelid='game_guild_bank_entries'::regclass AND conname='game_guild_bank_entries_check1';
 ALTER TABLE game_guild_bank_entries DROP CONSTRAINT game_guild_bank_entries_check1;
 EXECUTE format('ALTER TABLE game_guild_bank_entries ADD CONSTRAINT game_guild_bank_entries_check1 CHECK(CASE WHEN kind LIKE ''GUILDHALL_%%'' THEN CASE WHEN kind=''GUILDHALL_PAYMENT'' THEN abs(balance_after-balance_before)=amount ELSE balance_after=balance_before+CASE kind WHEN ''GUILDHALL_BID_REFUND'' THEN amount ELSE -amount END END ELSE (%s) END)',expr);
END $$;
DO $$ DECLARE expr TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint WHERE conrelid='game_account_bank_entries'::regclass AND conname='game_account_bank_entries_house_value';
 ALTER TABLE game_account_bank_entries DROP CONSTRAINT game_account_bank_entries_house_value;
 EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT game_account_bank_entries_house_value CHECK(CASE WHEN guild_kind LIKE ''GUILDHALL_%%'' THEN escrow_before IS NULL AND escrow_after IS NULL AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND CASE WHEN guild_kind=''GUILDHALL_PAYMENT'' THEN abs(balance_after-balance_before)=amount ELSE balance_after=balance_before+CASE guild_kind WHEN ''GUILDHALL_BID_REFUND'' THEN amount ELSE -amount END END ELSE (%s) END)',expr);
END $$;
CREATE OR REPLACE FUNCTION game_house_property_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'House properties are never deleted' USING ERRCODE='23514';END IF;
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key,NEW.kind)<>(OLD.world_id,OLD.house_key,OLD.kind)
  OR NEW.revision<>OLD.revision+1 OR NEW.acl_revision<OLD.acl_revision
  OR (OLD.state=6 AND NEW.state<>6)
  OR NEW.ownership_generation<>OLD.ownership_generation+
   ((NEW.owner_character_id,NEW.owner_account_id,NEW.owner_guild_id) IS DISTINCT FROM (OLD.owner_character_id,OLD.owner_account_id,OLD.owner_guild_id))::integer
  OR ((OLD.owner_character_id IS NOT NULL OR OLD.owner_guild_id IS NOT NULL) AND NEW.owner_character_id IS NULL AND NEW.owner_guild_id IS NULL
   AND NEW.acquisition_receipt_ref IS DISTINCT FROM OLD.acquisition_receipt_ref)) THEN
  RAISE EXCEPTION 'House property requires an exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_house_owner_in_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.owner_character_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_character_roots WHERE character_id=NEW.owner_character_id
   AND world_id=NEW.world_id AND lifecycle=1) THEN
  RAISE EXCEPTION 'House owner must be an active Character of this World' USING ERRCODE='23514';
 END IF;
 IF NEW.owner_guild_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.owner_guild_id AND world_id=NEW.world_id AND state IN(1,2,3)) THEN
  RAISE EXCEPTION 'Guild owner World mismatch' USING ERRCODE='23514'; END IF;
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key)<>(OLD.world_id,OLD.house_key)
  OR NEW.ownership_revision<>OLD.ownership_revision+1
  OR (NEW.owner_character_id IS NULL AND NEW.owner_guild_id IS NULL AND NEW.acquisition_receipt_ref<>OLD.acquisition_receipt_ref)
  OR ((NEW.owner_character_id IS NOT NULL OR NEW.owner_guild_id IS NOT NULL) AND NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref)) THEN
  RAISE EXCEPTION 'House ownership requires an exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_house_property_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_house_properties%ROWTYPE; a game_house_ownership%ROWTYPE; w UUID; h TEXT;
BEGIN
 -- Slot re-keying must check both the old and the new house.
 FOR w,h IN SELECT DISTINCT x.world_id,x.house_key FROM (
  SELECT OLD.world_id,OLD.house_key WHERE TG_OP<>'INSERT'
  UNION ALL SELECT NEW.world_id,NEW.house_key WHERE TG_OP<>'DELETE') x LOOP
  SELECT * INTO p FROM game_house_properties WHERE world_id=w AND house_key=h;
  SELECT * INTO a FROM game_house_ownership WHERE world_id=w AND house_key=h;
 IF p.world_id IS NULL OR a.world_id IS NULL OR (a.owner_character_id,a.owner_guild_id) IS DISTINCT FROM (p.owner_character_id,p.owner_guild_id)
   OR a.ownership_revision<>p.ownership_generation
   OR (p.acquisition_receipt_ref IS NOT NULL AND a.acquisition_receipt_ref<>p.acquisition_receipt_ref)
   OR (p.owner_guild_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=p.owner_guild_id AND g.world_id=w AND g.state IN(1,2,3)))
   OR (p.owner_character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_roots r
    WHERE r.character_id=p.owner_character_id AND r.account_id=p.owner_account_id AND r.world_id=w AND r.lifecycle=1))
   OR (SELECT count(*) FROM game_account_housing_slots s WHERE s.world_id=w AND s.house_key=h)<>
    (p.owner_account_id IS NOT NULL)::integer
   OR EXISTS(SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=w AND s.house_key=h
    AND s.account_id IS DISTINCT FROM p.owner_account_id) THEN
   RAISE EXCEPTION 'House canonical owner, legacy anchor and slot disagree' USING ERRCODE='23514';
  END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_account_bank_credit_headroom(p_account UUID,p_world UUID)
RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT greatest(0,9000000000000000-coalesce((SELECT balance FROM game_account_bank_balances
  WHERE account_id=p_account AND world_id=p_world),0)-coalesce((SELECT sum(escrow_gold-escrow_guild_gold)
  FROM game_house_bids WHERE account_id=p_account AND world_id=p_world AND state=1),0)
  -coalesce((SELECT sum(escrow_gold+held_credit_gold) FROM game_market_offers WHERE account_id=p_account AND world_id=p_world AND (state=1 OR held_credit_gold>0)),0))::bigint
$$;
CREATE OR REPLACE FUNCTION game_house_headroom_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1 AND guild_id IS NULL)
  AND EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=NEW.account_id AND world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'House bid and personal slot are mutually exclusive' USING ERRCODE='23514';END IF;
 IF coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=NEW.account_id AND world_id=NEW.world_id),0)
  +coalesce((SELECT sum(escrow_gold-escrow_guild_gold) FROM game_house_bids WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND state=1),0)
  +coalesce((SELECT sum(escrow_gold+held_credit_gold) FROM game_market_offers WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND (state=1 OR held_credit_gold>0)),0)>9000000000000000 THEN
  RAISE EXCEPTION 'HOUSE_BANK_HEADROOM' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;

CREATE FUNCTION game_guildhall_custody_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE g UUID; w UUID;
BEGIN
 g:=NEW.guild_id;
 IF g IS NULL THEN RETURN NULL; END IF;
 IF coalesce((SELECT balance FROM game_guild_bank_balances WHERE guild_id=g),0)+
  coalesce((SELECT sum(escrow_guild_gold) FROM game_house_bids WHERE guild_id=g AND state=1),0)>999999999999 THEN
  RAISE EXCEPTION 'GUILDHALL_GUILD_HEADROOM' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='game_house_bids' THEN
  w:=NEW.world_id;
  IF NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=g AND world_id=w)
   OR NOT EXISTS(SELECT 1 FROM game_house_auctions a JOIN game_house_properties p USING(world_id,house_key)
    WHERE a.auction_id=NEW.auction_id AND a.world_id=w AND p.kind='guildhall')
   OR (NEW.state=1 AND EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=g)) THEN
   RAISE EXCEPTION 'GUILDHALL_CUSTODY_BINDING' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guildhall_bid_custody AFTER INSERT OR UPDATE ON game_house_bids
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guildhall_custody_proven();
CREATE CONSTRAINT TRIGGER game_guildhall_bank_custody AFTER INSERT OR UPDATE ON game_guild_bank_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guildhall_custody_proven();

-- The House participant shares the existing value audit outbox. Account events
-- exist only when that Account changes; Guild and House events always exist.
ALTER TABLE game_guild_bank_event_outbox DROP CONSTRAINT game_guild_bank_event_outbox_owner_check,
 ADD CONSTRAINT game_guild_bank_event_outbox_owner_check CHECK(owner IN('BANK','GUILD','HOUSE'));
DO $$ DECLARE definition TEXT; BEGIN
 SELECT pg_get_functiondef('game_guild_bank_operation_proven()'::regprocedure) INTO definition;
 definition:=replace(definition,
  'WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN 1 ELSE 2 END',
  'WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN 1 WHEN o.kind LIKE ''GUILDHALL_%'' AND o.bank_before<>o.bank_after THEN 3 ELSE 2 END');
 definition:=replace(definition,
  'CASE WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN ARRAY[''BANK''] ELSE ARRAY[''BANK'',''GUILD''] END',
  'CASE WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN ARRAY[''BANK''] WHEN o.kind LIKE ''GUILDHALL_%'' THEN CASE WHEN o.bank_before<>o.bank_after THEN ARRAY[''BANK'',''GUILD'',''HOUSE''] ELSE ARRAY[''GUILD'',''HOUSE''] END ELSE ARRAY[''BANK'',''GUILD''] END');
 EXECUTE definition;
END $$;
-- Private participant: native Bank operation, Guild chain and Account chain are
-- committed with split custody. It has no runtime EXECUTE or table-write grant.
CREATE FUNCTION game_guild_bank_apply_hall(p JSONB) RETURNS VOID
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE g game_guild_bank_balances; b game_account_bank_balances; o UUID:=(p->>'operation')::uuid;
 gid UUID:=(p->>'guild')::uuid; acct UUID:=(p->>'account')::uuid; w UUID:=(p->>'world')::uuid;
 actor UUID:=(p->>'actor')::uuid; k TEXT:=p->>'kind'; ge UUID; be UUID; own TEXT; binding BYTEA;
 ga BIGINT:=(p->>'guild_after')::bigint; ba BIGINT:=(p->>'bank_after')::bigint;
BEGIN
 IF k NOT IN('GUILDHALL_BID_RESERVE','GUILDHALL_BID_REFUND','GUILDHALL_PAYMENT') THEN
  RAISE EXCEPTION 'GUILDHALL_BANK_KIND' USING ERRCODE='23514'; END IF;
 INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(gid,0) ON CONFLICT DO NOTHING;
 SELECT * INTO g FROM game_guild_bank_balances WHERE guild_id=gid FOR UPDATE;
 INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(acct,w,0) ON CONFLICT DO NOTHING;
 SELECT * INTO b FROM game_account_bank_balances WHERE account_id=acct AND world_id=w FOR UPDATE;
 SELECT decode(string_agg(lpad(to_hex(value::integer),2,'0'),'' ORDER BY ord),'hex') INTO binding
  FROM jsonb_array_elements_text(p->'binding') WITH ORDINALITY AS x(value,ord);
 INSERT INTO game_guild_bank_operations(operation_id,guild_id,world_id,account_id,actor,kind,request_binding,
  game_session_id,command_id,guild_revision,amount,result,bank_before,bank_after,guild_before,guild_after,
  claim_before,claim_after,reply,occurred_at,hall_house,hall_auction,hall_guild_before,hall_guild_after,
  hall_account_before,hall_account_after,hall_burn)
 SELECT o,gid,w,acct,actor,k,binding,(p->>'session')::uuid,(p->>'command')::numeric,
  revision,abs(ga-g.balance),1,b.balance,ba,g.balance,ga,0,0,'\x0801'::bytea,
  (extract(epoch FROM clock_timestamp())*1000)::bigint,p->>'house',(p->>'auction')::uuid,
  (p->>'escrow_guild_before')::bigint,(p->>'escrow_guild_after')::bigint,
  (p->>'escrow_account_before')::bigint,(p->>'escrow_account_after')::bigint,(p->>'burn')::bigint
  FROM game_guilds WHERE guild_id=gid;
 IF ga<>g.balance THEN
  ge:=game_character_uuid_v7();
  INSERT INTO game_guild_bank_entries(entry_id,operation_id,guild_id,ordinal,kind,amount,balance_before,balance_after,previous_entry_id)
   VALUES(ge,o,gid,g.ordinal+1,k,abs(ga-g.balance),g.balance,ga,g.last_entry_id);
  UPDATE game_guild_bank_balances SET balance=ga,last_entry_id=ge,ordinal=g.ordinal+1 WHERE guild_id=gid;
 END IF;
 IF ba<>b.balance THEN
  be:=game_character_uuid_v7();
  INSERT INTO game_account_bank_entries(entry_id,guild_operation_id,guild_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
   VALUES(be,o,k,acct,w,NULL,abs(ba-b.balance),b.balance,ba,b.last_entry_id,actor);
  UPDATE game_account_bank_balances SET balance=ba,last_entry_id=be WHERE account_id=acct AND world_id=w;
 END IF;
 FOREACH own IN ARRAY CASE WHEN ba<>b.balance THEN ARRAY['BANK','GUILD','HOUSE'] ELSE ARRAY['GUILD','HOUSE'] END LOOP
  INSERT INTO game_guild_bank_event_outbox(operation_id,owner,payload,payload_sha256,occurred_at,expires_at)
   SELECT o,own,convert_to(game_guild_bank_payload(x,own)::text,'UTF8'),sha256(convert_to(game_guild_bank_payload(x,own)::text,'UTF8')),
    occurred_at,occurred_at+2592000000 FROM game_guild_bank_operations x WHERE operation_id=o;
 END LOOP;
END $$;
CREATE OR REPLACE FUNCTION game_guild_bank_payload(o game_guild_bank_operations,own TEXT) RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$
 SELECT jsonb_build_object('version',1,'class',CASE WHEN o.hall_burn>0 THEN 'BURN' ELSE 'TRANSFER' END,'kind',o.kind,'owner',own,
  'guild',o.guild_id,'account',o.account_id,'world',o.world_id,'actor',o.actor,
  'bank_before',o.bank_before,'bank_after',o.bank_after,'guild_before',o.guild_before,'guild_after',o.guild_after,
  'claim_before',o.claim_before,'claim_after',o.claim_after,'amount',o.amount)
  ||CASE WHEN o.hall_house IS NULL THEN '{}'::jsonb ELSE jsonb_build_object('house',o.hall_house,'auction',o.hall_auction,
   'escrow_guild_before',o.hall_guild_before,'escrow_guild_after',o.hall_guild_after,
   'escrow_account_before',o.hall_account_before,'escrow_account_after',o.hall_account_after,'burn',o.hall_burn) END
$$;

CREATE FUNCTION game_guildhall_bid_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE gid UUID; w UUID:=(p->>'world')::uuid; actor UUID:=(p->>'actor')::uuid;
 sid UUID:=(p->>'session')::uuid; cid NUMERIC:=(p->>'command')::numeric;
 prop game_house_properties; a game_house_auctions; b game_house_bids;
 gb BIGINT; ab BIGINT; eg BIGINT; ea BIGINT; target BIGINT; oldg BIGINT; olda BIGINT;
 acct UUID; funding UUID; funding_actor UUID; mx BIGINT:=(p->>'maximum')::bigint;
 old game_guild_bank_operations; binding BYTEA; digest BYTEA; price BIGINT; reached BIGINT; k TEXT;
BEGIN
 IF mode NOT IN('prepare','apply') OR p IS NULL OR octet_length(p::text)>4096 OR mx IS NULL OR mx NOT BETWEEN 0 AND 999999999999
  OR NOT game_character_is_uuid_v7((p->>'operation')::uuid)
  OR jsonb_array_length(p->'binding') IS DISTINCT FROM 32 OR jsonb_array_length(p->'content_digest') IS DISTINCT FROM 32
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'GUILDHALL_INPUT' USING ERRCODE='23514'; END IF;
 SELECT decode(string_agg(lpad(to_hex(value::integer),2,'0'),'' ORDER BY ord),'hex') INTO binding
  FROM jsonb_array_elements_text(p->'binding') WITH ORDINALITY AS x(value,ord);
 SELECT decode(string_agg(lpad(to_hex(value::integer),2,'0'),'' ORDER BY ord),'hex') INTO digest
  FROM jsonb_array_elements_text(p->'content_digest') WITH ORDINALITY AS x(value,ord);
 SELECT * INTO old FROM game_guild_bank_operations WHERE operation_id=(p->>'operation')::uuid OR (game_session_id=sid AND command_id=cid);
 IF FOUND THEN
  IF old.request_binding<>binding OR old.hall_house IS DISTINCT FROM p->>'house' THEN RETURN jsonb_build_object('refusal','conflict'); END IF;
  RETURN jsonb_build_object('replayed',TRUE);
 END IF;
 -- Independent live session proof at the native mutation boundary, including
 -- Character and Account admission guards, assignment and every generation.
 IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
  JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id
  JOIN game_durability_admission_account_guards ag ON ag.account_id=s.account_id
  JOIN game_runtime_scope_assignments r ON r.world_id=s.world_id AND r.channel_id=s.runtime_scope_channel_id
  WHERE s.game_session_id=sid AND s.character_id=actor AND s.world_id=w AND s.runtime_scope_kind=1 AND s.session_state IN(1,2)
   AND s.current_generation=(p->>'connection')::numeric AND s.character_lease_generation=(p->>'lease')::numeric
   AND s.scope_ownership_generation=(p->>'scope_generation')::numeric AND cg.eligible
   AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=sid
   AND ag.holder_game_session_id=sid AND ag.presence_character_id=actor AND r.scope_kind=1 AND r.state=1
   AND r.holder_node_id=n AND r.holder_registration_revision=nr AND r.ownership_generation=s.scope_ownership_generation
   FOR SHARE OF s,cg,ag,r) THEN RETURN jsonb_build_object('refusal','authority'); END IF;
 SELECT account_id INTO acct FROM game_character_roots WHERE character_id=actor AND lifecycle=1 AND world_id=w FOR UPDATE;
 SELECT guild_id INTO gid FROM game_guild_members WHERE character_id=actor;
 IF gid IS NULL THEN RETURN jsonb_build_object('refusal','leader'); END IF;
 PERFORM 1 FROM game_guilds WHERE guild_id=gid AND world_id=w AND state=2 FOR UPDATE;
 IF NOT FOUND THEN RETURN jsonb_build_object('refusal','inactive'); END IF;
 PERFORM 1 FROM game_guild_members WHERE guild_id=gid AND character_id=actor AND rank_level=1 FOR UPDATE;
 IF NOT FOUND THEN RETURN jsonb_build_object('refusal','leader'); END IF;
 IF EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=gid) THEN RETURN jsonb_build_object('refusal','hall'); END IF;
 IF EXISTS(SELECT 1 FROM game_guild_house_bans WHERE guild_id=gid AND banned_until>clock_timestamp()) THEN RETURN jsonb_build_object('refusal','banned'); END IF;
 -- The server catalogue supplies kind/rent and the current runtime pin. A
 -- client supplies only the House key; activation shares the fenced bid transaction.
 INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
  VALUES(w,p->>'house','guildhall',(p->>'catalogue_rent')::bigint,digest) ON CONFLICT DO NOTHING;
 INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
  VALUES(w,p->>'house',1,game_character_uuid_v7()) ON CONFLICT DO NOTHING;
 SELECT * INTO prop FROM game_house_properties WHERE world_id=w AND house_key=p->>'house' FOR UPDATE;
 IF prop.kind IS DISTINCT FROM 'guildhall' OR prop.content_digest IS DISTINCT FROM digest OR prop.state NOT IN(1,2) THEN RETURN jsonb_build_object('refusal','catalogue'); END IF;
 SELECT * INTO a FROM game_house_auctions WHERE world_id=w AND house_key=prop.house_key AND NOT settled FOR UPDATE;
 IF NOT FOUND THEN
  IF prop.state<>1 THEN RETURN jsonb_build_object('refusal','closed'); END IF;
  INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at)
   VALUES(game_character_uuid_v7(),w,prop.house_key,prop.rent_gold,clock_timestamp()+interval '7 days') RETURNING * INTO a;
  UPDATE game_house_properties SET state=2,revision=revision+1 WHERE world_id=w AND house_key=prop.house_key;
 END IF;
 IF clock_timestamp()>=a.ends_at THEN RETURN jsonb_build_object('refusal','closed'); END IF;
 IF EXISTS(SELECT 1 FROM game_house_bids WHERE guild_id=gid AND state=1 AND auction_id<>a.auction_id) THEN RETURN jsonb_build_object('refusal','bid'); END IF;
 SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND guild_id=gid FOR UPDATE;
 IF b.auction_id IS NOT NULL AND b.state<>1 THEN RETURN jsonb_build_object('refusal','closed'); END IF;
 IF b.auction_id IS NULL AND (SELECT count(*) FROM (SELECT 1 FROM game_house_bids WHERE auction_id=a.auction_id LIMIT 257) bounded)>=256 THEN RETURN jsonb_build_object('refusal','full'); END IF;
 SELECT CASE count(*) WHEN 0 THEN 0 WHEN 1 THEN 0 ELSE least(max(maximum),min(maximum)+1) END INTO price
  FROM (SELECT maximum FROM game_house_bids WHERE auction_id=a.auction_id AND state=1 ORDER BY maximum DESC,reached_order LIMIT 2) top;
 IF mx<price OR mx+a.first_rent>999999999999 THEN RETURN jsonb_build_object('refusal','balance'); END IF;
 funding:=coalesce(b.account_id,acct);funding_actor:=coalesce(b.character_id,actor);
 oldg:=coalesce(b.escrow_guild_gold,0);olda:=coalesce(b.escrow_gold-b.escrow_guild_gold,0);target:=mx+a.first_rent;
 INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(gid,0) ON CONFLICT DO NOTHING;
 SELECT balance INTO gb FROM game_guild_bank_balances WHERE guild_id=gid FOR UPDATE;
 INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(funding,w,0) ON CONFLICT DO NOTHING;
 SELECT balance INTO ab FROM game_account_bank_balances WHERE account_id=funding AND world_id=w FOR UPDATE;
 IF mode='prepare' THEN RETURN jsonb_build_object('guild_balance',gb,'account_balance',ab,'escrow_guild',oldg,'escrow_account',olda,
  'rank',1,'active',TRUE,'has_hall',EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=gid),
  'has_other_bid',EXISTS(SELECT 1 FROM game_house_bids WHERE guild_id=gid AND state=1 AND auction_id<>a.auction_id),
  'personal_houses',(SELECT count(*) FROM game_account_housing_slots WHERE account_id=acct AND world_id=w),
  'rent',a.first_rent,'existing',b.auction_id IS NOT NULL,'same_leader',funding=acct AND funding_actor=actor); END IF;
 eg:=(p->>'escrow_guild')::bigint;ea:=(p->>'escrow_account')::bigint;
 -- Recompute the pure policy independently: Guild-first debit, Account-first return.
 IF target>oldg+olda THEN
  IF eg IS DISTINCT FROM oldg+least(target-oldg-olda,gb) OR ea IS DISTINCT FROM target-eg
   OR ea-olda>ab OR (ea>olda AND (funding<>acct OR funding_actor<>actor)) THEN RAISE EXCEPTION 'GUILDHALL_RESERVE_PLAN' USING ERRCODE='23514'; END IF;
  k:='GUILDHALL_BID_RESERVE';
 ELSE
  IF ea IS DISTINCT FROM greatest(0,olda-(oldg+olda-target)) OR eg IS DISTINCT FROM target-ea THEN RAISE EXCEPTION 'GUILDHALL_REFUND_PLAN' USING ERRCODE='23514'; END IF;
  k:='GUILDHALL_BID_REFUND';
 END IF;
 reached:=CASE WHEN b.maximum=mx THEN b.reached_order ELSE a.next_order END;
 PERFORM game_guild_bank_apply_hall(p||jsonb_build_object('guild',gid,'account',funding,'actor',funding_actor,'kind',k,'auction',a.auction_id,
  'guild_after',gb+oldg-eg,'bank_after',ab+olda-ea,'escrow_guild_before',oldg,'escrow_guild_after',eg,
  'escrow_account_before',olda,'escrow_account_after',ea,'burn',0));
 IF b.auction_id IS NULL THEN
 INSERT INTO game_house_bids(auction_id,account_id,world_id,character_id,maximum,first_rent,reached_order,escrow_gold,guild_id,escrow_guild_gold,guildhall_operation)
  VALUES(a.auction_id,funding,w,funding_actor,mx,a.first_rent,reached,eg+ea,gid,eg,(p->>'operation')::uuid)
 ;
 ELSE
  UPDATE game_house_bids SET maximum=mx,reached_order=reached,escrow_gold=eg+ea,escrow_guild_gold=eg,
   guildhall_revision=guildhall_revision+1,guildhall_operation=(p->>'operation')::uuid
   WHERE auction_id=a.auction_id AND guild_id=gid;
 END IF;
 UPDATE game_house_auctions SET next_order=greatest(next_order,reached+1) WHERE auction_id=a.auction_id;
 RETURN jsonb_build_object('applied',TRUE);
END $$;

DO $$ DECLARE f RECORD; BEGIN
 FOR f IN SELECT oid::regprocedure AS sig FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND
  proname IN('game_guild_bank_apply_hall','game_guildhall_bid_port','game_guildhall_custody_proven',
   'game_guild_bank_payload','game_house_property_transition','game_house_owner_in_world','game_house_property_proven',
   'game_account_bank_credit_headroom','game_house_headroom_proven') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f.sig,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.sig);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_guildhall_bid_port(TEXT,JSONB,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;

-- Resolve the live leader at every permission check. ACL and item custody stay
-- attached to the Guild property across leadership changes.
CREATE FUNCTION game_house_effective_owner(w UUID,h TEXT) RETURNS UUID LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT coalesce(p.owner_character_id,(SELECT character_id FROM game_guild_members m JOIN game_guilds g USING(guild_id)
  WHERE m.guild_id=p.owner_guild_id AND m.rank_level=1 AND g.world_id=w AND g.state IN(1,2)))
 FROM game_house_properties p WHERE p.world_id=w AND p.house_key=h
$$;
ALTER FUNCTION game_house_access(UUID,TEXT,UUID) RENAME TO game_house_personal_access;
CREATE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID)
RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB)
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_house_properties; gid UUID; rev NUMERIC; rank SMALLINT; grant_role TEXT;
BEGIN
 SELECT owner_guild_id INTO gid FROM game_house_properties WHERE world_id=p_world_id AND house_key=p_house_key;
 IF gid IS NULL THEN RETURN QUERY SELECT * FROM game_house_personal_access(p_world_id,p_house_key,p_character_id); RETURN; END IF;
 -- Same order as Guild mutation, then property. Recheck the untrusted discovery.
 SELECT revision INTO rev FROM game_guilds WHERE guild_id=gid AND world_id=p_world_id AND state IN(1,2) FOR SHARE;
 IF NOT FOUND THEN RETURN; END IF;
 SELECT rank_level INTO rank FROM game_guild_members WHERE guild_id=gid AND character_id=p_character_id FOR SHARE;
 SELECT * INTO p FROM game_house_properties WHERE world_id=p_world_id AND house_key=p_house_key FOR SHARE;
 IF p.owner_guild_id IS DISTINCT FROM gid OR p.state NOT IN(3,4) THEN RETURN; END IF;
 grant_role:=game_house_access_grant(p_world_id,p_house_key,p_character_id);
 grant_role:=CASE WHEN rank=1 THEN 'OWNER' WHEN rank=2 OR grant_role='SUBOWNER' THEN 'SUBOWNER'
  WHEN rank IS NOT NULL OR grant_role='GUEST' THEN 'GUEST' ELSE NULL END;
 IF grant_role IS NULL THEN RETURN; END IF;
 RETURN QUERY SELECT grant_role,game_house_access_fence(p_world_id,p_house_key),p.acl_revision,
  CASE WHEN rank IS NULL THEN NULL::jsonb ELSE jsonb_build_object(gid::text,rev::text) END;
END $$;
CREATE OR REPLACE FUNCTION game_house_notice_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key)<>(OLD.world_id,OLD.house_key)
   OR NEW.revision<>OLD.revision+1) THEN
  RAISE EXCEPTION 'House notice requires exact successor' USING ERRCODE='23514';
 END IF;
 IF (TG_OP='INSERT' AND NEW.revision<>1) OR NOT EXISTS(
  SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.game_session_id=NEW.last_session_id AND r.command_id=NEW.last_command_id
   AND r.world_id=NEW.world_id AND r.house_key=NEW.house_key
   AND r.result_revision=NEW.revision AND r.created_xact_id=pg_current_xact_id()
   AND game_house_effective_owner(p.world_id,p.house_key)=r.character_id AND p.ownership_generation=NEW.ownership_revision
   AND p.state IN(3,4)) THEN
  RAISE EXCEPTION 'House notice lacks same-transaction owner receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_house_management_acl_advance(s UUID,c NUMERIC) RETURNS void
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_management_receipts%ROWTYPE; p game_house_properties%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_management_receipts WHERE game_session_id=s AND command_id=c;
 SELECT * INTO p FROM game_house_properties WHERE world_id=r.world_id AND house_key=r.house_key FOR UPDATE;
 IF r.game_session_id IS NULL OR r.action_kind<>1 OR r.created_xact_id<>pg_current_xact_id()
  OR p.world_id IS NULL OR p.state NOT IN(3,4) OR p.acl_revision+1<>r.result_revision
  OR (game_house_effective_owner(p.world_id,p.house_key) IS DISTINCT FROM r.character_id AND (r.list_key<>'guest' OR NOT EXISTS(
    SELECT 1 FROM game_house_access(p.world_id,p.house_key,r.character_id) WHERE role='SUBOWNER' AND NOT content_fenced))) THEN
  RAISE EXCEPTION 'House ACL lacks current owner receipt' USING ERRCODE='23514';
 END IF;
 UPDATE game_house_properties SET revision=revision+1,acl_revision=r.result_revision
  WHERE world_id=r.world_id AND house_key=r.house_key;
END $$;

ALTER TABLE game_house_dispositions ADD COLUMN owner_guild_id UUID REFERENCES game_guilds,
 ADD COLUMN released_at BIGINT,
 DROP CONSTRAINT game_house_dispositions_cause_check,
 DROP CONSTRAINT game_house_dispositions_check,
 ADD CONSTRAINT game_house_disposition_cause CHECK(cause IN('MOVE_OUT','EVICTION','GUILD_DISBAND')),
 ADD CONSTRAINT game_house_disposition_deadline CHECK(CASE WHEN cause='MOVE_OUT' THEN due_at::numeric-noticed_at BETWEEN 86400000 AND 2592000000 ELSE due_at=noticed_at END),
 ADD CONSTRAINT game_house_disposition_release CHECK(released_at IS NULL OR (state=3 AND released_at>=drained_at));
DROP INDEX game_house_disposition_live;
CREATE UNIQUE INDEX game_house_disposition_live ON game_house_dispositions(world_id,house_key) WHERE state<>4 AND released_at IS NULL;
CREATE OR REPLACE FUNCTION game_house_disposition_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'UPDATE' OR
    (to_jsonb(NEW)-ARRAY['state','cancelled_at','frozen_at','drained_at','census_count','census_digest','written_xact_id','released_at']) IS DISTINCT FROM
    (to_jsonb(OLD)-ARRAY['state','cancelled_at','frozen_at','drained_at','census_count','census_digest','written_xact_id','released_at']) OR
    NOT ((OLD.state=1 AND NEW.state IN(2,4)) OR (OLD.state=2 AND NEW.state=3) OR (OLD.state=3 AND NEW.state=3 AND OLD.released_at IS NULL AND NEW.released_at IS NOT NULL)) OR
    (OLD.state=2 AND (NEW.frozen_at,NEW.census_count,NEW.census_digest) IS DISTINCT FROM (OLD.frozen_at,OLD.census_count,OLD.census_digest)) THEN
  RAISE EXCEPTION 'immutable house disposition binding/transition' USING ERRCODE='23514';
 END IF;
 NEW.written_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_house_disposition_fence_check(w UUID,h TEXT,i UUID,removing BOOLEAN) RETURNS VOID
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_dispositions;
BEGIN
 -- Narrow native-trigger property pin: never wait after an item/provenance lock.
 PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=h FOR SHARE NOWAIT;
 -- This release participant uses the same disposition/census tables. Unknown
 -- future participants still fail closed rather than being silently bypassed.
 IF to_regclass('game_house_guild_dispositions') IS NOT NULL THEN
  RAISE EXCEPTION 'uncomposed Guild disposition participant' USING ERRCODE='23514'; END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state IN(2,3) AND released_at IS NULL;
 IF FOUND AND NOT (removing AND i IS NOT NULL AND EXISTS(
  SELECT 1 FROM game_house_disposition_steps s WHERE s.operation_id=o.operation_id
   AND s.item_instance_id=i AND s.written_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'house disposition content is frozen' USING ERRCODE='55P03';
 END IF;
END $$;
CREATE OR REPLACE FUNCTION game_house_move_out_apply(w UUID,h TEXT,c UUID,notice BOOLEAN,due BIGINT,now_ms BIGINT)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE owner_row game_house_ownership; o game_house_dispositions;
BEGIN
 IF w IS NULL OR h IS NULL OR c IS NULL OR notice IS NULL OR now_ms IS NULL THEN RETURN NULL; END IF;
 SELECT * INTO owner_row FROM game_house_ownership WHERE world_id=w AND house_key=h FOR UPDATE;
 IF NOT FOUND OR game_house_effective_owner(w,h) IS DISTINCT FROM c THEN RETURN NULL; END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state<>4 AND released_at IS NULL FOR UPDATE;
 IF notice THEN
  IF FOUND OR due IS NULL OR due::numeric-now_ms NOT BETWEEN 86400000 AND 2592000000 THEN RETURN NULL; END IF;
  INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,owner_guild_id)
   VALUES(w,h,c,owner_row.ownership_revision,1,now_ms,due,owner_row.owner_guild_id) RETURNING * INTO o;
 ELSE
  IF NOT FOUND OR o.state<>1 OR now_ms>=o.due_at OR o.ownership_revision<>owner_row.ownership_revision THEN RETURN NULL; END IF;
  UPDATE game_house_dispositions SET state=4,cancelled_at=now_ms WHERE operation_id=o.operation_id RETURNING * INTO o;
 END IF;
 RETURN jsonb_build_object('operation',o.operation_id,'state',o.state,'due',o.due_at,'count',0,'delivered',0);
END $$;
CREATE OR REPLACE FUNCTION game_house_disposition_advance(op UUID,max_census INT,max_step INT,requested_step INT) RETURNS INT
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE o game_house_dispositions; owner_row game_house_ownership; n INT; x RECORD; ord NUMERIC(20,0); now_ms BIGINT; locked_roots UUID[]:='{}';
BEGIN
 IF max_census IS DISTINCT FROM 2000 OR max_step IS DISTINCT FROM 100 OR requested_step IS NULL THEN RAISE EXCEPTION 'invalid disposition bounds' USING ERRCODE='23514'; END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op;
 IF NOT FOUND THEN RETURN 0; END IF;
 -- Census/entry lock order is shared with the native handoff producer.
 LOCK TABLE game_house_scope_handoffs IN SHARE ROW EXCLUSIVE MODE;
 -- Recipient roots precede property/items/counters. The census bound also bounds this read.
 FOR x IN SELECT DISTINCT p.reclaim_subject_character_id AS id FROM game_item_house_reclaim_provenance p
  WHERE p.world_id=o.world_id AND p.house_key=o.house_key ORDER BY id LIMIT 2001 LOOP
  PERFORM 1 FROM game_character_roots WHERE character_id=x.id FOR UPDATE;
  locked_roots:=array_append(locked_roots,x.id);
 END LOOP;
 SELECT * INTO owner_row FROM game_house_ownership WHERE world_id=o.world_id AND house_key=o.house_key FOR UPDATE;
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE world_id=o.world_id AND house_key=o.house_key
  AND NOT(reclaim_subject_character_id=ANY(locked_roots))) THEN
  RAISE EXCEPTION 'house recipients changed before property lock' USING ERRCODE='40001'; END IF;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF o.state NOT IN(1,2) OR now_ms<o.due_at THEN RETURN 0; END IF;
 IF requested_step=0 AND o.state=2 THEN RETURN 0; END IF;
 IF requested_step<0 OR requested_step>20 THEN RAISE EXCEPTION 'invalid disposition step' USING ERRCODE='23514'; END IF;
 IF (CASE WHEN o.owner_guild_id IS NULL THEN owner_row.owner_character_id IS DISTINCT FROM o.owner_character_id ELSE owner_row.owner_guild_id IS DISTINCT FROM o.owner_guild_id END) OR owner_row.ownership_revision IS DISTINCT FROM o.ownership_revision THEN
  RAISE EXCEPTION 'house disposition ownership changed' USING ERRCODE='23514'; END IF;
 IF o.state=1 THEN
  PERFORM game_house_disposition_fence_check(o.world_id,o.house_key,NULL,FALSE);
  IF requested_step<>0 THEN RAISE EXCEPTION 'freeze step required' USING ERRCODE='23514'; END IF;
  SELECT count(*) INTO n FROM (SELECT item_instance_id FROM game_item_house_interior_locations
   WHERE world_id=o.world_id AND house_key=o.house_key ORDER BY item_instance_id LIMIT 2001) bounded;
  IF n>max_census THEN RAISE EXCEPTION 'house census exceeds 2000' USING ERRCODE='54000'; END IF;
  -- All writers pin the property; after this lock the bounded census cannot change.
  INSERT INTO game_house_disposition_census
   SELECT op,l.item_instance_id,p.reclaim_subject_character_id,to_jsonb(l),to_jsonb(p),to_jsonb(i)
   FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id)
   WHERE l.world_id=o.world_id AND l.house_key=o.house_key ORDER BY l.item_instance_id LIMIT 2000;
  IF (SELECT count(*) FROM game_house_disposition_census WHERE operation_id=op)<>n THEN
   RAISE EXCEPTION 'house census missing provenance' USING ERRCODE='23514'; END IF;
  UPDATE game_house_dispositions SET state=2,frozen_at=now_ms,census_count=n,
   census_digest=(SELECT sha256(coalesce(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea))
    FROM game_house_disposition_census WHERE operation_id=op) WHERE operation_id=op;
  PERFORM 1 FROM game_house_bed_revisions WHERE world_id=o.world_id AND house_key=o.house_key FOR UPDATE;
  UPDATE game_house_bed_sleepers SET freed_at=greatest(now_ms,slept_at),freed_cause='disposition'
   WHERE world_id=o.world_id AND house_key=o.house_key AND freed_at IS NULL;
  -- A negative result denotes this transaction's checkpoint without an item delivery.
  RETURN -1;
 END IF;
 IF EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE world_id=o.world_id AND runtime_scope_kind=2
   AND runtime_scope_house_key=o.house_key AND session_state IN(1,2)) OR
    EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=o.world_id AND house_key=o.house_key AND state IN(1,2)) THEN RETURN 0; END IF;
 IF EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=op AND step_number=requested_step) THEN RETURN 0; END IF;
 IF requested_step<>(SELECT coalesce(max(step_number),0)+1 FROM game_house_disposition_steps WHERE operation_id=op) THEN
  RAISE EXCEPTION 'nonconsecutive disposition step' USING ERRCODE='23514'; END IF;
 -- Items then inbox counters in deterministic order, before delivery itself locks a counter.
 FOR x IN SELECT c.item_instance_id FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL ORDER BY c.item_instance_id LIMIT 100 LOOP
  PERFORM 1 FROM game_item_instances WHERE item_instance_id=x.item_instance_id FOR NO KEY UPDATE;
 END LOOP;
 FOR x IN SELECT DISTINCT reclaim_subject AS id FROM (SELECT c.* FROM game_house_disposition_census c
  LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id) WHERE c.operation_id=op AND s.item_instance_id IS NULL
  ORDER BY c.item_instance_id LIMIT 100) batch ORDER BY id LOOP
  INSERT INTO game_character_inbox_counters(character_id) VALUES(x.id) ON CONFLICT DO NOTHING;
  PERFORM 1 FROM game_character_inbox_counters WHERE character_id=x.id FOR UPDATE;
 END LOOP;
 n:=0;
 FOR x IN SELECT c.* FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL ORDER BY c.item_instance_id LIMIT 100 LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id)
   JOIN game_item_instances i USING(item_instance_id) WHERE l.item_instance_id=x.item_instance_id
   AND to_jsonb(l)=x.source_location AND to_jsonb(p)=x.source_provenance AND to_jsonb(i)=x.source_item) THEN
   RAISE EXCEPTION 'house frozen source changed' USING ERRCODE='23514'; END IF;
  SELECT next_ordinal INTO ord FROM game_character_inbox_counters WHERE character_id=x.reclaim_subject;
  INSERT INTO game_house_disposition_steps VALUES(op,x.item_instance_id,x.reclaim_subject,ord,requested_step,pg_current_xact_id());
  DELETE FROM game_item_house_interior_locations WHERE item_instance_id=x.item_instance_id;
  DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=x.item_instance_id;
  PERFORM game_character_inbox_deliver(x.item_instance_id,x.reclaim_subject,o.world_id,'HOUSE_DISPOSITION',op::text);
  n:=n+1;
 END LOOP;
 IF NOT EXISTS(SELECT 1 FROM game_house_disposition_census c LEFT JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  WHERE c.operation_id=op AND s.item_instance_id IS NULL) THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=o.world_id AND house_key=o.house_key) THEN
   RAISE EXCEPTION 'house drained source remains' USING ERRCODE='23514'; END IF;
  UPDATE game_house_dispositions SET state=3,drained_at=now_ms WHERE operation_id=op;
  IF n=0 THEN RETURN -1; END IF;
 END IF;
 RETURN n;
END $$;
CREATE OR REPLACE FUNCTION game_house_move_out_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID; ch UUID; sk BYTEA; o game_house_dispositions; prior game_house_disposition_commands;
 c UUID; sid UUID; cid NUMERIC; a INT; due BIGINT; now_ms BIGINT; result JSONB; binding BYTEA;
 ids JSONB; step INT; moved INT; cur game_house_disposition_job_cursors; last_row RECORD;
BEGIN
 IF mode NOT IN('command','query','candidates','advance','audit_snapshot','audit_insert') OR p IS NULL OR jsonb_typeof(p)<>'object'
  OR octet_length(p::text)>(CASE WHEN mode='audit_insert' THEN 300000 ELSE 2048 END)
  OR NOT p ?& ARRAY['world','channel','generation']
  OR EXISTS(SELECT 1 FROM jsonb_object_keys(p) k WHERE k NOT IN('world','channel','generation','house','session','character','command','connection','lease','action','due','operation','event','now','envelope'))
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'house disposition ingress refused' USING ERRCODE='23514'; END IF;
 w:=(p->>'world')::uuid;ch:=(p->>'channel')::uuid;sk:='\x01'::bytea||uuid_send(w)||uuid_send(ch);
 IF mode='command' THEN
  sid:=(p->>'session')::uuid;cid:=(p->>'command')::numeric;binding:=convert_to(p::text,'UTF8');
  IF sid IS NULL OR cid IS NULL OR cid NOT BETWEEN 1 AND 18446744073709551615 THEN RAISE EXCEPTION 'invalid command' USING ERRCODE='23514'; END IF;
  PERFORM pg_advisory_xact_lock(hashtextextended(sid::text||':'||cid::text,178));
  SELECT * INTO prior FROM game_house_disposition_commands WHERE game_session_id=sid AND command_id=cid;
  IF FOUND THEN
   IF prior.binding<>binding THEN RETURN jsonb_build_object('conflict',TRUE); END IF;
   RETURN prior.outcome;
  END IF;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN game_durability_admission_runtime_guards rg USING(scope_key)
  WHERE ra.scope_key=sk AND ra.scope_kind=1 AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
   AND ra.ownership_generation=(p->>'generation')::numeric AND ra.holder_node_id=n AND ra.holder_registration_revision=nr
   AND rg.ready AND rg.ownership_generation=ra.ownership_generation FOR SHARE OF ra,rg) THEN RETURN NULL; END IF;
 IF mode IN('command','query') THEN
  sid:=(p->>'session')::uuid;c:=(p->>'character')::uuid;
  IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id)
    JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id
    JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
    WHERE s.game_session_id=sid AND s.character_id=c AND s.account_id=r.account_id AND s.world_id=w AND r.world_id=w AND r.lifecycle=1
     AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=w AND s.runtime_scope_channel_id=ch
     AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
     AND s.current_generation=(p->>'connection')::numeric AND s.character_lease_generation=(p->>'lease')::numeric
     AND s.scope_ownership_generation=(p->>'generation')::numeric AND s.session_state IN(1,2)
     AND cg.eligible AND cg.account_id=r.account_id AND cg.world_id=w AND cg.lease_generation=s.character_lease_generation
     AND cg.holder_game_session_id=sid AND ag.presence_character_id=c AND ag.holder_game_session_id=sid
     FOR SHARE OF s,r,cg,ag) THEN RETURN NULL; END IF;
  -- Root before property. Query pins ownership too, preventing a concurrent owner change
  -- between its authorization decision and private projection.
  PERFORM 1 FROM game_character_roots WHERE character_id=c FOR UPDATE;
  PERFORM 1 FROM game_guilds g JOIN game_house_properties hp ON hp.owner_guild_id=g.guild_id WHERE hp.world_id=w AND hp.house_key=p->>'house' FOR SHARE OF g;
  PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=p->>'house' AND game_house_effective_owner(world_id,house_key)=c FOR UPDATE;
  IF NOT FOUND THEN RETURN NULL; END IF;
  IF mode='query' THEN
   IF (p->>'action')::int IS DISTINCT FROM 0 OR (p->>'due')::bigint IS DISTINCT FROM 0 THEN RETURN NULL; END IF;
   SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND house_key=p->>'house'
    ORDER BY (state<>4) DESC,noticed_at DESC,operation_id DESC LIMIT 1;
   IF NOT FOUND THEN RETURN jsonb_build_object('state',0); END IF;
   RETURN jsonb_build_object('operation',o.operation_id,'state',o.state,'due',o.due_at,'count',coalesce(o.census_count,0),
    'delivered',(SELECT count(*) FROM game_house_disposition_steps WHERE operation_id=o.operation_id));
  END IF;
  a:=(p->>'action')::int;due:=(p->>'due')::bigint;
  IF a IS NULL OR a NOT IN(1,2) OR due IS NULL OR (a=2 AND due<>0) THEN RETURN NULL; END IF;
  now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
  result:=game_house_move_out_apply(w,p->>'house',c,a=1,CASE WHEN a=1 THEN due END,now_ms);
  IF result IS NULL THEN RETURN NULL; END IF;
  result:=result||jsonb_build_object('now',now_ms,'new',FALSE);
  INSERT INTO game_house_disposition_commands VALUES(sid,cid,binding,(result->>'operation')::uuid,result);
  RETURN result||jsonb_build_object('new',TRUE);
 ELSIF mode='candidates' THEN
  INSERT INTO game_house_disposition_job_cursors(world_id) VALUES(w) ON CONFLICT DO NOTHING;
  SELECT * INTO cur FROM game_house_disposition_job_cursors WHERE world_id=w FOR UPDATE;
  SELECT coalesce(jsonb_agg(operation_id ORDER BY due_at,operation_id),'[]'::jsonb) INTO ids FROM (
   SELECT operation_id,due_at FROM game_house_dispositions WHERE world_id=w AND state IN(1,2)
    AND due_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
    AND (cur.last_operation IS NULL OR (due_at,operation_id)>(cur.last_due,cur.last_operation))
    ORDER BY due_at,operation_id LIMIT 100) candidates;
  IF jsonb_array_length(ids)=0 AND cur.last_operation IS NOT NULL THEN
   SELECT coalesce(jsonb_agg(operation_id ORDER BY due_at,operation_id),'[]'::jsonb) INTO ids FROM (
    SELECT operation_id,due_at FROM game_house_dispositions WHERE world_id=w AND state IN(1,2)
     AND due_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint ORDER BY due_at,operation_id LIMIT 100) wrapped;
  END IF;
  IF jsonb_array_length(ids)=0 THEN UPDATE game_house_disposition_job_cursors SET last_due=NULL,last_operation=NULL WHERE world_id=w;
  ELSE
   SELECT due_at,operation_id INTO last_row FROM game_house_dispositions WHERE operation_id=(ids->>-1)::uuid;
   UPDATE game_house_disposition_job_cursors SET last_due=last_row.due_at,last_operation=last_row.operation_id WHERE world_id=w;
  END IF;
  RETURN ids;
 END IF;
 SELECT * INTO o FROM game_house_dispositions WHERE world_id=w AND operation_id=(p->>'operation')::uuid;
 IF NOT FOUND THEN RETURN NULL; END IF;
 IF mode='advance' THEN
  step:=CASE WHEN o.state=1 THEN 0 ELSE (SELECT coalesce(max(step_number),0)+1 FROM game_house_disposition_steps WHERE operation_id=o.operation_id) END;
  moved:=game_house_disposition_advance(o.operation_id,2000,100,step);
  SELECT * INTO o FROM game_house_dispositions WHERE operation_id=o.operation_id;
  RETURN to_jsonb(moved<>0);
 END IF;
 IF o.written_xact_id<>pg_current_xact_id() AND NOT EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=o.operation_id AND written_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'audit requires own physical disposition mutation' USING ERRCODE='23514'; END IF;
 IF mode='audit_snapshot' THEN
  RETURN jsonb_build_object('operation',o.operation_id,'world',w,'house',o.house_key,'owner',o.owner_character_id,
   'state',o.state,'due',o.due_at,'count',coalesce(o.census_count,0),'digest',encode(o.census_digest,'hex'),'event',game_character_uuid_v7(),
   'now',floor(extract(epoch FROM clock_timestamp())*1000)::bigint,'transfers',
   (SELECT coalesce(jsonb_agg(jsonb_build_object('item',item_instance_id,'recipient',reclaim_subject,'ordinal',ordinal::text) ORDER BY item_instance_id),'[]'::jsonb)
    FROM (SELECT * FROM game_house_disposition_steps WHERE operation_id=o.operation_id AND written_xact_id=pg_current_xact_id() ORDER BY item_instance_id LIMIT 101) batch));
 END IF;
 IF mode='audit_insert' THEN
  INSERT INTO game_house_disposition_audit_outbox(event_id,operation_id,envelope,occurred_at,expires_at,envelope_sha256)
   VALUES((p->>'event')::uuid,o.operation_id,decode(p->>'envelope','hex'),(p->>'now')::bigint,(p->>'now')::bigint+2592000000,sha256(decode(p->>'envelope','hex')));
  RETURN 'true'::jsonb;
 END IF;
 RAISE EXCEPTION 'unknown house disposition ingress' USING ERRCODE='23514';
END $$;

DO $$ DECLARE f RECORD; BEGIN
 FOR f IN SELECT oid::regprocedure AS sig FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND
  proname IN('game_house_effective_owner','game_house_access','game_house_notice_proven','game_house_management_acl_advance',
   'game_house_disposition_guard','game_house_disposition_fence_check','game_house_move_out_apply','game_house_disposition_advance','game_house_move_out_port') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f.sig,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.sig);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_house_effective_owner(UUID,TEXT),game_house_access(UUID,TEXT,UUID) TO oteryn_game_runtime;

-- Receipt-linked Hall custody cannot be rewritten, erased or resurrected.
ALTER TABLE game_house_bids ADD COLUMN guildhall_revision BIGINT NOT NULL DEFAULT 1 CHECK(guildhall_revision>0),
 ADD COLUMN guildhall_operation UUID REFERENCES game_guild_bank_operations DEFERRABLE INITIALLY DEFERRED;
CREATE FUNCTION game_guildhall_bid_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations;
BEGIN
 IF TG_OP='DELETE' THEN
  IF OLD.guild_id IS NOT NULL THEN RAISE EXCEPTION 'retained Guildhall custody' USING ERRCODE='23514'; END IF;
  RETURN OLD;
 END IF;
 IF NEW.guild_id IS NULL AND (TG_OP='INSERT' OR OLD.guild_id IS NULL) THEN RETURN NEW; END IF;
 SELECT * INTO o FROM game_guild_bank_operations WHERE operation_id=NEW.guildhall_operation;
 IF o.operation_id IS NULL OR o.created_xact_id<>pg_current_xact_id() OR o.guild_id<>NEW.guild_id
  OR o.hall_auction<>NEW.auction_id OR (o.account_id,o.actor,o.world_id)<>(NEW.account_id,NEW.character_id,NEW.world_id)
  OR (o.hall_guild_after,o.hall_account_after)<>(NEW.escrow_guild_gold,NEW.escrow_gold-NEW.escrow_guild_gold)
  OR (TG_OP='INSERT' AND (NEW.guildhall_revision<>1 OR o.hall_guild_before<>0 OR o.hall_account_before<>0))
  OR (TG_OP='UPDATE' AND ((NEW.bid_id,NEW.auction_id,NEW.guild_id,NEW.account_id,NEW.character_id,NEW.world_id,NEW.first_rent) IS DISTINCT FROM
    (OLD.bid_id,OLD.auction_id,OLD.guild_id,OLD.account_id,OLD.character_id,OLD.world_id,OLD.first_rent)
   OR OLD.state<>1 OR NEW.guildhall_revision<>OLD.guildhall_revision+1
   OR (o.hall_guild_before,o.hall_account_before)<>(OLD.escrow_guild_gold,OLD.escrow_gold-OLD.escrow_guild_gold))) THEN
  RAISE EXCEPTION 'Guildhall custody needs exact same-transaction receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guildhall_bid_transition BEFORE INSERT OR UPDATE OR DELETE ON game_house_bids
 FOR EACH ROW EXECUTE FUNCTION game_guildhall_bid_transition();
CREATE TRIGGER game_guildhall_bid_no_truncate BEFORE TRUNCATE ON game_house_bids
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Bounded existing owner-cycle discovery. Cursor movement is durable and a
-- blocked interior cannot starve other Guild properties in the World.
ALTER TABLE game_house_disposition_job_cursors ADD COLUMN last_guildhall_house TEXT;
CREATE FUNCTION game_guildhall_job_id(w UUID,h TEXT,g NUMERIC,phase TEXT,period TEXT) RETURNS UUID
 LANGUAGE sql IMMUTABLE AS $$
 SELECT encode(set_byte(set_byte(substring(sha256(convert_to(w::text||':'||h||':'||g::text||':'||phase||':'||period,'UTF8')) FROM 1 FOR 16),6,112),8,128),'hex')::uuid
$$;

CREATE FUNCTION game_guildhall_job_port(mode TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID:=(p->>'world')::uuid; h TEXT:=p->>'house'; gid UUID; x RECORD;
 prop game_house_properties; a game_house_auctions; b game_house_bids; d game_house_dispositions;
 ids JSONB; cursor_house TEXT; winner UUID; winner_max BIGINT; second_max BIGINT; price BIGINT:=0;
 leader UUID; acct UUID; gb BIGINT; ab BIGINT; burn BIGINT:=0; bg BIGINT:=0; ba BIGINT:=0;
 operation UUID; eg BIGINT; ea BIGINT; now_ms BIGINT; base JSONB; cause TEXT; locked_guilds UUID[]:='{}';
BEGIN
 IF mode NOT IN('candidates','advance','settle','disband') OR p IS NULL OR octet_length(p::text)>2048
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'GUILDHALL_JOB_AUTHORITY' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments r JOIN game_durability_admission_runtime_guards g USING(scope_key)
  WHERE r.world_id=w AND r.scope_kind=1 AND r.state=1 AND r.holder_node_id=n AND r.holder_registration_revision=nr
   AND g.ready AND g.ownership_generation=r.ownership_generation
   AND (p->>'generation' IS NULL OR r.ownership_generation=(p->>'generation')::numeric)
   AND (p->>'channel' IS NULL OR r.channel_id=(p->>'channel')::uuid)
   ORDER BY r.scope_key LIMIT 1 FOR SHARE OF r,g) THEN
  RAISE EXCEPTION 'GUILDHALL_JOB_ASSIGNMENT' USING ERRCODE='23514'; END IF;
 now_ms:=(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF mode='candidates' THEN
  INSERT INTO game_house_disposition_job_cursors(world_id) VALUES(w) ON CONFLICT DO NOTHING;
  SELECT last_guildhall_house INTO cursor_house FROM game_house_disposition_job_cursors WHERE world_id=w FOR UPDATE;
  SELECT coalesce(jsonb_agg(house_key ORDER BY house_key),'[]'::jsonb) INTO ids FROM (
   SELECT q.house_key FROM game_house_properties q WHERE q.world_id=w AND q.kind='guildhall' AND q.house_key>coalesce(cursor_house,'') AND
    ((q.owner_guild_id IS NOT NULL AND (q.paid_until<=clock_timestamp() OR q.state=5 OR EXISTS(SELECT 1 FROM game_house_dispositions dd WHERE dd.world_id=w AND dd.house_key=q.house_key AND dd.state=3 AND dd.released_at IS NULL) OR EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=q.owner_guild_id AND state=3)))
     OR EXISTS(SELECT 1 FROM game_house_auctions aa WHERE aa.world_id=w AND aa.house_key=q.house_key AND (aa.ends_at<=clock_timestamp() OR EXISTS(SELECT 1 FROM game_house_bids db JOIN game_guilds dg ON dg.guild_id=db.guild_id WHERE db.auction_id=aa.auction_id AND db.state=1 AND dg.state=3))
      AND (NOT aa.settled OR EXISTS(SELECT 1 FROM game_house_bids bb WHERE bb.auction_id=aa.auction_id AND bb.state=1))))
   ORDER BY q.house_key LIMIT 50) bounded;
  UPDATE game_house_disposition_job_cursors SET last_guildhall_house=CASE WHEN jsonb_array_length(ids)=0 THEN NULL ELSE ids->>(jsonb_array_length(ids)-1) END WHERE world_id=w;
  RETURN ids;
 END IF;
 IF mode='disband' THEN
  gid:=(p->>'guild')::uuid;
  IF NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=gid AND world_id=w AND state=3)
   OR NOT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=gid AND world_id=w AND step=1) THEN
   RAISE EXCEPTION 'GUILDHALL_DISBAND_CONTEXT' USING ERRCODE='23514'; END IF;
  SELECT aa.house_key INTO h FROM game_house_bids bb JOIN game_house_auctions aa USING(auction_id)
   WHERE bb.guild_id=gid AND bb.state=1 ORDER BY aa.house_key LIMIT 1;
  IF h IS NULL THEN SELECT house_key INTO h FROM game_house_properties WHERE owner_guild_id=gid AND world_id=w; END IF;
  IF h IS NULL THEN RETURN 'null'::jsonb; END IF;
 END IF;
 -- Discovery never grants a lock: pin roots, Guilds and members in deterministic
 -- order before the property, auction, custody and the two native balances.
 FOR x IN SELECT DISTINCT r.character_id FROM game_character_roots r JOIN game_guild_members m USING(character_id)
  WHERE m.rank_level=1 AND m.guild_id IN(
   SELECT owner_guild_id FROM game_house_properties WHERE world_id=w AND house_key=h
   UNION SELECT bb.guild_id FROM game_house_bids bb JOIN game_house_auctions aa USING(auction_id) WHERE aa.world_id=w AND aa.house_key=h AND bb.state=1)
  ORDER BY r.character_id LIMIT 257 LOOP
  PERFORM 1 FROM game_character_roots WHERE character_id=x.character_id FOR SHARE;
 END LOOP;
 FOR x IN SELECT g.guild_id FROM game_guilds g WHERE g.guild_id IN(
   SELECT owner_guild_id FROM game_house_properties WHERE world_id=w AND house_key=h
   UNION SELECT bb.guild_id FROM game_house_bids bb JOIN game_house_auctions aa USING(auction_id) WHERE aa.world_id=w AND aa.house_key=h AND bb.state=1)
  ORDER BY g.guild_id LIMIT 257 LOOP
  PERFORM 1 FROM game_guilds WHERE guild_id=x.guild_id FOR UPDATE;
  PERFORM 1 FROM game_guild_members WHERE guild_id=x.guild_id AND rank_level=1 FOR UPDATE;
  locked_guilds:=array_append(locked_guilds,x.guild_id);
 END LOOP;
 SELECT * INTO prop FROM game_house_properties WHERE world_id=w AND house_key=h AND kind='guildhall' FOR UPDATE;
 IF NOT FOUND THEN RETURN 'null'::jsonb; END IF;
 IF (prop.owner_guild_id IS NOT NULL AND NOT(prop.owner_guild_id=ANY(locked_guilds))) OR EXISTS(
  SELECT 1 FROM game_house_bids bb JOIN game_house_auctions aa USING(auction_id)
   WHERE aa.world_id=w AND aa.house_key=h AND bb.state=1 AND NOT(bb.guild_id=ANY(locked_guilds))) THEN
  RAISE EXCEPTION 'Guildhall participants changed before property lock' USING ERRCODE='40001'; END IF;
 SELECT * INTO a FROM game_house_auctions WHERE world_id=w AND house_key=h AND
  (NOT settled OR EXISTS(SELECT 1 FROM game_house_bids WHERE auction_id=game_house_auctions.auction_id AND state=1))
  ORDER BY ends_at,auction_id LIMIT 1 FOR UPDATE;
 IF mode='advance' AND a.auction_id IS NOT NULL THEN
  SELECT bb.guild_id INTO gid FROM game_house_bids bb JOIN game_guilds gg ON gg.guild_id=bb.guild_id
   WHERE bb.auction_id=a.auction_id AND bb.state=1 AND gg.state=3 ORDER BY bb.guild_id LIMIT 1;
  IF gid IS NOT NULL THEN mode:='disband'; END IF;
 END IF;
 IF a.auction_id IS NOT NULL AND (mode='disband' OR clock_timestamp()>=a.ends_at) THEN
  IF mode='disband' THEN
   SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND guild_id=gid AND state=1 FOR UPDATE;
  ELSIF NOT a.settled THEN
   SELECT bb.guild_id,bb.maximum INTO winner,winner_max FROM game_house_bids bb JOIN game_guilds g ON g.guild_id=bb.guild_id
    WHERE bb.auction_id=a.auction_id AND bb.state=1 AND g.state=2 AND g.world_id=w
     AND NOT EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=bb.guild_id)
     AND NOT EXISTS(SELECT 1 FROM game_guild_house_bans WHERE guild_id=bb.guild_id AND banned_until>clock_timestamp())
     AND EXISTS(SELECT 1 FROM game_guild_members m JOIN game_character_roots r USING(character_id) WHERE m.guild_id=bb.guild_id AND m.rank_level=1 AND r.lifecycle=1 AND r.world_id=w)
    ORDER BY bb.maximum DESC,bb.reached_order LIMIT 1;
   IF winner IS NOT NULL THEN
    SELECT maximum INTO second_max FROM game_house_bids WHERE auction_id=a.auction_id AND state=1 AND guild_id<>winner ORDER BY maximum DESC,reached_order LIMIT 1;
    price:=CASE WHEN second_max IS NULL THEN 0 ELSE least(winner_max,second_max+1) END;
    SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND guild_id=winner AND state=1 FOR UPDATE;
    burn:=price+a.first_rent;
   ELSE
    SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND state=1 ORDER BY guild_id LIMIT 1 FOR UPDATE;
   END IF;
  ELSE
   SELECT * INTO b FROM game_house_bids WHERE auction_id=a.auction_id AND state=1 ORDER BY guild_id LIMIT 1 FOR UPDATE;
  END IF;
  IF b.auction_id IS NOT NULL THEN
   eg:=b.escrow_guild_gold;ea:=b.escrow_gold-eg;bg:=least(burn,eg);ba:=burn-bg;
   IF ba>ea THEN RAISE EXCEPTION 'GUILDHALL_SETTLEMENT_CUSTODY' USING ERRCODE='23514'; END IF;
   SELECT balance INTO gb FROM game_guild_bank_balances WHERE guild_id=b.guild_id FOR UPDATE;
   SELECT balance INTO ab FROM game_account_bank_balances WHERE account_id=b.account_id AND world_id=w FOR UPDATE;
   operation:=game_guildhall_job_id(w,h,b.guildhall_revision,CASE WHEN burn>0 OR winner IS NOT NULL THEN 'winner' ELSE 'release' END,a.auction_id::text||':'||b.guild_id::text);
   base:=jsonb_build_object('operation',operation,'world',w,'house',h,'guild',b.guild_id,'account',b.account_id,'actor',b.character_id,
    'kind',CASE WHEN winner=b.guild_id THEN 'GUILDHALL_PAYMENT' ELSE 'GUILDHALL_BID_REFUND' END,'auction',a.auction_id,
    'binding',(SELECT jsonb_agg(get_byte(uuid_send(operation),i)) FROM generate_series(0,15) i)||
     (SELECT jsonb_agg(get_byte(uuid_send(operation),i)) FROM generate_series(0,15) i),
    'guild_after',gb+eg-bg,'bank_after',ab+ea-ba,'escrow_guild_before',eg,'escrow_guild_after',0,'escrow_account_before',ea,'escrow_account_after',0,'burn',burn);
   PERFORM game_guild_bank_apply_hall(base);
   UPDATE game_house_bids SET state=CASE WHEN winner=b.guild_id THEN 2 ELSE 3 END,escrow_gold=0,escrow_guild_gold=0,
    guildhall_revision=guildhall_revision+1,guildhall_operation=operation WHERE bid_id=b.bid_id;
   IF winner=b.guild_id THEN
    UPDATE game_house_properties SET state=3,owner_guild_id=winner,ownership_generation=ownership_generation+1,revision=revision+1,
     paid_until=clock_timestamp()+interval '30 days',acquisition_receipt_ref=operation WHERE world_id=w AND house_key=h;
    UPDATE game_house_ownership SET owner_guild_id=winner,ownership_revision=ownership_revision+1,acquisition_receipt_ref=operation WHERE world_id=w AND house_key=h;
   END IF;
  END IF;
  IF mode<>'disband' AND NOT a.settled THEN
   UPDATE game_house_auctions SET settled=TRUE WHERE auction_id=a.auction_id;
   IF winner IS NULL THEN UPDATE game_house_properties SET state=1,revision=revision+1 WHERE world_id=w AND house_key=h AND state=2; END IF;
  END IF;
  RETURN 'null'::jsonb;
 END IF;
 IF prop.owner_guild_id IS NULL THEN RETURN 'null'::jsonb; END IF;
 gid:=prop.owner_guild_id;
 SELECT m.character_id,r.account_id INTO leader,acct FROM game_guild_members m JOIN game_character_roots r USING(character_id)
  WHERE m.guild_id=gid AND m.rank_level=1 AND r.world_id=w;
 IF leader IS NULL THEN RAISE EXCEPTION 'GUILDHALL_LEADER_UNAVAILABLE' USING ERRCODE='23514'; END IF;
 SELECT * INTO d FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state<>4 AND released_at IS NULL FOR UPDATE;
 IF d.state=3 THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=w AND house_key=h)
   OR EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE world_id=w AND runtime_scope_kind=2 AND runtime_scope_house_key=h AND session_state IN(1,2))
   OR EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=w AND house_key=h AND state IN(1,2)) THEN RETURN 'null'::jsonb; END IF;
  UPDATE game_house_properties SET state=1,owner_guild_id=NULL,paid_until=NULL,rent_grace_until=NULL,content_fence_operation=NULL,
   ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=w AND house_key=h;
  UPDATE game_house_ownership SET owner_guild_id=NULL,ownership_revision=ownership_revision+1 WHERE world_id=w AND house_key=h;
  UPDATE game_house_dispositions SET released_at=now_ms WHERE operation_id=d.operation_id;
  DELETE FROM game_house_acl_entries WHERE world_id=w AND house_key=h;
  DELETE FROM game_house_acl WHERE world_id=w AND house_key=h;
  DELETE FROM game_house_editors WHERE world_id=w AND house_key=h;
  RETURN to_jsonb(d.operation_id::text);
 END IF;
 IF d.operation_id IS NOT NULL THEN RETURN 'null'::jsonb; END IF;
 IF EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=gid AND state=3) THEN cause:='GUILD_DISBAND';
 ELSIF prop.paid_until<=clock_timestamp() THEN
  INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(gid,0) ON CONFLICT DO NOTHING;
  SELECT balance INTO gb FROM game_guild_bank_balances WHERE guild_id=gid FOR UPDATE;
  INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(acct,w,0) ON CONFLICT DO NOTHING;
  SELECT balance INTO ab FROM game_account_bank_balances WHERE account_id=acct AND world_id=w FOR UPDATE;
  burn:=prop.rent_gold;bg:=least(burn,gb);ba:=burn-bg;
  IF ba<=ab THEN
   SELECT * INTO a FROM game_house_auctions aa WHERE aa.world_id=w AND aa.house_key=h AND EXISTS(SELECT 1 FROM game_house_bids bb WHERE bb.auction_id=aa.auction_id AND bb.guild_id=gid AND bb.state=2) ORDER BY aa.ends_at DESC LIMIT 1;
   IF a.auction_id IS NULL THEN RAISE EXCEPTION 'GUILDHALL_ACQUISITION_RECEIPT' USING ERRCODE='23514'; END IF;
   operation:=game_guildhall_job_id(w,h,prop.ownership_generation,'rent',(extract(epoch FROM prop.paid_until)*1000000)::bigint::text);
   PERFORM game_guild_bank_apply_hall(jsonb_build_object('operation',operation,'world',w,'house',h,'guild',gid,'account',acct,'actor',leader,
    'kind','GUILDHALL_PAYMENT','auction',a.auction_id,'binding',
     (SELECT jsonb_agg(get_byte(uuid_send(operation),i)) FROM generate_series(0,15) i)||
     (SELECT jsonb_agg(get_byte(uuid_send(operation),i)) FROM generate_series(0,15) i),
    'guild_after',gb-bg,'bank_after',ab-ba,'escrow_guild_before',0,'escrow_guild_after',0,'escrow_account_before',0,'escrow_account_after',0,'burn',burn));
   UPDATE game_house_properties SET paid_until=paid_until+interval '30 days',rent_grace_until=NULL,last_guildhall_operation=operation,revision=revision+1 WHERE world_id=w AND house_key=h;
   RETURN 'null'::jsonb;
  END IF;
  IF prop.rent_grace_until IS NULL THEN
   UPDATE game_house_properties SET rent_grace_until=paid_until+interval '7 days',revision=revision+1 WHERE world_id=w AND house_key=h;
   RETURN 'null'::jsonb;
  END IF;
  IF clock_timestamp()>=prop.rent_grace_until THEN cause:='EVICTION'; END IF;
 END IF;
 IF cause IS NOT NULL THEN
  INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,owner_guild_id,ownership_revision,cause,state,noticed_at,due_at)
   VALUES(w,h,leader,gid,prop.ownership_generation,cause,1,now_ms,now_ms) RETURNING * INTO d;
  UPDATE game_house_properties SET state=5,content_fence_operation=d.operation_id,revision=revision+1 WHERE world_id=w AND house_key=h;
  IF cause='EVICTION' THEN
   INSERT INTO game_guild_house_bans(guild_id,world_id,banned_until) VALUES(gid,w,clock_timestamp()+interval '30 days')
    ON CONFLICT(guild_id) DO UPDATE SET banned_until=EXCLUDED.banned_until;
  END IF;
  RETURN to_jsonb(d.operation_id::text);
 END IF;
 RETURN 'null'::jsonb;
END $$;

CREATE FUNCTION game_guildhall_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.kind LIKE 'GUILDHALL_%' AND NOT EXISTS(SELECT 1 FROM game_house_bids WHERE guildhall_operation=NEW.operation_id
   AND guild_id=NEW.guild_id AND auction_id=NEW.hall_auction AND account_id=NEW.account_id AND character_id=NEW.actor)
  AND NOT EXISTS(SELECT 1 FROM game_house_properties WHERE world_id=NEW.world_id AND house_key=NEW.hall_house
   AND owner_guild_id=NEW.guild_id AND last_guildhall_operation=NEW.operation_id) THEN
  RAISE EXCEPTION 'Hall financial operation lacks its native House custody/period' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guildhall_operation_proven AFTER INSERT ON game_guild_bank_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guildhall_operation_proven();
CREATE FUNCTION game_house_guild_disband_released_proven(g UUID,w UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=g AND world_id=w AND state IN(3,4))
  AND NOT EXISTS(SELECT 1 FROM game_house_bids WHERE guild_id=g AND state=1)
  AND NOT EXISTS(SELECT 1 FROM game_house_properties WHERE owner_guild_id=g)
$$;
CREATE OR REPLACE FUNCTION game_guild_bank_release_ready(g UUID,w UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT game_house_guild_disband_released_proven(g,w) IS TRUE
 -- The pinned search_path starts with pg_catalog, so current_schema() is not
 -- the Game schema. Resolve it from a known Game relation instead.
 AND NOT EXISTS(SELECT 1 FROM pg_class c
  WHERE c.relnamespace=(SELECT relnamespace FROM pg_class WHERE oid='game_guilds'::regclass)
   AND c.relkind IN('r','p') AND
   (c.relname LIKE 'game_guildhall_%' OR c.relname LIKE 'game_house_guild_%'))
$$;
DO $$ DECLARE f RECORD; BEGIN
 FOR f IN SELECT oid::regprocedure AS sig FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND
  proname IN('game_guildhall_bid_transition','game_guildhall_job_id','game_guildhall_job_port',
   'game_guildhall_operation_proven','game_house_guild_disband_released_proven','game_guild_bank_release_ready') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f.sig,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.sig);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_guildhall_job_port(TEXT,JSONB,UUID,NUMERIC,BYTEA),
 game_guild_bank_release_ready(UUID,UUID),game_house_guild_disband_released_proven(UUID,UUID) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_guild_bank_payload(game_guild_bank_operations,TEXT) TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_house_management_acl_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE w UUID; h TEXT; k TEXT;
BEGIN
 w:=CASE WHEN TG_OP='DELETE' THEN OLD.world_id ELSE NEW.world_id END;
 h:=CASE WHEN TG_OP='DELETE' THEN OLD.house_key ELSE NEW.house_key END;
 k:=CASE WHEN TG_OP='DELETE' THEN OLD.list_key ELSE NEW.list_key END;
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_house_dispositions d JOIN game_house_properties p USING(world_id,house_key)
  WHERE d.world_id=w AND d.house_key=h AND d.owner_guild_id IS NOT NULL AND d.state=3
   AND d.released_at IS NOT NULL AND d.written_xact_id=pg_current_xact_id()
   AND p.state=1 AND p.owner_guild_id IS NULL AND p.ownership_generation=d.ownership_revision+1) THEN
  RETURN NULL;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.world_id=w AND r.house_key=h AND r.list_key=k AND r.action_kind=1
   AND r.created_xact_id=pg_current_xact_id() AND r.result_revision=p.acl_revision)
  OR (SELECT count(*) FROM (SELECT 1 FROM game_house_acl_entries WHERE world_id=w AND house_key=h AND list_key=k LIMIT 201) bounded)>200 THEN
  RAISE EXCEPTION 'House ACL requires bounded same-transaction receipt' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' AND NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=w AND lifecycle=1) THEN
  RAISE EXCEPTION 'House ACL member must be active in this World' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_management_acl_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_management_acl_proven() FROM PUBLIC;
