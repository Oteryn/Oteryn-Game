-- E12: a Cook sale is one existing Inbox MINT plus one native coin/bank fee.
CREATE TABLE game_hireling_food_receipts (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 hireling_id UUID NOT NULL REFERENCES game_house_hirelings(hireling_id),
 intent_binding BYTEA NOT NULL CHECK(octet_length(intent_binding)=32),
 item_id BIGINT NOT NULL,
 price_gold NUMERIC(20,0) NOT NULL,
 mint_transaction_id UUID NOT NULL UNIQUE REFERENCES game_reward_claim_inbox_mint_receipts(transaction_id),
 food_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns(transaction_id),
 fee_occurrence_id UUID NOT NULL UNIQUE,
 committed_character_revision NUMERIC(20,0) NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(game_session_id,command_id),
 CHECK((price_gold=15000 AND item_id IN(29408,29409,29410,29411,29412,29413,29414,29415,29416))
    OR (price_gold=90000 AND item_id BETWEEN 29408 AND 29415))
);
CREATE TRIGGER game_hireling_food_immutable BEFORE UPDATE OR DELETE ON game_hireling_food_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_food_no_truncate BEFORE TRUNCATE ON game_hireling_food_receipts
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_hireling_food_stamp BEFORE INSERT ON game_hireling_food_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- Extend the closed fee cause without replacing any existing source proof.
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=472)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=472 AND charm_key IS NULL AND purchase_service_key LIKE ''oteryn:service.hireling.food.%'' AND fee_gold_units IN(15000,90000))) NOT VALID';
 -- Like the item-only repair/imbuement/travel owners (0179/0196/0223),
 -- Inbox food mint does not advance CharacterRevision. Admit revision 1 only
 -- for this additional closed cause; preserve every existing cause's CHECK.
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_committed_character_revision_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check CHECK (('||substr(body,7)||') OR (cause_kind=472 AND committed_character_revision=1))';
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing native item-only fee proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
  '(NEW.cause_kind=472 AND EXISTS(SELECT 1 FROM game_hireling_food_receipts h WHERE h.character_id=NEW.character_id AND h.fee_transaction_id=NEW.transaction_id AND h.fee_occurrence_id=NEW.cause_occurrence_id AND h.committed_character_revision=NEW.committed_character_revision AND h.price_gold=NEW.fee_gold_units AND NEW.purchase_service_key=''oteryn:service.hireling.food.''||h.item_id::text AND h.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
END $$;

-- Inbox mint receipts remain private to their native owner. This closed trigger
-- reads them without granting the runtime role direct access to those tables.
CREATE FUNCTION game_hireling_food_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_reward_claim_inbox_mint_receipts m
   JOIN game_reward_claim_inbox_mint_lines l ON l.transaction_id=m.transaction_id
   JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
   JOIN game_item_fee_burns f ON f.transaction_id=NEW.fee_transaction_id
   JOIN game_character_roots cr ON cr.character_id=NEW.character_id
   WHERE m.transaction_id=NEW.mint_transaction_id AND m.character_id=NEW.character_id
    AND m.game_session_id=NEW.game_session_id AND m.command_id=NEW.command_id AND m.item_count=1
    AND m.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
    AND l.item_instance_id=NEW.food_item_instance_id AND i.quantity=1 AND i.lifecycle=1
    AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i'||NEW.item_id::text
    AND i.minted_transaction_id=m.transaction_id AND f.character_id=m.character_id AND f.world_id=m.world_id
    AND f.cause_kind=472 AND f.cause_occurrence_id=NEW.fee_occurrence_id AND f.fee_gold_units=NEW.price_gold
    AND f.purchase_service_key='oteryn:service.hireling.food.'||NEW.item_id::text
    AND f.committed_character_revision=NEW.committed_character_revision
    AND cr.character_revision=NEW.committed_character_revision AND cr.world_id=m.world_id
    AND f.created_xact_id=pg_current_xact_id())
  THEN
  RAISE EXCEPTION 'Hireling food requires its exact same-transaction Inbox mint and native fee' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_food_proven AFTER INSERT ON game_hireling_food_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_food_guard();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_hireling_food_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_hireling_food_guard() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON game_hireling_food_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_hireling_food_receipts TO oteryn_game_runtime;

-- Generated exact Crystal201189f package catalogue. Runtime has no write grant.
CREATE TABLE game_hireling_package_catalogue (
 package_index SMALLINT PRIMARY KEY CHECK(package_index BETWEEN 0 AND 22),
 name TEXT NOT NULL UNIQUE, price_gold BIGINT NOT NULL CHECK(price_gold>0),
 item_ids BIGINT[] NOT NULL CHECK(cardinality(item_ids)=3),
 quantities INTEGER[] NOT NULL CHECK(cardinality(quantities)=3), source_line INTEGER NOT NULL
);
-- BEGIN GENERATED HIRELING PACKAGES
INSERT INTO game_hireling_package_catalogue(package_index,name,price_gold,item_ids,quantities,source_line) VALUES
(0,'bash',6250,ARRAY[9657,22189,10405]::bigint[],ARRAY[20,15,10]::integer[],1823),
(1,'blockade',16150,ARRAY[9641,11703,20199]::bigint[],ARRAY[20,25,25]::integer[],1832),
(2,'chop',13050,ARRAY[10196,11447,21200]::bigint[],ARRAY[20,25,20]::integer[],1841),
(3,'epiphany',10650,ARRAY[9635,11452,10309]::bigint[],ARRAY[25,15,15]::integer[],1850),
(4,'precision',6750,ARRAY[11464,18994,10298]::bigint[],ARRAY[25,20,10]::integer[],1859),
(5,'slash',6550,ARRAY[9691,21202,9654]::bigint[],ARRAY[25,25,5]::integer[],1868),
(6,'featherweight',12250,ARRAY[25694,25702,20205]::bigint[],ARRAY[20,10,5]::integer[],1878),
(7,'strike',16700,ARRAY[11444,10311,22728]::bigint[],ARRAY[20,25,5]::integer[],1887),
(8,'swiftness',5225,ARRAY[17458,10302,14081]::bigint[],ARRAY[15,25,20]::integer[],1896),
(9,'vampirism',10475,ARRAY[9685,9633,9663]::bigint[],ARRAY[25,15,5]::integer[],1905),
(10,'vibrancy',15000,ARRAY[22053,23507,28567]::bigint[],ARRAY[20,15,5]::integer[],1914),
(11,'void',17400,ARRAY[11492,20200,22730]::bigint[],ARRAY[25,25,5]::integer[],1923),
(12,'electrify',3770,ARRAY[18993,21975,23508]::bigint[],ARRAY[25,5,1]::integer[],1933),
(13,'frost',9750,ARRAY[9661,21801,9650]::bigint[],ARRAY[25,10,5]::integer[],1942),
(14,'reap',3475,ARRAY[11484,9647,10420]::bigint[],ARRAY[25,20,5]::integer[],1951),
(15,'scorch',15875,ARRAY[9636,5920,5954]::bigint[],ARRAY[25,5,5]::integer[],1960),
(16,'venom',1820,ARRAY[9686,9640,21194]::bigint[],ARRAY[25,20,2]::integer[],1969),
(17,'cloud fabric',13775,ARRAY[9644,14079,9665]::bigint[],ARRAY[20,15,10]::integer[],1979),
(18,'demon presence',20250,ARRAY[9639,9638,10304]::bigint[],ARRAY[25,25,20]::integer[],1988),
(19,'dragon hide',10850,ARRAY[5877,16131,11658]::bigint[],ARRAY[20,10,5]::integer[],1997),
(20,'lich shroud',5650,ARRAY[11466,22007,9660]::bigint[],ARRAY[25,20,5]::integer[],2006),
(21,'quara scale',3650,ARRAY[10295,10307,14012]::bigint[],ARRAY[25,15,10]::integer[],2015),
(22,'snake skin',12550,ARRAY[17823,9694,11702]::bigint[],ARRAY[25,20,10]::integer[],2024);
-- END GENERATED HIRELING PACKAGES

CREATE TABLE game_hireling_package_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_npc_trades(transaction_id),
 hireling_id UUID NOT NULL REFERENCES game_house_hirelings(hireling_id),
 package_index SMALLINT NOT NULL REFERENCES game_hireling_package_catalogue(package_index),
 present_item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 intent_binding BYTEA NOT NULL CHECK(octet_length(intent_binding)=32),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_hireling_package_lines (
 transaction_id UUID NOT NULL REFERENCES game_hireling_package_receipts(transaction_id),
 line_ordinal SMALLINT NOT NULL CHECK(line_ordinal BETWEEN 1 AND 3),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 definition_revision TEXT NOT NULL CHECK(octet_length(definition_revision) BETWEEN 1 AND 512),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,line_ordinal)
);

CREATE FUNCTION game_hireling_package_current(p_transaction UUID) RETURNS BOOLEAN
 LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE p game_hireling_package_receipts%ROWTYPE; c game_hireling_package_catalogue%ROWTYPE;
        t game_npc_trades%ROWTYPE; n INTEGER;
BEGIN
 SELECT * INTO p FROM game_hireling_package_receipts WHERE transaction_id=p_transaction;
 IF NOT FOUND OR p.created_xact_id<>pg_current_xact_id() THEN RETURN FALSE; END IF;
 SELECT * INTO STRICT c FROM game_hireling_package_catalogue WHERE package_index=p.package_index;
 SELECT * INTO t FROM game_npc_trades WHERE transaction_id=p_transaction;
 IF NOT FOUND OR t.created_xact_id<>pg_current_xact_id() OR t.side<>1 OR t.quantity<>1 OR t.item_units<>1
   OR t.unit_price<>c.price_gold OR t.item_instance_id<>p.present_item_instance_id
   OR t.item_definition_family<>'Item' OR t.item_definition_production_key<>'oteryn:item.tibia.i2856'
   OR t.npc_key<>'oteryn:npc.hireling:'||replace(p.hireling_id::text,'-','')
   OR (SELECT count(*) FROM game_hireling_package_lines WHERE transaction_id=p_transaction)<>3 THEN RETURN FALSE; END IF;
 FOR n IN 1..3 LOOP
  IF NOT EXISTS(SELECT 1 FROM game_hireling_package_lines l
    JOIN game_item_instances i USING(item_instance_id)
    JOIN game_item_nested_entries e USING(item_instance_id)
    WHERE l.transaction_id=p_transaction AND l.line_ordinal=n AND l.created_xact_id=pg_current_xact_id()
     AND i.minted_transaction_id=p_transaction AND i.world_id=t.world_id AND i.quantity=c.quantities[n]
     AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i'||c.item_ids[n]::text
     AND i.definition_revision_ref=l.definition_revision AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
     AND e.world_id=t.world_id AND e.parent_item_instance_id=p.present_item_instance_id AND e.placement_ordinal=n
     AND e.placed_transaction_id=p_transaction) THEN RETURN FALSE; END IF;
 END LOOP;
 RETURN TRUE;
END $$;
CREATE FUNCTION game_hireling_package_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_hireling_package_current(NEW.transaction_id) THEN
  RAISE EXCEPTION 'Hireling package requires its native paid present and exact three nested stacks' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_hireling_package_receipts','game_hireling_package_lines'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id()',name||'_stamp',name);
  EXECUTE format('CREATE CONSTRAINT TRIGGER %I AFTER INSERT ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_package_guard()',name||'_proven',name);
 END LOOP;
END $$;

-- Every original guard remains intact. Only a complete same-transaction package supplies
-- the three additional mint/custody proofs; loose or incomplete ingredients cannot commit.
DO $$ DECLARE body TEXT; needle TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_mint_consistency_guard()'::regprocedure) INTO STRICT body;
 body:=regexp_replace(body,'BEGIN',
  'BEGIN IF EXISTS(SELECT 1 FROM game_hireling_package_lines l WHERE l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.minted_transaction_id AND game_hireling_package_current(l.transaction_id)) THEN RETURN NULL; END IF;');
 EXECUTE body;
 SELECT pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure) INTO STRICT body;
 body:=regexp_replace(body,'BEGIN',
  'BEGIN IF TG_OP=''INSERT'' AND EXISTS(SELECT 1 FROM game_hireling_package_lines l WHERE l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id AND game_hireling_package_current(l.transaction_id)) THEN RETURN NULL; END IF;');
 EXECUTE body;
 SELECT pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure) INTO STRICT body;
 needle:='WHERE i.minted_transaction_id = NEW.transaction_id) <> mints';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native NPC mint-count proof'; END IF;
 body:=replace(body,needle,'WHERE i.minted_transaction_id = NEW.transaction_id AND NOT EXISTS(SELECT 1 FROM game_hireling_package_lines l WHERE l.item_instance_id=i.item_instance_id AND l.transaction_id=NEW.transaction_id AND game_hireling_package_current(l.transaction_id))) <> mints');
 EXECUTE body;
 SELECT pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure) INTO STRICT body;
 body:=regexp_replace(body,'BEGIN',
  'BEGIN IF octet_length(NEW.envelope)<=34816 AND game_hireling_package_current(NEW.transaction_id) AND EXISTS(SELECT 1 FROM game_npc_trades t WHERE t.transaction_id=NEW.transaction_id AND t.event_id=NEW.event_id AND t.envelope_sha256=NEW.envelope_sha256 AND t.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;');
 EXECUTE body;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_package_current(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_package_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_hireling_package_catalogue,game_hireling_package_receipts,game_hireling_package_lines FROM PUBLIC;
REVOKE ALL ON FUNCTION game_hireling_package_current(UUID),game_hireling_package_guard() FROM PUBLIC;
GRANT SELECT ON game_hireling_package_catalogue TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_hireling_package_receipts,game_hireling_package_lines TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_hireling_package_current(UUID) TO oteryn_game_runtime;

-- Append to 0702_hireling_services.sql (reserved migration block 0700..0703).
-- ANSWER Q8, PR2783: native physical SELL payout, maximum executable Crystal price
-- item43963=5,400,000. Six crystal stacks plus two residual denominations are bounded
-- by the existing 23 participant /66 work-unit /34,816-byte SELL_ALL resource rows.
-- No new money owner, bank payout or ground fallback. All native burn/mint proofs remain.

CREATE FUNCTION game_hireling_npc_sell_current(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(
   SELECT 1 FROM game_npc_trades t
   JOIN game_house_hirelings h ON t.npc_key='oteryn:npc.hireling:'||replace(h.hireling_id::text,'-','')
   JOIN game_durability_reconnect_sessions s ON s.game_session_id=t.game_session_id
   JOIN game_hireling_entitlements e ON e.character_id=h.owner_character_id AND e.entitlement_id=1004
   WHERE t.transaction_id=p_transaction AND t.created_xact_id=pg_current_xact_id()
     AND t.side=2 AND t.total_gold_units BETWEEN 1010000 AND 5400000
     AND h.active AND h.world_id=t.world_id AND s.world_id=h.world_id
     AND s.character_id=t.character_id AND s.runtime_scope_kind=2
     AND s.runtime_scope_house_key=h.house_key
     AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id
 )
$$;

ALTER TABLE game_npc_trades
 DROP CONSTRAINT game_npc_trades_minted_gold_units_check,
 ADD CONSTRAINT game_npc_trades_minted_gold_units_check CHECK(minted_gold_units BETWEEN 0 AND 5400000),
 DROP CONSTRAINT game_npc_trades_conservation,
 ADD CONSTRAINT game_npc_trades_conservation CHECK(CASE side
   WHEN 1 THEN burned_gold_units-minted_gold_units+bank_debit_gold_units=total_gold_units
     AND minted_gold_units<=9999 AND line_count<=23
   ELSE burned_gold_units=0 AND minted_gold_units=total_gold_units AND bank_debit_gold_units=0
     AND CASE WHEN npc_key ~ '^oteryn:npc\.hireling:[0-9a-f]{32}$'
       THEN total_gold_units<=5400000 AND line_count<=CASE WHEN sell_all THEN 23 ELSE 9 END
       ELSE total_gold_units<=1009999 AND line_count<=CASE WHEN sell_all THEN 23 ELSE 4 END END
 END);

-- Amend only denomination ordering and the new bounded current-Hireling precondition.
-- The original native consistency function, including 0702 package proofs, stays intact.
DO $$ DECLARE body TEXT; needle TEXT; BEGIN
 SELECT pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure) INTO STRICT body;
 needle:='p.coin_worth <= l.coin_worth';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native NPC denomination ordering'; END IF;
 body:=replace(body,needle,'(p.coin_worth < l.coin_worth OR (p.coin_worth = l.coin_worth AND NOT (NEW.side=2 AND NEW.total_gold_units>1009999 AND l.coin_worth=10000 AND p.quantity_after=100)))');
 body:=regexp_replace(body,'BEGIN',
  'BEGIN IF NEW.side=2 AND NEW.total_gold_units>1009999 AND (NOT game_hireling_npc_sell_current(NEW.transaction_id) OR (SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=NEW.transaction_id AND line_kind=2)>8 OR (SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=NEW.transaction_id AND line_kind=2 AND coin_worth=10000)>6) THEN RAISE EXCEPTION ''extended NPCSELL requires current Hireling and bounded physical stacks'' USING ERRCODE=''23514''; END IF;');
 EXECUTE body;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_npc_sell_current(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hireling_npc_sell_current(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_hireling_npc_sell_current(UUID) TO oteryn_game_runtime;

-- The existing Bank owner records actual House authority separately from origin Channel.
ALTER TABLE game_account_bank_operations
 ADD COLUMN source_house_key TEXT,
 ADD COLUMN source_hireling_id UUID REFERENCES game_house_hirelings(hireling_id),
 ADD COLUMN source_connection_generation NUMERIC(20,0)
   CHECK(source_connection_generation BETWEEN 1 AND 18446744073709551615),
 ADD CONSTRAINT game_bank_hireling_current_generation_shape
   CHECK((source_house_key IS NULL)=(source_connection_generation IS NULL)),
 ADD CONSTRAINT game_bank_hireling_scope_shape CHECK(
   (source_house_key IS NULL AND source_hireling_id IS NULL)
   OR (source_house_key IS NOT NULL AND source_hireling_id IS NOT NULL
       AND game_house_scope_key_valid(source_house_key)));
CREATE INDEX game_bank_hireling_identity ON game_account_bank_operations(source_hireling_id);

CREATE FUNCTION game_bank_hireling_scope_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.source_house_key IS NOT NULL AND NOT EXISTS(
   SELECT 1 FROM game_npc_bank_confirmations c
   JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.game_session_id
   JOIN game_house_hirelings h ON h.hireling_id=NEW.source_hireling_id
   JOIN game_hireling_entitlements e ON e.character_id=h.owner_character_id AND e.entitlement_id=1001
   WHERE c.operation_occurrence_id=NEW.operation_occurrence_id
     AND c.character_id=NEW.acting_character_id AND c.world_id=NEW.world_id
     AND c.origin_channel_id=NEW.channel_id
     AND c.origin_scope_generation=NEW.runtime_scope_ownership_generation
     AND c.npc_key='oteryn:npc.hireling:'||replace(h.hireling_id::text,'-','')
     AND s.character_id=c.character_id AND s.world_id=c.world_id
     AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=NEW.source_house_key
     -- A retained confirmation is immutable origin provenance. A real
     -- same-session resume supplies a separately current execution generation.
     AND c.origin_connection_generation<=NEW.source_connection_generation
     AND s.current_generation=NEW.source_connection_generation
     AND s.character_lease_generation=c.origin_character_lease_generation
     AND s.scope_ownership_generation=c.origin_scope_generation
     AND h.active AND h.world_id=NEW.world_id AND h.house_key=NEW.source_house_key
     AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id
 ) THEN
   RAISE EXCEPTION 'Hireling Bank operation requires exact current House confirmation' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_bank_hireling_scope_proven
 AFTER INSERT ON game_account_bank_operations DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_bank_hireling_scope_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_bank_hireling_scope_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_bank_hireling_scope_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bank_hireling_scope_proven() TO oteryn_game_runtime;

-- Same Bank resource row, extended by536 bytes for an explicit max-size House scope.
ALTER TABLE game_account_bank_audit_outbox
 DROP CONSTRAINT game_account_bank_audit_outbox_envelope_check,
 ADD CONSTRAINT game_account_bank_audit_outbox_envelope_check
 CHECK(octet_length(envelope) BETWEEN 1 AND 26255);

-- Append to 0702_hireling_services.sql, after the NPCSELL extension.
-- Named-House attribution for the existing NPC trade, native fee and stash owners.
-- World + canonical House key determines game_house_scope_instance_id; Channel columns
-- remain origin provenance. Original Channel events and their gates remain unchanged.

ALTER TABLE game_npc_trades ADD COLUMN house_key TEXT NULL
 CHECK(house_key IS NULL OR (octet_length(house_key) BETWEEN 1 AND 512 AND house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$'));
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trade_house_shape
 CHECK((npc_key ~ '^oteryn:npc\.hireling:[0-9a-f]{32}$')=(house_key IS NOT NULL));
ALTER TABLE game_item_fee_burns ADD COLUMN house_key TEXT NULL
 CHECK(house_key IS NULL OR (octet_length(house_key) BETWEEN 1 AND 512 AND house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$'));
ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_hireling_fee_house_shape
 CHECK((cause_kind=472)=(house_key IS NOT NULL));
ALTER TABLE game_character_stash_operations
 ADD COLUMN house_key TEXT NULL CHECK(house_key IS NULL OR (octet_length(house_key) BETWEEN 1 AND 512 AND house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$')),
 ADD COLUMN house_scope_ownership_generation NUMERIC(20,0) NULL
 CHECK(house_scope_ownership_generation IS NULL OR house_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
 ADD CONSTRAINT game_hireling_stash_house_shape CHECK((house_key IS NULL)=(house_scope_ownership_generation IS NULL));

CREATE FUNCTION game_hireling_service_scope_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE current_scope BOOLEAN;
BEGIN
 IF NEW.house_key IS NULL THEN RETURN NULL; END IF;
 IF TG_TABLE_NAME='game_npc_trades' THEN
   SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
     JOIN game_house_hirelings h ON h.world_id=s.world_id AND h.house_key=s.runtime_scope_house_key
       AND NEW.npc_key='oteryn:npc.hireling:'||replace(h.hireling_id::text,'-','')
     WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
       AND s.world_id=NEW.world_id AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=NEW.house_key
       AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.house_key)
       AND s.scope_ownership_generation=NEW.runtime_scope_ownership_generation AND s.origin_channel_id=NEW.channel_id
       AND h.active AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id
       AND NEW.created_xact_id=pg_current_xact_id()) INTO current_scope;
 ELSIF TG_TABLE_NAME='game_item_fee_burns' THEN
   SELECT EXISTS(SELECT 1 FROM game_hireling_food_receipts r
     JOIN game_durability_reconnect_sessions s ON s.game_session_id=r.game_session_id
     JOIN game_house_hirelings h ON h.hireling_id=r.hireling_id
     WHERE r.fee_transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id
       AND r.created_xact_id=pg_current_xact_id() AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id
       AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=NEW.house_key
       AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.house_key)
       AND s.scope_ownership_generation=NEW.runtime_scope_ownership_generation AND s.origin_channel_id=NEW.channel_id
       AND h.active AND h.world_id=NEW.world_id AND h.house_key=NEW.house_key
       AND NEW.cause_kind=472 AND NEW.created_xact_id=pg_current_xact_id()) INTO current_scope;
 ELSIF TG_TABLE_NAME='game_character_stash_operations' THEN
   SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
     JOIN game_house_hirelings h ON h.world_id=s.world_id AND h.house_key=s.runtime_scope_house_key AND h.active
     JOIN game_hireling_entitlements e ON e.character_id=h.owner_character_id AND e.entitlement_id=1003
     WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
       AND s.world_id=NEW.world_id AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=NEW.house_key
       AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.house_key)
       AND s.scope_ownership_generation=NEW.house_scope_ownership_generation
       AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id
       AND NEW.created_xact_id=pg_current_xact_id()) INTO current_scope;
 ELSE
   RAISE EXCEPTION 'unexpected Hireling scope owner' USING ERRCODE='23514';
 END IF;
 IF current_scope IS DISTINCT FROM true THEN
   RAISE EXCEPTION 'Hireling service must record its actual named House scope' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_trade_house_proven AFTER INSERT ON game_npc_trades
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_service_scope_proven();
CREATE CONSTRAINT TRIGGER game_hireling_fee_house_proven AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_service_scope_proven();
CREATE CONSTRAINT TRIGGER game_hireling_stash_house_proven AFTER INSERT ON game_character_stash_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_service_scope_proven();
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure) INTO STRICT body;
 body:=regexp_replace(body,'BEGIN',
  'BEGIN IF octet_length(NEW.envelope)<=26612 AND EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.event_id=NEW.event_id AND f.transaction_id=NEW.transaction_id AND f.cause_kind=472 AND f.house_key IS NOT NULL AND f.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF; IF octet_length(NEW.envelope)<=34816 AND EXISTS(SELECT 1 FROM game_npc_trades t WHERE t.event_id=NEW.event_id AND t.transaction_id=NEW.transaction_id AND t.house_key IS NOT NULL AND t.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;');
 EXECUTE body;
 EXECUTE format('ALTER FUNCTION game_hireling_service_scope_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hireling_service_scope_proven() FROM PUBLIC;

-- E12 gourmet food: native USE471, one shared600s exhaustion and native
-- ConditionStore remaining-tick snapshots. Crystal201189f hireling_foods.lua1-67,
-- condition.cpp175,356-369,552-590; iologindata_save_player.cpp264-276.
-- Append after the existing Hireling service migration; no independent item owner.
CREATE TABLE game_hireling_gourmet_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 food_id INTEGER NOT NULL CHECK(food_id BETWEEN 29408 AND 29416),
 started_at TIMESTAMPTZ NOT NULL,
 cooldown_until TIMESTAMPTZ NOT NULL CHECK(cooldown_until=started_at+interval '600 seconds'),
 attributes_item INTEGER CHECK(attributes_item IN(29408,29409,29410,29411,29413)),
 attributes_ms INTEGER NOT NULL CHECK(attributes_ms BETWEEN 0 AND 3600000),
 haste_ms INTEGER NOT NULL CHECK(haste_ms BETWEEN 0 AND 3600000),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK((attributes_item IS NULL)=(attributes_ms=0))
);
CREATE INDEX game_hireling_gourmet_receipts_character ON game_hireling_gourmet_receipts(character_id);
CREATE TRIGGER game_hireling_gourmet_receipts_immutable BEFORE UPDATE OR DELETE ON game_hireling_gourmet_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_gourmet_receipts_no_truncate BEFORE TRUNCATE ON game_hireling_gourmet_receipts
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_gourmet_receipts_stamp BEFORE INSERT ON game_hireling_gourmet_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE TABLE game_hireling_gourmet_state (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 last_transaction_id UUID NOT NULL REFERENCES game_hireling_gourmet_receipts(transaction_id),
 cooldown_until TIMESTAMPTZ NOT NULL,
 attributes_item INTEGER CHECK(attributes_item IN(29408,29409,29410,29411,29413)),
 attributes_ms INTEGER NOT NULL CHECK(attributes_ms BETWEEN 0 AND 3600000),
 haste_ms INTEGER NOT NULL CHECK(haste_ms BETWEEN 0 AND 3600000),
 CHECK((attributes_item IS NULL)=(attributes_ms=0))
);
CREATE TRIGGER game_hireling_gourmet_state_no_delete BEFORE DELETE ON game_hireling_gourmet_state
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_gourmet_state_no_truncate BEFORE TRUNCATE ON game_hireling_gourmet_state
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_hireling_gourmet_state_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_hireling_gourmet_receipts%ROWTYPE;
BEGIN
 IF TG_OP='UPDATE' AND NEW.character_id<>OLD.character_id THEN
  RAISE EXCEPTION 'gourmet state cannot change Character' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND NEW.last_transaction_id=OLD.last_transaction_id THEN
  IF NEW.cooldown_until<>OLD.cooldown_until OR NEW.haste_ms>OLD.haste_ms
    OR NEW.attributes_ms>OLD.attributes_ms
    OR (NEW.attributes_item IS NOT NULL AND NEW.attributes_item IS DISTINCT FROM OLD.attributes_item)
  THEN RAISE EXCEPTION 'gourmet checkpoint can only decrease native remaining ticks' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT * INTO r FROM game_hireling_gourmet_receipts WHERE transaction_id=NEW.last_transaction_id;
  IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id() OR r.character_id<>NEW.character_id
    OR r.cooldown_until<>NEW.cooldown_until OR r.attributes_item IS DISTINCT FROM NEW.attributes_item
    OR r.attributes_ms<>NEW.attributes_ms OR r.haste_ms<>NEW.haste_ms THEN
   RAISE EXCEPTION 'gourmet state requires its same-transaction native USE receipt' USING ERRCODE='23514';
  END IF;
  IF TG_OP='UPDATE' THEN
   IF r.started_at<OLD.cooldown_until THEN
    RAISE EXCEPTION 'shared gourmet food cooldown is still active' USING ERRCODE='23514';
   END IF;
   IF (NEW.attributes_ms>OLD.attributes_ms OR NEW.attributes_item IS DISTINCT FROM OLD.attributes_item)
      AND NEW.attributes_item IS NOT NULL AND NEW.attributes_item<>r.food_id
     OR NEW.haste_ms>OLD.haste_ms AND r.food_id<>29412 THEN
    RAISE EXCEPTION 'a food cannot grant another condition source' USING ERRCODE='23514';
   END IF;
  ELSE
   IF NEW.attributes_item IS NOT NULL AND NEW.attributes_item<>r.food_id
      OR NEW.haste_ms>0 AND r.food_id<>29412 THEN
    RAISE EXCEPTION 'initial gourmet effect must match the consumed food' USING ERRCODE='23514';
   END IF;
  END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_hireling_gourmet_state_guard BEFORE INSERT OR UPDATE ON game_hireling_gourmet_state
 FOR EACH ROW EXECUTE FUNCTION game_hireling_gourmet_state_guard();

CREATE FUNCTION game_hireling_gourmet_native_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_hireling_gourmet_receipts%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_hireling_gourmet_receipts WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
  SELECT 1 FROM game_item_use_records u WHERE u.transaction_id=r.transaction_id
   AND u.cause_kind=471 AND u.shape=1 AND u.character_id=r.character_id
   AND u.game_session_id=r.game_session_id AND u.command_id=r.command_id
   AND u.source_definition_family='Item'
   AND u.source_definition_production_key='oteryn:item.tibia.i'||r.food_id::text
   AND u.source_quantity_after=u.source_quantity_before-1
   AND u.created_xact_id=r.created_xact_id
 ) OR NOT EXISTS(
  SELECT 1 FROM game_hireling_gourmet_state s WHERE s.character_id=r.character_id
   AND s.last_transaction_id=r.transaction_id AND s.cooldown_until=r.cooldown_until
 ) THEN RAISE EXCEPTION 'gourmet food needs same native burn, cooldown and condition snapshot' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_hireling_gourmet_receipt_proven AFTER INSERT ON game_hireling_gourmet_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hireling_gourmet_native_proven();
CREATE CONSTRAINT TRIGGER game_hireling_gourmet_burn_proven AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=471)
 EXECUTE FUNCTION game_hireling_gourmet_native_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hireling_gourmet_state_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_gourmet_native_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_hireling_gourmet_receipts,game_hireling_gourmet_state FROM PUBLIC;
REVOKE ALL ON FUNCTION game_hireling_gourmet_state_guard(),game_hireling_gourmet_native_proven() FROM PUBLIC;
GRANT SELECT,INSERT ON game_hireling_gourmet_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_hireling_gourmet_state TO oteryn_game_runtime;

-- The newly consumed source grants its exact donor condition. Other source
-- families may retain only the prior, decreasing ticks under the existing guard.
ALTER TABLE game_hireling_gourmet_receipts
 ADD CONSTRAINT game_hireling_gourmet_receipt_source_condition CHECK (
  CASE
   WHEN food_id IN (29408,29409,29410,29411,29413)
    THEN attributes_item IS NOT DISTINCT FROM food_id AND attributes_ms=3600000
   WHEN food_id=29412 THEN haste_ms=3600000
   ELSE TRUE
  END
 );

-- E12: source NPCBUY creates decoration kit23398 + original unWrapId.
-- Extend the existing HOUSE-ITEM-WIRE-1 owner and its exact custody proof chain.
-- Crystal201189f: items.hpp168-170, game.cpp2761-2767. No Ground target gains Inventory admission.
CREATE TABLE game_house_furniture_kit_catalogue (
 source_row INTEGER PRIMARY KEY CHECK(source_row BETWEEN 0 AND 1674),
 target_key TEXT NOT NULL UNIQUE,
 price_gold BIGINT NOT NULL CHECK(price_gold>0),
 source_line INTEGER NOT NULL
);
-- BEGIN GENERATED HIRELING FURNITURE KITS
INSERT INTO game_house_furniture_kit_catalogue(source_row,target_key,price_gold,source_line) VALUES
(0,'oteryn:item.tibia.i32482',40,35),
(1,'oteryn:item.tibia.i32473',40,36),
(2,'oteryn:item.tibia.i32486',40,37),
(3,'oteryn:item.tibia.i32477',40,38),
(4,'oteryn:item.tibia.i32483',40,39),
(5,'oteryn:item.tibia.i32474',40,40),
(6,'oteryn:item.tibia.i32487',40,41),
(7,'oteryn:item.tibia.i32478',40,42),
(8,'oteryn:item.tibia.i32484',40,43),
(9,'oteryn:item.tibia.i32475',40,44),
(10,'oteryn:item.tibia.i32488',40,45),
(11,'oteryn:item.tibia.i32479',40,46),
(12,'oteryn:item.tibia.i32489',40,47),
(13,'oteryn:item.tibia.i32480',40,48),
(14,'oteryn:item.tibia.i32485',40,49),
(15,'oteryn:item.tibia.i32476',40,50),
(82,'oteryn:item.tibia.i2426',20,117),
(90,'oteryn:item.tibia.i2352',20,125);
-- END GENERATED HIRELING FURNITURE KITS
CREATE TABLE game_house_furniture_kit_origins (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 mint_transaction_id UUID NOT NULL UNIQUE REFERENCES game_npc_trades(transaction_id),
 source_row INTEGER NOT NULL REFERENCES game_house_furniture_kit_catalogue(source_row),
 target_key TEXT NOT NULL,
 target_revision TEXT NOT NULL CHECK(target_revision='definition-r1'),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_house_furniture_kit_origin_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(
  SELECT 1 FROM game_house_furniture_kit_catalogue c
  JOIN game_npc_trades t ON t.transaction_id=NEW.mint_transaction_id
  JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
  JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id
  WHERE c.source_row=NEW.source_row AND c.target_key=NEW.target_key
   AND NEW.created_xact_id=pg_current_xact_id() AND t.created_xact_id=pg_current_xact_id()
   AND t.side=1 AND NOT t.sell_all AND t.quantity=1 AND t.item_units=1
   AND t.unit_price=c.price_gold AND t.total_gold_units=c.price_gold
   AND t.npc_key ~ '^oteryn:npc\.hireling:[0-9a-f]{32}$' AND t.house_key IS NOT NULL
   AND t.item_instance_id=i.item_instance_id AND t.item_definition_family='Item'
   AND t.item_definition_production_key='oteryn:item.tibia.i23398'
   AND t.item_definition_revision_ref=NEW.target_revision
   AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i23398'
   AND i.definition_revision_ref=NEW.target_revision AND i.world_id=t.world_id
   AND i.quantity=1 AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
   AND i.minted_transaction_id=t.transaction_id AND e.world_id=t.world_id
   AND e.character_id=t.character_id AND e.parent_item_instance_id=t.backpack_item_instance_id
   AND e.placed_transaction_id=t.transaction_id
   AND EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=t.transaction_id
    AND a.event_id=t.event_id AND a.envelope_sha256=t.envelope_sha256)
 ) THEN RAISE EXCEPTION 'furniture kit requires its exact same-transaction native NPCBUY mint' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_house_furniture_kit_origin_immutable BEFORE UPDATE OR DELETE ON game_house_furniture_kit_origins
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_house_furniture_kit_origin_no_truncate BEFORE TRUNCATE ON game_house_furniture_kit_origins
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_house_furniture_kit_origin_stamp BEFORE INSERT ON game_house_furniture_kit_origins
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE CONSTRAINT TRIGGER game_house_furniture_kit_origin_proven AFTER INSERT ON game_house_furniture_kit_origins
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_furniture_kit_origin_proven();

CREATE FUNCTION game_house_furniture_wrap_pair(p_item UUID,p_before TEXT,p_after TEXT,p_action SMALLINT)
 RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER STRICT AS $$
 SELECT (p_action=1 AND p_before='oteryn:item.tibia.i6355' AND p_after='oteryn:item.tibia.i6371')
  OR (p_action=2 AND p_before='oteryn:item.tibia.i6371' AND p_after='oteryn:item.tibia.i6355')
  OR EXISTS(SELECT 1 FROM game_house_furniture_kit_origins k
    JOIN game_house_furniture_kit_catalogue c USING(source_row)
    WHERE k.item_instance_id=p_item AND k.target_key=c.target_key AND k.target_revision='definition-r1'
     AND ((p_action=1 AND p_before=k.target_key AND p_after='oteryn:item.tibia.i23398')
       OR (p_action=2 AND p_before='oteryn:item.tibia.i23398' AND p_after=k.target_key)))
$$;

-- Every existing session, lease, content/ACL, occupancy, capacity, lineage and
-- custody proof is preserved. Only the recognized pair and first-unwrap origin
-- are extended; a fabricated or unproven kit still cannot place furniture.
DO $migration$ DECLARE body TEXT; needle TEXT; replacement TEXT; BEGIN
 SELECT pg_get_functiondef('game_house_furniture_wrap_complete(UUID)'::regprocedure) INTO STRICT body;
 needle:=$old$OR (r.action=1 AND ((r.before_item->>'definition_production_key')<>'oteryn:item.tibia.i6355' OR r.after_key<>'oteryn:item.tibia.i6371'))
  OR (r.action=2 AND ((r.before_item->>'definition_production_key')<>'oteryn:item.tibia.i6371' OR r.after_key<>'oteryn:item.tibia.i6355'))$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing existing furniture wrap pair proof'; END IF;
 EXECUTE replace(body,needle,'OR NOT game_house_furniture_wrap_pair(r.item_instance_id,r.before_item->>''definition_production_key'',r.after_key,r.action)');
 SELECT pg_get_functiondef('game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure) INTO STRICT body;
 needle:=$old$OR (p_action=1 AND (i.definition_production_key<>'oteryn:item.tibia.i6355' OR p_after<>'oteryn:item.tibia.i6371'))
  OR (p_action=2 AND (i.definition_production_key<>'oteryn:item.tibia.i6371' OR p_after<>'oteryn:item.tibia.i6355'))$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing existing furniture conversion admission'; END IF;
 body:=replace(body,needle,'OR NOT game_house_furniture_wrap_pair(p_item,i.definition_production_key,p_after,p_action)');
 needle:=$old$IF NOT FOUND OR NOT EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts prior
    WHERE prior.item_instance_id=p_item AND prior.action=1 AND prior.after_key=i.definition_production_key
     AND prior.before_item->>'definition_production_key'=p_after) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;$old$;
 replacement:=$new$IF NOT FOUND OR (NOT EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts prior
    WHERE prior.item_instance_id=p_item AND prior.action=1 AND prior.after_key=i.definition_production_key
     AND prior.before_item->>'definition_production_key'=p_after)
    AND NOT EXISTS(SELECT 1 FROM game_house_furniture_kit_origins k
      WHERE k.item_instance_id=p_item AND k.target_key=p_after AND k.target_revision=i.definition_revision_ref
       AND i.definition_production_key='oteryn:item.tibia.i23398')) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;$new$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing existing furniture origin proof'; END IF;
 EXECUTE replace(body,needle,replacement);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_kit_origin_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_house_furniture_kit_catalogue,game_house_furniture_kit_origins FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_kit_catalogue,game_house_furniture_kit_origins TO oteryn_game_runtime;
GRANT INSERT ON game_house_furniture_kit_origins TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_house_furniture_kit_origin_proven(),game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
