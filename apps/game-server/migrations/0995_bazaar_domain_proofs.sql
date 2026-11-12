-- Snapshot the unchanged progression interpretation into each authority-only
-- revision, so an ownership/lifecycle change never fabricates XP or breaks the
-- existing gap-free CharacterRevision chain.
ALTER TABLE game_bazaar_character_transitions
 ADD COLUMN level_before BIGINT NOT NULL,
 ADD COLUMN level_after BIGINT NOT NULL CHECK(level_after=level_before),
 ADD COLUMN experience_before BIGINT NOT NULL,
 ADD COLUMN experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 ADD COLUMN profile_revision TEXT NOT NULL,
 ADD COLUMN ruleset_revision TEXT NOT NULL,
 ADD COLUMN content_revision TEXT NOT NULL,
 ADD COLUMN simulation_revision TEXT NOT NULL,
 ADD COLUMN evidence_revision TEXT NOT NULL,
 ADD COLUMN declaration_revision TEXT NOT NULL,
 ADD COLUMN policy_revision TEXT NOT NULL,
 ADD COLUMN reward_revision TEXT NOT NULL,
 ADD CONSTRAINT game_bazaar_transition_revision UNIQUE(character_id,new_revision);
CREATE FUNCTION game_bazaar_transition_capture() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE s game_character_progression_state;
BEGIN
 SELECT * INTO STRICT s FROM game_character_progression_state WHERE character_id=NEW.character_id FOR UPDATE;
 IF s.character_revision<>NEW.old_revision THEN RAISE EXCEPTION 'Bazaar progression revision mismatch' USING ERRCODE='23514';END IF;
 NEW.level_before:=s.level;NEW.level_after:=s.level;NEW.experience_before:=s.total_experience;NEW.experience_after:=s.total_experience;
 NEW.profile_revision:=s.profile_revision;NEW.ruleset_revision:=s.ruleset_revision;NEW.content_revision:=s.content_revision;
 NEW.simulation_revision:=s.simulation_revision;NEW.evidence_revision:=s.evidence_revision;
 NEW.declaration_revision:=s.declaration_revision;NEW.policy_revision:=s.policy_revision;NEW.reward_revision:=s.reward_revision;
 NEW.created_xact_id:=pg_current_xact_id();

 RETURN NEW;
END $$;
CREATE TRIGGER game_bazaar_transition_capture BEFORE INSERT ON game_bazaar_character_transitions FOR EACH ROW EXECUTE FUNCTION game_bazaar_transition_capture();
CREATE FUNCTION game_bazaar_transition_progression() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 UPDATE game_character_progression_state SET character_revision=NEW.new_revision WHERE character_id=NEW.character_id AND character_revision=NEW.old_revision;
 IF NOT FOUND THEN RAISE EXCEPTION 'Bazaar progression successor missing' USING ERRCODE='23514';END IF;RETURN NULL;END $$;
CREATE TRIGGER game_bazaar_transition_progression AFTER INSERT ON game_bazaar_character_transitions FOR EACH ROW EXECUTE FUNCTION game_bazaar_transition_progression();
CREATE VIEW game_bazaar_character_revision_chain WITH(security_invoker=true) AS
 SELECT character_id,old_revision AS original_character_revision,new_revision AS committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,
 simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_bazaar_character_transitions;
CREATE VIEW game_bazaar_xp_revision_chain WITH(security_invoker=true) AS
 SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,
 profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_xp_receipts UNION ALL SELECT * FROM game_bazaar_character_revision_chain;
DO $$ DECLARE body TEXT;BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_character_xp_receipts' IN body)=0 THEN RAISE EXCEPTION 'Bazaar Character chain anchor missing';END IF;
 EXECUTE replace(body,'game_character_xp_receipts','game_bazaar_xp_revision_chain');
END $$;
CREATE FUNCTION game_bazaar_transition_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_bazaar_decisions d JOIN game_character_roots r ON r.character_id=NEW.character_id
 WHERE d.issuer=NEW.issuer AND d.operation_id=NEW.operation_id AND d.result_code='CHAR_CMD_OK'
 AND d.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
 AND r.account_id=NEW.new_account AND r.character_revision=NEW.new_revision AND r.lifecycle=NEW.new_lifecycle
 AND (convert_from(d.intent,'UTF8')::jsonb->>'character_id')::uuid=NEW.character_id
 AND (CASE WHEN d.command='TransferCharacterOwnership' THEN
      (convert_from(d.intent,'UTF8')::jsonb->>'expected_from_account_id')::uuid=NEW.old_account
      AND (convert_from(d.intent,'UTF8')::jsonb->>'to_account_id')::uuid=NEW.new_account
      AND NEW.old_lifecycle=NEW.new_lifecycle
     WHEN d.command='LockBazaarCharacter' THEN NEW.old_account=NEW.new_account AND NEW.old_lifecycle=1 AND NEW.new_lifecycle=2
     WHEN d.command='UnlockBazaarCharacter' THEN NEW.old_account=NEW.new_account AND NEW.old_lifecycle=2 AND NEW.new_lifecycle=1
     ELSE false END)) THEN RAISE EXCEPTION 'Bazaar Character transition requires its exact committed authority decision' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_bazaar_transition_proven AFTER INSERT ON game_bazaar_character_transitions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bazaar_transition_proven();
REVOKE ALL ON game_bazaar_character_revision_chain,game_bazaar_xp_revision_chain FROM PUBLIC;
GRANT SELECT ON game_bazaar_character_revision_chain,game_bazaar_xp_revision_chain TO oteryn_game_runtime,oteryn_game_control;
DO $$ DECLARE n TEXT;BEGIN
 FOR n IN SELECT oid::regprocedure::text FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname LIKE 'game_bazaar_%' LOOP EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());END LOOP;
END $$;

ALTER TABLE game_bazaar_locks
 ADD COLUMN transfer_sale_id UUID,
 ADD COLUMN transfer_funding BYTEA CHECK(octet_length(transfer_funding)<=1024),
 ADD COLUMN transfer_buyer UUID;
CREATE TABLE game_bazaar_signing_keys (
 kid TEXT PRIMARY KEY CHECK(octet_length(kid) BETWEEN 1 AND 64),
 public_key BYTEA CHECK(octet_length(public_key)=32),
 revoked BOOLEAN NOT NULL,
 CHECK(public_key IS NOT NULL OR revoked)
);
CREATE FUNCTION game_bazaar_signing_key_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.kid=OLD.kid AND NEW.public_key IS NOT DISTINCT FROM OLD.public_key
  AND NOT OLD.revoked AND NEW.revoked THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'Bazaar kid binding is immutable and revocation irreversible' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_bazaar_signing_key_guard BEFORE UPDATE OR DELETE ON game_bazaar_signing_keys FOR EACH ROW EXECUTE FUNCTION game_bazaar_signing_key_guard();
CREATE TRIGGER game_bazaar_signing_keys_no_truncate BEFORE TRUNCATE ON game_bazaar_signing_keys EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_bazaar_signing_keys FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE(revoked) ON game_bazaar_signing_keys TO oteryn_game_runtime;
GRANT SELECT ON game_bazaar_signing_keys TO oteryn_game_control;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_bazaar_signing_key_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;
-- Key custody policy is retained when configuration changes, independently of a
-- later PENDING command transaction. Removing a revoked kid cannot erase it.
CREATE FUNCTION game_bazaar_configure_signing_keys() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE k TEXT; v TEXT; old_key BYTEA; is_revoked BOOLEAN;
BEGIN
 IF jsonb_typeof(coalesce(NEW.policy->'signing_public_keys','{}'::jsonb))<>'object'
  OR (SELECT count(*) FROM jsonb_object_keys(coalesce(NEW.policy->'signing_public_keys','{}'::jsonb)))>2
  OR jsonb_typeof(coalesce(NEW.policy->'revoked_signing_kids','[]'::jsonb))<>'array'
  OR jsonb_array_length(coalesce(NEW.policy->'revoked_signing_kids','[]'::jsonb))>1024 THEN
  RAISE EXCEPTION 'invalid Bazaar signing trust profile' USING ERRCODE='23514'; END IF;
 FOR k IN SELECT jsonb_array_elements_text(coalesce(NEW.policy->'revoked_signing_kids','[]'::jsonb)) LOOP
  IF k !~ '^[a-z0-9][a-z0-9._:/-]{0,63}$' THEN RAISE EXCEPTION 'invalid Bazaar revoked kid' USING ERRCODE='23514';END IF;
  INSERT INTO game_bazaar_signing_keys(kid,public_key,revoked) VALUES(k,NULL,true)
   ON CONFLICT(kid) DO UPDATE SET revoked=true WHERE NOT game_bazaar_signing_keys.revoked;
 END LOOP;
 FOR k,v IN SELECT key,value FROM jsonb_each_text(coalesce(NEW.policy->'signing_public_keys','{}'::jsonb)) LOOP
  IF k !~ '^[a-z0-9][a-z0-9._:/-]{0,63}$' OR v !~ '^[0-9a-f]{64}$' THEN
   RAISE EXCEPTION 'invalid Bazaar public key binding' USING ERRCODE='23514';END IF;
  INSERT INTO game_bazaar_signing_keys(kid,public_key,revoked) VALUES(k,decode(v,'hex'),false) ON CONFLICT DO NOTHING;
  SELECT public_key,revoked INTO old_key,is_revoked FROM game_bazaar_signing_keys WHERE kid=k;
  IF is_revoked OR old_key IS DISTINCT FROM decode(v,'hex') THEN
   RAISE EXCEPTION 'Bazaar signing kid cannot be rebound or re-trusted' USING ERRCODE='23514';END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER game_bazaar_configure_signing_keys BEFORE UPDATE ON game_bazaar_configuration FOR EACH ROW EXECUTE FUNCTION game_bazaar_configure_signing_keys();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_bazaar_configure_signing_keys() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;
REVOKE ALL ON FUNCTION game_bazaar_configure_signing_keys() FROM PUBLIC;

-- Bazaar item moves preserve native ItemInstances and all supported attributes.
-- Split identities are conserved outputs of an existing instance, not Store MINTs.
ALTER TABLE game_bazaar_escrows DROP CONSTRAINT game_bazaar_escrows_item_instance_id_fkey;
ALTER TABLE game_bazaar_escrows ADD CONSTRAINT game_bazaar_escrows_item_instance_id_fkey
 FOREIGN KEY(item_instance_id) REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE game_bazaar_item_transitions (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 escrow_id UUID NOT NULL REFERENCES game_bazaar_escrows(escrow_id) DEFERRABLE INITIALLY DEFERRED,
 item_instance_id UUID NOT NULL CHECK(game_character_is_uuid_v7(item_instance_id)),
 source_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 kind TEXT NOT NULL CHECK(kind IN('ESCROW','DELIVER','RETURN')),
 before_item JSONB NOT NULL CHECK(jsonb_typeof(before_item)='object'), before_entry JSONB,
 quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 4294967295), split BOOLEAN NOT NULL,
 source_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 destination_character_id UUID NULL REFERENCES game_character_roots(character_id),
 issuer TEXT NOT NULL CHECK(issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(issuer,operation_id) REFERENCES game_bazaar_decisions(issuer,operation_id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((kind='ESCROW')=(before_entry IS NOT NULL)),
 CHECK((kind<>'ESCROW')=(destination_character_id IS NOT NULL)),
 CHECK(NOT split OR (kind='ESCROW' AND item_instance_id<>source_item_instance_id)),
 CHECK(split OR item_instance_id=source_item_instance_id)
);
CREATE INDEX game_bazaar_item_transitions_item ON game_bazaar_item_transitions(item_instance_id);
CREATE INDEX game_bazaar_item_transitions_source ON game_bazaar_item_transitions(source_item_instance_id);
CREATE INDEX game_bazaar_item_transitions_escrow ON game_bazaar_item_transitions(escrow_id);
CREATE INDEX game_bazaar_item_transitions_source_character ON game_bazaar_item_transitions(source_character_id);
CREATE INDEX game_bazaar_item_transitions_destination ON game_bazaar_item_transitions(destination_character_id);
CREATE TRIGGER game_bazaar_item_transition_immutable BEFORE UPDATE OR DELETE ON game_bazaar_item_transitions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_bazaar_item_transition_no_truncate BEFORE TRUNCATE ON game_bazaar_item_transitions EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_bazaar_item_transitions FROM PUBLIC;
GRANT SELECT,INSERT ON game_bazaar_item_transitions TO oteryn_game_runtime;
GRANT SELECT ON game_bazaar_item_transitions TO oteryn_game_control;

CREATE FUNCTION game_bazaar_item_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE p game_bazaar_item_transitions; e game_bazaar_escrows; d game_bazaar_decisions;
 i game_item_instances; src game_item_instances; intent JSONB; result JSONB;
 source_tier SMALLINT; target_tier SMALLINT; attrs BYTEA;
BEGIN
 SELECT * INTO STRICT p FROM game_bazaar_item_transitions WHERE transaction_id=p_transaction;
 SELECT * INTO STRICT e FROM game_bazaar_escrows WHERE escrow_id=p.escrow_id;
 SELECT * INTO STRICT d FROM game_bazaar_decisions WHERE issuer=p.issuer AND operation_id=p.operation_id;
 SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=p.item_instance_id;
 SELECT * INTO STRICT src FROM game_item_instances WHERE item_instance_id=p.source_item_instance_id;
 intent:=convert_from(d.intent,'UTF8')::jsonb;result:=convert_from(d.receipt,'UTF8')::jsonb;
 IF p.created_xact_id<>pg_current_xact_id() OR d.created_xact_id<>pg_current_xact_id()
    OR d.result_code<>'CHAR_CMD_OK' OR result->>'outcome'<>'COMMITTED'
    OR d.transaction_id<>p.transaction_id OR d.world_id IS DISTINCT FROM e.world_id
    OR (result->'result_data'->>'escrow_id')::uuid<>p.escrow_id
    OR e.last_operation_id<>p.operation_id OR e.item_instance_id<>p.item_instance_id
    OR e.world_id<>(intent->>'world_id')::uuid OR e.listing_id<>(intent->>'listing_id')::uuid
    OR e.seller_account_id<>(intent->>'seller_account_id')::uuid
    OR i.world_id<>e.world_id OR i.quantity<>p.quantity OR i.lifecycle<>1
    OR (result->'result_data'->'item'->>'item_instance_id')::uuid<>i.item_instance_id
    OR i.definition_family NOT IN ('Item','item')
    OR (result->'result_data'->'item'->'definition'->>'family') IS DISTINCT FROM
       (CASE WHEN i.definition_family IN ('Item','item') THEN 'item' ELSE NULL END)
    OR (result->'result_data'->'item'->'definition'->>'production_key') IS DISTINCT FROM i.definition_production_key
    OR (result->'result_data'->'item'->'definition'->>'revision_ref') IS DISTINCT FROM i.definition_revision_ref
    OR (result->'result_data'->'item'->>'quantity')::bigint<>i.quantity
    OR (result->'result_data'->'item'->>'state_revision')::numeric<>i.state_revision
    OR decode(result->'result_data'->'item'->>'attributes_hash','hex')<>e.attributes_hash
    OR sha256(e.attributes)<>e.attributes_hash THEN
  RAISE EXCEPTION 'Bazaar item move lacks exact committed authority evidence' USING ERRCODE='23514';
 END IF;
 SELECT tier INTO source_tier FROM game_item_tiers WHERE item_instance_id=p.source_item_instance_id;
 SELECT tier INTO target_tier FROM game_item_tiers WHERE item_instance_id=p.item_instance_id;
 attrs:=convert_to(CASE WHEN target_tier IS NULL THEN '{}' ELSE '{"tier":"'||target_tier::text||'"}' END,'UTF8');
 IF attrs<>e.attributes OR source_tier IS DISTINCT FROM target_tier THEN
  RAISE EXCEPTION 'Bazaar item attributes are not conserved' USING ERRCODE='23514';END IF;
 IF p.kind='ESCROW' THEN
  IF d.command<>'EscrowBazaarItem' OR e.state<>'ESCROWED' OR e.revision<>1
    OR e.escrow_receipt_digest IS DISTINCT FROM d.receipt_digest
    OR (intent->>'item_instance_id')::uuid<>p.source_item_instance_id
    OR (intent->>'quantity')::bigint<>p.quantity
    OR (intent->>'seller_character_id')::uuid<>p.source_character_id
    OR (p.before_entry->>'character_id')::uuid<>p.source_character_id
    OR (p.before_entry->>'item_instance_id')::uuid<>p.source_item_instance_id
    OR NOT EXISTS(SELECT 1 FROM game_bazaar_item_locations l WHERE l.item_instance_id=i.item_instance_id AND l.escrow_id=e.escrow_id)
    OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=p.source_character_id AND r.account_id=e.seller_account_id AND r.world_id=e.world_id AND r.lifecycle=1)
    OR (to_jsonb(src)-ARRAY['quantity','state_revision','last_transaction_id']) IS DISTINCT FROM
       (p.before_item-ARRAY['quantity','state_revision','last_transaction_id'])
    OR src.state_revision<>(p.before_item->>'state_revision')::numeric+1
    OR src.last_transaction_id<>p.transaction_id THEN
   RAISE EXCEPTION 'Bazaar escrow source does not match its conserved instance' USING ERRCODE='23514';END IF;
  IF p.split THEN
   IF src.quantity+p.quantity<>(p.before_item->>'quantity')::bigint OR src.quantity<1
    OR i.state_revision<>1 OR i.minted_transaction_id<>p.transaction_id OR i.last_transaction_id IS NOT NULL
    OR (to_jsonb(i)-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision']) IS DISTINCT FROM
       (p.before_item-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision'])
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.item_instance_id=src.item_instance_id AND to_jsonb(c)=p.before_entry) THEN
    RAISE EXCEPTION 'Bazaar split must conserve complete instance state and quantity' USING ERRCODE='23514';END IF;
  ELSE
   IF src.quantity<>(p.before_item->>'quantity')::bigint
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=src.item_instance_id) THEN
    RAISE EXCEPTION 'Bazaar whole escrow must remove exactly its source entry' USING ERRCODE='23514';END IF;
  END IF;
 ELSE
  IF d.command<>(CASE p.kind WHEN 'DELIVER' THEN 'DeliverBazaarItem' ELSE 'ReturnBazaarItem' END)
   OR e.escrow_receipt_digest IS DISTINCT FROM decode(intent->>'escrow_receipt_digest','hex')
   OR e.state<>(CASE p.kind WHEN 'DELIVER' THEN 'DELIVERED' ELSE 'RETURNED' END)
   OR e.revision<>(intent->>'expected_escrow_revision')::numeric+1
   OR e.destination_character_id<>p.destination_character_id
   OR (to_jsonb(i)-ARRAY['state_revision','last_transaction_id']) IS DISTINCT FROM (p.before_item-ARRAY['state_revision','last_transaction_id'])
   OR i.state_revision<>(p.before_item->>'state_revision')::numeric+1 OR i.last_transaction_id<>p.transaction_id
   OR EXISTS(SELECT 1 FROM game_bazaar_item_locations WHERE item_instance_id=i.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_character_inbox_locations l JOIN game_character_inbox_deliveries b USING(item_instance_id,character_id,world_id,ordinal)
      JOIN game_character_roots r ON r.character_id=l.character_id
      WHERE l.item_instance_id=i.item_instance_id AND l.character_id=p.destination_character_id AND l.world_id=e.world_id
        AND r.account_id=e.destination_account_id AND r.world_id=e.world_id AND r.lifecycle=1
        AND b.cause_kind=(CASE p.kind WHEN 'DELIVER' THEN 'BAZAAR_DELIVERY' ELSE 'BAZAAR_RETURN' END)
        AND b.cause_ref=p.operation_id::text AND b.created_xact_id=pg_current_xact_id())
   OR (p.kind='RETURN' AND e.destination_account_id<>e.seller_account_id) THEN
   RAISE EXCEPTION 'Bazaar delivery/return must consume escrow into the exact authorized Inbox' USING ERRCODE='23514';END IF;
 END IF;
END $$;
CREATE FUNCTION game_bazaar_item_consistent() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 PERFORM game_bazaar_item_complete(NEW.transaction_id);RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_bazaar_item_consistent AFTER INSERT ON game_bazaar_item_transitions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bazaar_item_consistent();

-- Prepend narrow, same-transaction Bazaar arms to the current owner guards.
-- Every previous owner arm and its rejection path remain unchanged.
DO $migration$ DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_mint_consistency_guard','game_forge_tier_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'Bazaar item guard anchor missing: %',n;END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_bazaar_item_transitions p WHERE p.source_item_instance_id=OLD.item_instance_id
      AND p.created_xact_id=pg_current_xact_id() AND p.transaction_id=NEW.last_transaction_id AND p.before_item=to_jsonb(OLD)) THEN
    IF (to_jsonb(NEW)-ARRAY['quantity','state_revision','last_transaction_id']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['quantity','state_revision','last_transaction_id'])
      OR NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<1 THEN
     RAISE EXCEPTION 'Bazaar item successor changes only conserved quantity/revision' USING ERRCODE='23514';END IF;
    RETURN NEW;
   END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_bazaar_item_transitions p WHERE p.transaction_id=NEW.last_transaction_id
      AND p.source_item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_bazaar_item_complete(NEW.last_transaction_id);RETURN NULL;END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_bazaar_item_transitions p WHERE p.kind='ESCROW' AND NOT p.split
      AND p.source_item_instance_id=OLD.item_instance_id AND p.before_entry=to_jsonb(OLD) AND p.created_xact_id=pg_current_xact_id()) THEN
    RETURN NULL;END IF;
$arm$;
  ELSIF n='game_item_mint_consistency_guard' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_bazaar_item_transitions p WHERE p.kind='ESCROW' AND p.split
      AND p.item_instance_id=NEW.item_instance_id AND p.transaction_id=NEW.minted_transaction_id AND p.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_bazaar_item_complete(NEW.minted_transaction_id);RETURN NULL;END IF;
$arm$;
  ELSE arm:=$arm$
   IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_bazaar_item_transitions p JOIN game_item_tiers t ON t.item_instance_id=p.source_item_instance_id
      WHERE p.split AND p.kind='ESCROW' AND p.item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id() AND t.tier=NEW.tier) THEN
    RETURN NEW;END IF;
$arm$;
  END IF;
  body:=overlay(body PLACING E'BEGIN\n'||arm FROM begin_at FOR 5);EXECUTE body;
 END LOOP;
END $migration$;

CREATE FUNCTION game_bazaar_attributes_frozen() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE item UUID;
BEGIN
 item:=CASE WHEN TG_OP='DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
 IF EXISTS(SELECT 1 FROM game_bazaar_item_locations WHERE item_instance_id=item) THEN
  RAISE EXCEPTION 'Bazaar custody preserves exact frozen attributes' USING ERRCODE='23514';END IF;
 RETURN CASE WHEN TG_OP='DELETE' THEN OLD ELSE NEW END;
END $$;
DO $$ DECLARE n TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_bindings','game_item_fluid_states','game_item_tiers','game_item_keys','game_item_timed_states','game_item_texts','game_item_text_revisions','game_item_imbuements'] LOOP
 EXECUTE format('CREATE TRIGGER game_bazaar_attributes_frozen BEFORE INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_bazaar_attributes_frozen()',n);END LOOP;
 FOR n IN SELECT oid::regprocedure::text FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname IN ('game_bazaar_item_complete','game_bazaar_item_consistent','game_bazaar_attributes_frozen') LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());END LOOP;
END $$;

-- Custody cannot be inserted or consumed by a free-standing Inbox call. Both
-- sides require this transaction's exact Bazaar operation, including rejects.
ALTER TABLE game_bazaar_item_transitions ADD CONSTRAINT game_bazaar_item_transition_instance_fkey
 FOREIGN KEY(item_instance_id) REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED;
CREATE FUNCTION game_bazaar_location_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE transition UUID; item UUID; escrow UUID;
BEGIN
 item:=CASE WHEN TG_OP='DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
 escrow:=CASE WHEN TG_OP='DELETE' THEN OLD.escrow_id ELSE NEW.escrow_id END;
 SELECT p.transaction_id INTO transition FROM game_bazaar_item_transitions p
  WHERE p.item_instance_id=item AND p.escrow_id=escrow AND p.created_xact_id=pg_current_xact_id()
    AND (CASE WHEN TG_OP='INSERT' THEN p.kind='ESCROW' ELSE p.kind IN('DELIVER','RETURN') END);
 IF transition IS NULL THEN RAISE EXCEPTION 'Bazaar custody requires its exact current operation' USING ERRCODE='23514';END IF;
 PERFORM game_bazaar_item_complete(transition);RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_bazaar_location_operation_proven AFTER INSERT OR DELETE ON game_bazaar_item_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bazaar_location_operation_proven();
DO $$BEGIN EXECUTE format('ALTER FUNCTION game_bazaar_location_operation_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;

-- An admitted empty container remains empty while Bazaar owns it. A stale
-- inventory handle, native script or another source cannot add hidden contents.
CREATE FUNCTION game_bazaar_contents_frozen() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_bazaar_item_locations WHERE item_instance_id=NEW.parent_item_instance_id) THEN
  RAISE EXCEPTION 'Bazaar custody cannot gain container contents' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
DO $$ DECLARE n TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_container_entries','game_item_nested_entries','game_item_corpse_container_entries'] LOOP
 EXECUTE format('CREATE TRIGGER game_bazaar_contents_frozen BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION game_bazaar_contents_frozen()',n);END LOOP;
 EXECUTE format('ALTER FUNCTION game_bazaar_contents_frozen() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- GPROD-BAZAAR extends the existing Account+World bank ledger, not a second
-- balance or ledger. Kind NULL is the existing typed domain-cause convention.
ALTER TABLE game_account_bank_entries
 ADD COLUMN bazaar_issuer TEXT CHECK(bazaar_issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'),
 ADD COLUMN bazaar_operation_id UUID,
 ADD COLUMN bazaar_kind TEXT CHECK(bazaar_kind IN('BAZAAR_DEBIT','BAZAAR_CREDIT')),
 ADD CONSTRAINT game_account_bank_entries_bazaar_reference CHECK(
  (bazaar_operation_id IS NOT NULL)=(bazaar_issuer IS NOT NULL)
  AND (bazaar_operation_id IS NOT NULL)=(bazaar_kind IS NOT NULL)),
 ADD CONSTRAINT game_account_bank_entries_bazaar_fkey FOREIGN KEY(bazaar_issuer,bazaar_operation_id)
  REFERENCES game_bazaar_decisions(issuer,operation_id) DEFERRABLE INITIALLY DEFERRED,
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_guild_reference,
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,npc_trade_transaction_id,house_operation_id,guild_operation_id,market_transaction_id,house_rent_payment_id,bazaar_operation_id)=1),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
  (guild_operation_id IS NOT NULL)=(guild_kind IS NOT NULL)
  AND (market_transaction_id IS NOT NULL)=(market_kind IS NOT NULL)
  AND (house_rent_payment_id IS NOT NULL)=(house_rent_kind IS NOT NULL)
  AND (kind IS NULL)=(guild_kind IS NOT NULL OR market_kind IS NOT NULL OR house_rent_kind IS NOT NULL OR bazaar_kind IS NOT NULL));
CREATE UNIQUE INDEX game_bazaar_bank_operation_kind ON game_account_bank_entries(bazaar_issuer,bazaar_operation_id,bazaar_kind) WHERE bazaar_operation_id IS NOT NULL;
DO $$ DECLARE expr TEXT;BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expr FROM pg_constraint
 WHERE conrelid='game_account_bank_entries'::regclass AND conname='game_account_bank_entries_house_value';
 ALTER TABLE game_account_bank_entries DROP CONSTRAINT game_account_bank_entries_house_value;
 EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT game_account_bank_entries_house_value CHECK(CASE WHEN bazaar_kind IS NOT NULL THEN escrow_before IS NULL AND escrow_after IS NULL AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND balance_after=balance_before+CASE bazaar_kind WHEN ''BAZAAR_CREDIT'' THEN amount ELSE -amount END ELSE (%s) END)',expr);
END $$;
CREATE TABLE game_bazaar_gold_burns (
 burn_entry_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(burn_entry_id)),
 issuer TEXT NOT NULL,operation_id UUID NOT NULL,world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1 AND 999999999999),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(issuer,operation_id),
 FOREIGN KEY(issuer,operation_id) REFERENCES game_bazaar_decisions(issuer,operation_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TRIGGER game_bazaar_gold_burn_immutable BEFORE UPDATE OR DELETE ON game_bazaar_gold_burns FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_bazaar_gold_burn_no_truncate BEFORE TRUNCATE ON game_bazaar_gold_burns EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_bazaar_gold_burns FROM PUBLIC;
GRANT SELECT,INSERT ON game_bazaar_gold_burns TO oteryn_game_runtime;
GRANT SELECT ON game_bazaar_gold_burns TO oteryn_game_control;

-- Same-physical-transaction proof, including the existing ledger chain, exact
-- receipt identities, both Account+World parties and debit=credit+burn.
CREATE FUNCTION game_bazaar_payout_proven(i TEXT,o UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE d game_bazaar_decisions; intent JSONB; facts JSONB; debit game_account_bank_entries; credit game_account_bank_entries; burn game_bazaar_gold_burns;
BEGIN
 SELECT * INTO d FROM game_bazaar_decisions WHERE issuer=i AND operation_id=o;
 IF NOT FOUND OR d.command<>'PayoutBazaarGold' OR d.result_code<>'CHAR_CMD_OK' OR d.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Bazaar bank ledger has no same-transaction payout decision' USING ERRCODE='23514';END IF;
 intent:=convert_from(d.intent,'UTF8')::jsonb;facts:=convert_from(d.receipt,'UTF8')::jsonb->'result_data';
 SELECT * INTO debit FROM game_account_bank_entries WHERE bazaar_issuer=i AND bazaar_operation_id=o AND bazaar_kind='BAZAAR_DEBIT';
 SELECT * INTO credit FROM game_account_bank_entries WHERE bazaar_issuer=i AND bazaar_operation_id=o AND bazaar_kind='BAZAAR_CREDIT';
 SELECT * INTO burn FROM game_bazaar_gold_burns WHERE issuer=i AND operation_id=o;
 IF facts IS NULL OR jsonb_typeof(facts)<>'object' OR debit.entry_id IS NULL OR credit.entry_id IS NULL OR burn.burn_entry_id IS NULL
  OR debit.created_xact_id<>pg_current_xact_id() OR credit.created_xact_id<>pg_current_xact_id() OR burn.created_xact_id<>pg_current_xact_id()
  OR debit.account_id IS DISTINCT FROM (intent->>'buyer_account_id')::uuid OR credit.account_id IS DISTINCT FROM (intent->>'seller_account_id')::uuid
  OR d.world_id IS DISTINCT FROM debit.world_id OR debit.account_id=credit.account_id OR debit.world_id IS DISTINCT FROM (intent->>'world_id')::uuid OR credit.world_id<>debit.world_id OR burn.world_id<>debit.world_id
  OR debit.amount IS DISTINCT FROM (intent->>'gross_amount')::bigint OR credit.amount IS DISTINCT FROM (intent->>'seller_amount')::bigint OR burn.amount IS DISTINCT FROM (intent->>'fee_amount')::bigint
  OR debit.amount<>credit.amount+burn.amount OR burn.amount<>ceil(debit.amount::numeric*250/10000)
  OR facts->>'buyer_ledger_entry_id' IS DISTINCT FROM debit.entry_id::text OR facts->>'seller_ledger_entry_id' IS DISTINCT FROM credit.entry_id::text OR facts->>'burn_entry_id' IS DISTINCT FROM burn.burn_entry_id::text
  OR NOT EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=debit.account_id AND b.world_id=debit.world_id AND b.last_entry_id=debit.entry_id AND b.balance=debit.balance_after)
  OR NOT EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=credit.account_id AND b.world_id=credit.world_id AND b.last_entry_id=credit.entry_id AND b.balance=credit.balance_after)
  OR (debit.previous_entry_id IS NULL AND debit.balance_before<>0)
  OR (credit.previous_entry_id IS NULL AND credit.balance_before<>0)
  OR (debit.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=debit.previous_entry_id AND p.account_id=debit.account_id AND p.world_id=debit.world_id AND p.balance_after=debit.balance_before))
  OR (credit.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=credit.previous_entry_id AND p.account_id=credit.account_id AND p.world_id=credit.world_id AND p.balance_after=credit.balance_before)) THEN
  RAISE EXCEPTION 'Bazaar payout must commit exact chained debit credit burn and receipt together' USING ERRCODE='23514';END IF;
END $$;
CREATE FUNCTION game_bazaar_bank_entry_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 PERFORM game_bazaar_payout_proven(NEW.bazaar_issuer,NEW.bazaar_operation_id);RETURN NULL;END $$;
CREATE FUNCTION game_bazaar_gold_burn_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 PERFORM game_bazaar_payout_proven(NEW.issuer,NEW.operation_id);RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_bazaar_bank_entry_proof AFTER INSERT ON game_account_bank_entries DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.bazaar_operation_id IS NOT NULL) EXECUTE FUNCTION game_bazaar_bank_entry_guard();
CREATE CONSTRAINT TRIGGER game_bazaar_gold_burn_proof AFTER INSERT ON game_bazaar_gold_burns DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bazaar_gold_burn_guard();

-- Hold and incoming-headroom totals are checked while the exact same existing
-- bank rows are locked; all ordinary debits/credits also hit 0994's balance fence.
CREATE FUNCTION game_bazaar_hold_bank_totals() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE held NUMERIC;incoming NUMERIC;buyer_total BIGINT;seller_total BIGINT;native_total NUMERIC;
BEGIN
 PERFORM 1 FROM game_account_bank_balances WHERE account_id IN(NEW.buyer_account_id,NEW.seller_account_id) AND world_id=NEW.world_id ORDER BY account_id FOR UPDATE;
 SELECT balance INTO buyer_total FROM game_account_bank_balances WHERE account_id=NEW.buyer_account_id AND world_id=NEW.world_id;
 SELECT balance INTO seller_total FROM game_account_bank_balances WHERE account_id=NEW.seller_account_id AND world_id=NEW.world_id;
 SELECT coalesce(sum(amount),0) INTO held FROM game_bazaar_gold_holds WHERE buyer_account_id=NEW.buyer_account_id AND world_id=NEW.world_id AND amount>0;
 SELECT coalesce(sum(incoming_net),0) INTO incoming FROM game_bazaar_gold_holds WHERE seller_account_id=NEW.seller_account_id AND world_id=NEW.world_id AND incoming_net>0;
 native_total:=9000000000000000-game_account_bank_credit_headroom_before_bazaar(NEW.seller_account_id,NEW.world_id);
 IF buyer_total IS NULL OR seller_total IS NULL OR held>buyer_total OR buyer_total>999999999999 OR greatest(seller_total,native_total)+incoming>999999999999 THEN
 RAISE EXCEPTION 'Bazaar hold funds and seller incoming headroom must remain covered' USING ERRCODE='23514';END IF;RETURN NULL;
END $$;
CREATE TRIGGER game_bazaar_hold_bank_totals AFTER INSERT OR UPDATE ON game_bazaar_gold_holds FOR EACH ROW EXECUTE FUNCTION game_bazaar_hold_bank_totals();

ALTER FUNCTION game_account_bank_credit_headroom(UUID,UUID) RENAME TO game_account_bank_credit_headroom_before_bazaar;
CREATE FUNCTION game_account_bank_credit_headroom(p_account UUID,p_world UUID) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE WHEN coalesce((SELECT sum(incoming_net) FROM game_bazaar_gold_holds WHERE seller_account_id=p_account AND world_id=p_world AND incoming_net>0),0)>0 THEN
 least(game_account_bank_credit_headroom_before_bazaar(p_account,p_world),greatest(0,999999999999-coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=p_account AND world_id=p_world),0)-coalesce((SELECT sum(incoming_net) FROM game_bazaar_gold_holds WHERE seller_account_id=p_account AND world_id=p_world AND incoming_net>0),0)))::bigint
 ELSE game_account_bank_credit_headroom_before_bazaar(p_account,p_world) END
$$;
REVOKE ALL ON FUNCTION game_account_bank_credit_headroom(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_bank_credit_headroom(UUID,UUID) TO oteryn_game_runtime;
DO $$ DECLARE n TEXT;BEGIN
 FOR n IN SELECT oid::regprocedure::text FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND (proname LIKE 'game_bazaar_%' OR proname='game_account_bank_credit_headroom') LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());END LOOP;
END $$;

CREATE TABLE game_bazaar_projection_state (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), source TEXT NOT NULL,
 epoch NUMERIC(20) NOT NULL CHECK(epoch BETWEEN 1 AND 18446744073709551615),
 enumeration_cursor TEXT NOT NULL, enumeration_done BOOLEAN NOT NULL,
 started_at BIGINT NOT NULL
);
REVOKE ALL ON game_bazaar_projection_state FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_bazaar_projection_state TO oteryn_game_runtime;
GRANT SELECT ON game_bazaar_projection_state TO oteryn_game_control;
CREATE TRIGGER game_bazaar_projection_no_truncate BEFORE TRUNCATE ON game_bazaar_projection_state EXECUTE FUNCTION game_character_reject_truncate();
ALTER TABLE game_bazaar_projection_state ADD COLUMN version BIGINT NOT NULL DEFAULT 1 CHECK(version>0), ADD COLUMN policy_digest BYTEA NULL CHECK(octet_length(policy_digest)=32), ADD COLUMN last_observed BIGINT NOT NULL DEFAULT 0;
ALTER TABLE game_bazaar_publication_outbox ADD COLUMN enumeration_version BIGINT NOT NULL DEFAULT 0;
-- Scope assignment and content owners do not have Bazaar projection write grants.
-- Only this trigger updates the projection, under its pinned migration-owner path.
CREATE FUNCTION game_bazaar_projection_dirty() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
 UPDATE game_bazaar_projection_state SET version=version+1,enumeration_cursor='',enumeration_done=false WHERE singleton;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_bazaar_projection_dirty() FROM PUBLIC;
CREATE TRIGGER game_bazaar_new_world AFTER INSERT OR UPDATE ON game_runtime_scope_assignments FOR EACH STATEMENT EXECUTE FUNCTION game_bazaar_projection_dirty();
CREATE TRIGGER game_bazaar_new_root AFTER INSERT ON game_character_roots FOR EACH STATEMENT EXECUTE FUNCTION game_bazaar_projection_dirty();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_bazaar_projection_dirty() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;
CREATE TRIGGER game_bazaar_content_changed AFTER INSERT OR UPDATE OR DELETE ON game_content_activations FOR EACH STATEMENT EXECUTE FUNCTION game_bazaar_projection_dirty();

-- Every Character-owned writer joins the root lock, including offline Inbox,
-- mail and account-bank causes. This is additive to each existing guard chain.
-- Existing domain owners need no direct root privileges to enforce this fence.
-- The search_path is pinned below before any domain writer can run the trigger.
CREATE FUNCTION game_bazaar_frozen_character_writer() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID; col TEXT; record JSONB; lifecycle SMALLINT;
BEGIN
 record:=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 FOREACH col IN ARRAY TG_ARGV LOOP
 c:=(record->>col)::uuid;
 IF c IS NULL THEN CONTINUE; END IF;
 SELECT r.lifecycle INTO lifecycle FROM game_character_roots r WHERE r.character_id=c FOR SHARE;
 IF lifecycle=2 AND NOT(TG_TABLE_NAME='game_account_bank_entries' AND record->>'bazaar_kind' IN('BAZAAR_DEBIT','BAZAAR_CREDIT') AND record->>'bazaar_operation_id' IS NOT NULL) AND NOT(TG_TABLE_NAME='game_character_progression_state' AND EXISTS(
   SELECT 1 FROM game_bazaar_character_transitions t WHERE t.character_id=c AND t.created_xact_id=pg_current_xact_id()
    AND t.new_revision=(record->>'character_revision')::numeric)) THEN
 RAISE EXCEPTION 'Bazaar locked Character denies owned-state mutation' USING ERRCODE='23514';END IF;
 END LOOP;
 IF TG_OP='DELETE' THEN RETURN OLD;ELSE RETURN NEW;END IF;
END $$;
REVOKE ALL ON FUNCTION game_bazaar_frozen_character_writer() FROM PUBLIC;
DO $$ DECLARE t RECORD; args TEXT;BEGIN
 FOR t IN SELECT c.conrelid AS relation,array_agg(DISTINCT a.attname) AS cols FROM pg_constraint c
 JOIN LATERAL unnest(c.conkey,c.confkey) AS keys(local_column,root_column) ON true
 JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=keys.local_column
 JOIN pg_attribute parent ON parent.attrelid=c.confrelid AND parent.attnum=keys.root_column AND parent.attname='character_id'
 WHERE c.contype='f' AND c.confrelid='game_character_roots'::regclass
 AND c.conrelid::regclass::text NOT LIKE 'game_bazaar_%'
 GROUP BY c.conrelid LOOP
 SELECT string_agg(quote_literal(col),',') INTO args FROM unnest(t.cols) col;
 EXECUTE format('CREATE TRIGGER game_bazaar_frozen_character BEFORE INSERT OR UPDATE OR DELETE ON %s FOR EACH ROW EXECUTE FUNCTION game_bazaar_frozen_character_writer(%s)',t.relation::regclass,args);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_bazaar_frozen_character_writer() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- Existing ANL envelope, schema revision1; no Bazaar-specific envelope or HTTP
-- endpoint. Atomic immutable decision evidence is distinct from finite audit.
CREATE FUNCTION game_bazaar_varint(n NUMERIC) RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE result BYTEA:=''::bytea;part INT;BEGIN
 IF n<0 OR n>18446744073709551615 THEN RAISE EXCEPTION 'invalid audit varint';END IF;
 LOOP part:=mod(n,128)::int;n:=trunc(n/128);IF n>0 THEN part:=part+128;END IF;result:=result||set_byte(decode('00','hex'),0,part);EXIT WHEN n=0;END LOOP;RETURN result;END $$;
CREATE FUNCTION game_bazaar_bytes(tag INT,body BYTEA) RETURNS BYTEA LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT game_bazaar_varint(tag*8+2)||game_bazaar_varint(octet_length(body))||body
$$;
CREATE TABLE game_bazaar_audit_outbox (
 event_id UUID PRIMARY KEY REFERENCES game_bazaar_decisions(event_id),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 1024),
 envelope_digest BYTEA GENERATED ALWAYS AS(sha256(envelope)) STORED,
 occurred_at BIGINT NOT NULL,expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+7776000000),
 acknowledged BOOLEAN NOT NULL DEFAULT false
);
CREATE TABLE game_bazaar_audit_legal_holds (
 event_id UUID PRIMARY KEY REFERENCES game_bazaar_audit_outbox(event_id),
 authorizing_actor TEXT NOT NULL CHECK(octet_length(authorizing_actor) BETWEEN 1 AND 128),
 reason TEXT NOT NULL CHECK(octet_length(reason) BETWEEN 1 AND 512),started_at BIGINT NOT NULL
);
CREATE FUNCTION game_bazaar_audit_decision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE envelope BYTEA;BEGIN
 envelope:=decode('0801','hex')||game_bazaar_bytes(2,uuid_send(NEW.event_id))||decode('1806200128023003','hex')
  ||game_bazaar_bytes(7,convert_to('PLAT3_AUTHORITY_AUDIT_RETENTION_V1','UTF8'))
  ||decode('40','hex')||game_bazaar_varint(NEW.decided_at::numeric*1000)
  ||CASE WHEN NEW.world_id IS NULL THEN ''::bytea ELSE game_bazaar_bytes(9,uuid_send(NEW.world_id)) END
  ||game_bazaar_bytes(17,uuid_send(NEW.operation_id))
  ||game_bazaar_bytes(18,game_bazaar_bytes(1,uuid_send(NEW.transaction_id))||decode('10011801','hex'))
  ||game_bazaar_bytes(25,convert_to(NEW.server_build_id,'UTF8'))
  ||game_bazaar_bytes(26,NEW.payload)||game_bazaar_bytes(27,sha256(NEW.payload));
 INSERT INTO game_bazaar_audit_outbox(event_id,envelope,occurred_at,expires_at) VALUES(NEW.event_id,envelope,NEW.decided_at*1000,NEW.decided_at*1000+7776000000);
 RETURN NULL;END $$;
CREATE TRIGGER game_bazaar_audit_decision AFTER INSERT ON game_bazaar_decisions FOR EACH ROW EXECUTE FUNCTION game_bazaar_audit_decision();
CREATE FUNCTION game_bazaar_audit_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 -- Stored generated values are not populated in NEW until after BEFORE triggers.
 -- Immutable envelope bytes still constrain the digest that PostgreSQL recomputes.
 IF TG_OP='UPDATE' AND (to_jsonb(NEW)-ARRAY['acknowledged','envelope_digest'])=(to_jsonb(OLD)-ARRAY['acknowledged','envelope_digest']) AND NEW.acknowledged THEN RETURN NEW;END IF;
 IF TG_OP='DELETE' AND OLD.expires_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND NOT EXISTS(SELECT 1 FROM game_bazaar_audit_legal_holds WHERE event_id=OLD.event_id) THEN RETURN OLD;END IF;
 RAISE EXCEPTION 'Bazaar audit is immutable until unheld original expiry' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_bazaar_audit_guard BEFORE UPDATE OR DELETE ON game_bazaar_audit_outbox FOR EACH ROW EXECUTE FUNCTION game_bazaar_audit_guard();
CREATE FUNCTION game_bazaar_decision_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='UPDATE' AND OLD.payload IS NOT NULL AND NEW.payload IS NULL
 AND (to_jsonb(NEW)-'payload')=(to_jsonb(OLD)-'payload')
 AND OLD.decided_at*1000+7776000000<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
 AND NOT EXISTS(SELECT 1 FROM game_bazaar_audit_legal_holds WHERE event_id=OLD.event_id) THEN RETURN NEW;END IF;
 RAISE EXCEPTION 'Bazaar authority decision is immutable' USING ERRCODE='23514';END $$;
DROP TRIGGER game_bazaar_decision_immutable ON game_bazaar_decisions;
CREATE TRIGGER game_bazaar_decision_immutable BEFORE UPDATE OR DELETE ON game_bazaar_decisions FOR EACH ROW EXECUTE FUNCTION game_bazaar_decision_immutable();
CREATE FUNCTION game_bazaar_expire_audit(max_entries INT) RETURNS INT LANGUAGE plpgsql SECURITY DEFINER AS $$ DECLARE n INT;BEGIN
 IF max_entries NOT BETWEEN 1 AND 64 THEN RAISE EXCEPTION 'invalid Bazaar audit expiry limit';END IF;
 WITH expired AS(SELECT event_id FROM game_bazaar_audit_outbox a WHERE expires_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND NOT EXISTS(SELECT 1 FROM game_bazaar_audit_legal_holds h WHERE h.event_id=a.event_id) ORDER BY expires_at,event_id LIMIT max_entries FOR UPDATE SKIP LOCKED),
 deleted AS(DELETE FROM game_bazaar_audit_outbox WHERE event_id IN(SELECT event_id FROM expired) RETURNING event_id)
 UPDATE game_bazaar_decisions SET payload=NULL WHERE event_id IN(SELECT event_id FROM deleted);GET DIAGNOSTICS n=ROW_COUNT;RETURN n;END $$;
REVOKE ALL ON game_bazaar_audit_outbox,game_bazaar_audit_legal_holds FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE(acknowledged) ON game_bazaar_audit_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_bazaar_audit_outbox,game_bazaar_audit_legal_holds TO oteryn_game_control;
GRANT SELECT ON game_bazaar_audit_legal_holds TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_bazaar_expire_audit(INT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bazaar_expire_audit(INT) TO oteryn_game_runtime;
CREATE TRIGGER game_bazaar_audit_no_truncate BEFORE TRUNCATE ON game_bazaar_audit_outbox EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_bazaar_audit_hold_no_truncate BEFORE TRUNCATE ON game_bazaar_audit_legal_holds EXECUTE FUNCTION game_character_reject_truncate();
DO $$ DECLARE n TEXT;BEGIN
 FOR n IN SELECT oid::regprocedure::text FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname LIKE 'game_bazaar_%' LOOP EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());END LOOP;
END $$;

-- Recovery admission also runs on restricted runtime connections. Keep the
-- cross-owner custody and bank checks behind read-only boolean definer ports;
-- runtime must not gain SELECT on private HouseInterior, Depot or bank tables.
-- These are the unchanged Bazaar recovery predicates, not mutation authority.
CREATE FUNCTION game_bazaar_escrows_invalid() RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $recovery$
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_escrows e
 LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
 LEFT JOIN game_item_tiers tier ON tier.item_instance_id=e.item_instance_id
 CROSS JOIN LATERAL (SELECT convert_from(e.item,'UTF8')::jsonb AS item) j
 WHERE i.item_instance_id IS NULL
 OR e.attributes_hash IS DISTINCT FROM sha256(e.attributes)
 OR j.item->>'item_instance_id' IS DISTINCT FROM e.item_instance_id::text
 OR j.item->>'world_id' IS DISTINCT FROM e.world_id::text
 OR j.item->'definition'->>'family' IS DISTINCT FROM 'item'
 OR j.item->>'attributes_hash' IS DISTINCT FROM encode(e.attributes_hash,'hex')
 OR NOT EXISTS(
  SELECT 1 FROM game_bazaar_item_transitions p JOIN game_bazaar_decisions d
   ON d.issuer=p.issuer AND d.operation_id=p.operation_id
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
   CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE p.escrow_id=e.escrow_id AND p.kind='ESCROW' AND p.item_instance_id=e.item_instance_id
   AND p.source_character_id=e.seller_character_id AND d.command='EscrowBazaarItem'
   AND d.result_code='CHAR_CMD_OK' AND d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
   AND d.receipt_digest=e.escrow_receipt_digest AND d.world_id=e.world_id
   AND q.intent->>'listing_id'=e.listing_id::text
   AND q.intent->>'seller_account_id'=e.seller_account_id::text
   AND q.intent->>'seller_character_id'=e.seller_character_id::text
   AND q.receipt->'result_data'->>'escrow_id'=e.escrow_id::text
   AND q.receipt->'result_data'->>'escrow_revision'='1'
   AND q.receipt->'result_data'->'item'->>'item_instance_id'=e.item_instance_id::text
   AND q.receipt->'result_data'->'item'->'definition'=j.item->'definition'
   AND q.receipt->'result_data'->'item'->'definition'->>'production_key'=p.before_item->>'definition_production_key'
   AND q.receipt->'result_data'->'item'->'definition'->>'revision_ref'=p.before_item->>'definition_revision_ref'
   AND (q.receipt->'result_data'->'item'->>'quantity')::bigint=p.quantity
   AND q.receipt->'result_data'->'item'->>'attributes_hash'=encode(e.attributes_hash,'hex')
   AND q.intent->>'item_instance_id'=p.source_item_instance_id::text
   AND q.intent->>'expected_state_revision'=p.before_item->>'state_revision'
   AND (e.state IN('DELIVERED','RETURNED') OR (
    (to_jsonb(i)-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision'])
      =(p.before_item-ARRAY['item_instance_id','quantity','minted_transaction_id','last_transaction_id','state_revision'])
    AND ((p.split AND i.state_revision=1 AND i.minted_transaction_id=p.transaction_id AND i.last_transaction_id IS NULL)
     OR (NOT p.split AND i.state_revision=(p.before_item->>'state_revision')::numeric+1
       AND i.last_transaction_id=p.transaction_id AND i.minted_transaction_id=(p.before_item->>'minted_transaction_id')::uuid))
   ))
 )
 OR (e.state IN('ESCROWED','INVALID') AND (
  i.lifecycle<>1 OR i.world_id IS DISTINCT FROM e.world_id
  OR j.item->'definition'->>'production_key' IS DISTINCT FROM i.definition_production_key
  OR j.item->'definition'->>'revision_ref' IS DISTINCT FROM i.definition_revision_ref
  OR i.definition_family NOT IN('Item','item')
  OR i.quantity IS DISTINCT FROM (j.item->>'quantity')::bigint
  OR i.state_revision IS DISTINCT FROM (j.item->>'state_revision')::numeric
  OR e.attributes IS DISTINCT FROM convert_to(CASE WHEN coalesce(tier.tier,0)=0
    THEN '{}' ELSE '{"tier":"'||tier.tier::text||'"}' END,'UTF8')
  OR j.item->'attributes' IS DISTINCT FROM CASE WHEN coalesce(tier.tier,0)=0
    THEN '[]'::jsonb ELSE jsonb_build_array(jsonb_build_object('key','tier','value',tier.tier::text)) END
  OR NOT EXISTS(SELECT 1 FROM game_bazaar_item_locations l
     WHERE l.item_instance_id=e.item_instance_id AND l.escrow_id=e.escrow_id)
  OR NOT EXISTS(SELECT 1 FROM game_bazaar_decisions d
    WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.operation_id=e.last_operation_id
     AND d.command='EscrowBazaarItem' AND d.result_code='CHAR_CMD_OK'
     AND convert_from(d.receipt,'UTF8')::jsonb->'result_data'->'item'=j.item)
  OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_market_escrow_locations WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_bindings WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_text_revisions WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=e.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=e.item_instance_id)
 ))
 OR (e.state IN('DELIVERED','RETURNED') AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_item_transitions p JOIN game_bazaar_decisions d
   ON d.issuer=p.issuer AND d.operation_id=p.operation_id
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb AS receipt,
   CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  JOIN game_character_inbox_deliveries inbox ON inbox.item_instance_id=p.item_instance_id
   AND inbox.cause_ref=p.operation_id::text AND inbox.character_id=p.destination_character_id
   AND inbox.world_id=e.world_id
  WHERE p.escrow_id=e.escrow_id AND p.item_instance_id=e.item_instance_id
   AND p.operation_id=e.last_operation_id AND p.destination_character_id=e.destination_character_id
   AND p.source_character_id=e.seller_character_id AND p.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY'
   AND p.kind=CASE e.state WHEN 'DELIVERED' THEN 'DELIVER' ELSE 'RETURN' END
   AND d.command=CASE e.state WHEN 'DELIVERED' THEN 'DeliverBazaarItem' ELSE 'ReturnBazaarItem' END
   AND inbox.cause_kind=CASE e.state WHEN 'DELIVERED' THEN 'BAZAAR_DELIVERY' ELSE 'BAZAAR_RETURN' END
   AND d.result_code='CHAR_CMD_OK' AND d.world_id=e.world_id
   AND q.receipt->'result_data'->>'escrow_id'=e.escrow_id::text
   AND q.receipt->'result_data'->>'escrow_revision'=e.revision::text
   AND q.receipt->'result_data'->>'destination_account_id'=e.destination_account_id::text
   AND q.receipt->'result_data'->>'destination_character_id'=e.destination_character_id::text
   AND q.receipt->'result_data'->'item'=j.item
   AND q.intent->>'escrow_receipt_digest'=encode(e.escrow_receipt_digest,'hex')
   AND q.intent->>'listing_id'=e.listing_id::text
   AND q.intent->>'seller_account_id'=e.seller_account_id::text
   AND (e.state='RETURNED' OR (q.intent->>'sale_id'=e.sale_id::text
      AND q.intent->'funding'=convert_from(e.funding,'UTF8')::jsonb))
   AND (p.before_item->>'state_revision')::numeric+1=(j.item->>'state_revision')::numeric
   AND p.quantity=(j.item->>'quantity')::bigint
 ))
) OR EXISTS(
 SELECT 1 FROM game_bazaar_item_locations l LEFT JOIN game_bazaar_escrows e USING(escrow_id)
 WHERE e.escrow_id IS NULL OR e.item_instance_id<>l.item_instance_id OR e.state NOT IN('ESCROWED','INVALID')
) OR EXISTS(
 SELECT 1 FROM game_bazaar_item_transitions t JOIN game_bazaar_escrows e USING(escrow_id)
 WHERE t.kind IN('DELIVER','RETURN') AND (e.last_operation_id<>t.operation_id
   OR e.state<>CASE t.kind WHEN 'DELIVER' THEN 'DELIVERED' ELSE 'RETURNED' END)
);
$recovery$;
REVOKE ALL ON FUNCTION game_bazaar_escrows_invalid() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bazaar_escrows_invalid() TO oteryn_game_runtime;

CREATE FUNCTION game_bazaar_holds_invalid() RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $recovery$
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_gold_holds h
 WHERE NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT convert_from(d.receipt,'UTF8')::jsonb->'result_data' AS facts,
    CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.result_code='CHAR_CMD_OK'
   AND d.command IN('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold')
   AND q.facts->>'hold_id'=h.hold_id::text AND q.facts->>'hold_revision'=h.revision::text
   AND coalesce(q.facts->>'held_amount',q.facts->>'remaining_held')=h.amount::text
   AND (d.command<>'ReleaseBazaarGold' OR q.facts->>'hold_state'=
      CASE WHEN h.state='RELEASED' THEN 'RELEASED' ELSE 'ACTIVE' END)
   AND q.intent->>'listing_id'=h.listing_id::text AND q.intent->>'buyer_account_id'=h.buyer_account_id::text
   AND q.intent->>'seller_account_id'=h.seller_account_id::text AND d.world_id=h.world_id
 )
 OR (h.state='RELEASED' AND h.incoming_net<>0)
 OR (h.state='ACTIVE' AND (h.sale_id IS NOT NULL OR h.gross_amount IS NOT NULL
    OR h.delivery_operation_id IS NOT NULL OR h.delivery_receipt_digest IS NOT NULL))
 OR ((h.state='PAID' OR (h.state='RELEASED' AND h.delivery_receipt_digest IS NOT NULL)) AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d JOIN game_bazaar_gold_burns b
   ON b.issuer=d.issuer AND b.operation_id=d.operation_id
  CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent) q
  WHERE d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.command='PayoutBazaarGold'
   AND d.result_code='CHAR_CMD_OK' AND q.intent->>'hold_id'=h.hold_id::text
   AND q.intent->>'sale_id'=h.sale_id::text AND q.intent->>'delivery_operation_id'=h.delivery_operation_id::text
   AND q.intent->>'delivery_receipt_digest'=encode(h.delivery_receipt_digest,'hex')
 ))
 OR (h.state='DISPOSITION_PENDING' AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_locks l JOIN game_bazaar_decisions d
   ON d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.operation_id=l.last_operation_id
  WHERE l.transfer_operation_id=h.delivery_operation_id AND l.listing_id=h.listing_id
   AND l.transfer_sale_id=h.sale_id AND l.transfer_buyer=h.buyer_account_id
   AND l.seller_account_id=h.seller_account_id AND l.world_id=h.world_id
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'reservation_id'=h.hold_id::text
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'hold_revision'=h.revision::text
   AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'gross_amount'=h.gross_amount::text
   AND d.command='BindBazaarCharacterTransfer' AND d.result_code='CHAR_CMD_OK'
 ))
 OR ((h.state IN('AWAITING_PAYOUT','PAID') OR h.delivery_receipt_digest IS NOT NULL) AND NOT EXISTS(
  SELECT 1 FROM game_bazaar_decisions d
  CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
   convert_from(d.receipt,'UTF8')::jsonb AS receipt) q
  WHERE d.operation_id=h.delivery_operation_id AND d.receipt_digest=h.delivery_receipt_digest
   AND d.result_code='CHAR_CMD_OK' AND d.world_id=h.world_id
   AND ((d.issuer='OTERYN_PLATFORM_BAZAAR_AUTHORITY' AND d.command='DeliverBazaarItem'
     AND q.intent->>'listing_id'=h.listing_id::text AND q.intent->>'sale_id'=h.sale_id::text
     AND q.intent->>'seller_account_id'=h.seller_account_id::text
     AND q.intent->>'buyer_account_id'=h.buyer_account_id::text
     AND q.intent->'funding'->>'reservation_id'=h.hold_id::text
     AND q.intent->'funding'->>'gross_amount'=h.gross_amount::text
     AND EXISTS(SELECT 1 FROM game_bazaar_escrows e WHERE e.last_operation_id=d.operation_id
        AND e.state='DELIVERED' AND e.sale_id=h.sale_id AND e.listing_id=h.listing_id))
    OR (d.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND d.command='TransferCharacterOwnership'
     AND q.receipt->>'to_account_id'=h.buyer_account_id::text
     AND EXISTS(SELECT 1 FROM game_bazaar_locks l WHERE l.transfer_sale_id=h.sale_id
       AND l.transfer_buyer=h.buyer_account_id AND l.listing_id=h.listing_id
       AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'reservation_id'=h.hold_id::text)))
 ))
) OR EXISTS(
 SELECT 1 FROM (
  SELECT buyer_account_id AS account_id,world_id FROM game_bazaar_gold_holds WHERE amount>0
  UNION SELECT seller_account_id,world_id FROM game_bazaar_gold_holds WHERE incoming_net>0
 ) a LEFT JOIN game_account_bank_balances b USING(account_id,world_id)
 WHERE b.account_id IS NULL
 OR coalesce((SELECT sum(h.amount) FROM game_bazaar_gold_holds h
    WHERE h.buyer_account_id=a.account_id AND h.world_id=a.world_id),0)>b.balance
 OR 9000000000000000-game_account_bank_credit_headroom_before_bazaar(a.account_id,a.world_id)
    +coalesce((SELECT sum(h.incoming_net) FROM game_bazaar_gold_holds h
     WHERE h.seller_account_id=a.account_id AND h.world_id=a.world_id),0)>999999999999
);
$recovery$;
REVOKE ALL ON FUNCTION game_bazaar_holds_invalid() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bazaar_holds_invalid() TO oteryn_game_runtime;

CREATE FUNCTION game_bazaar_bank_invalid() RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $recovery$
SELECT EXISTS(
 SELECT 1 FROM game_bazaar_gold_burns burn
 LEFT JOIN game_bazaar_decisions d ON d.issuer=burn.issuer AND d.operation_id=burn.operation_id
 LEFT JOIN game_account_bank_entries debit ON debit.bazaar_issuer=burn.issuer
   AND debit.bazaar_operation_id=burn.operation_id AND debit.bazaar_kind='BAZAAR_DEBIT'
 LEFT JOIN game_account_bank_entries credit ON credit.bazaar_issuer=burn.issuer
   AND credit.bazaar_operation_id=burn.operation_id AND credit.bazaar_kind='BAZAAR_CREDIT'
 CROSS JOIN LATERAL (SELECT CASE WHEN d.result_code='CHAR_CMD_OK' THEN convert_from(d.intent,'UTF8')::jsonb ELSE '{}'::jsonb END AS intent,
   convert_from(d.receipt,'UTF8')::jsonb->'result_data' AS facts) q
 WHERE d.operation_id IS NULL OR d.command<>'PayoutBazaarGold' OR d.result_code<>'CHAR_CMD_OK'
 OR debit.entry_id IS NULL OR credit.entry_id IS NULL
 OR debit.account_id IS DISTINCT FROM (q.intent->>'buyer_account_id')::uuid
 OR credit.account_id IS DISTINCT FROM (q.intent->>'seller_account_id')::uuid
 OR debit.world_id IS DISTINCT FROM burn.world_id OR credit.world_id IS DISTINCT FROM burn.world_id
 OR d.world_id IS DISTINCT FROM burn.world_id
 OR debit.amount IS DISTINCT FROM (q.intent->>'gross_amount')::bigint
 OR credit.amount IS DISTINCT FROM (q.intent->>'seller_amount')::bigint
 OR burn.amount IS DISTINCT FROM (q.intent->>'fee_amount')::bigint
 OR debit.amount<>credit.amount+burn.amount OR burn.amount<>ceil(debit.amount::numeric*250/10000)
 OR q.facts->>'buyer_ledger_entry_id' IS DISTINCT FROM debit.entry_id::text
 OR q.facts->>'seller_ledger_entry_id' IS DISTINCT FROM credit.entry_id::text
 OR q.facts->>'burn_entry_id' IS DISTINCT FROM burn.burn_entry_id::text
 OR NOT EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.hold_id=(q.intent->>'hold_id')::uuid
    AND h.sale_id=(q.intent->>'sale_id')::uuid AND h.delivery_operation_id=(q.intent->>'delivery_operation_id')::uuid
    AND encode(h.delivery_receipt_digest,'hex')=q.intent->>'delivery_receipt_digest')
) OR EXISTS(
 SELECT 1 FROM game_account_bank_entries e WHERE e.bazaar_operation_id IS NOT NULL
 AND NOT EXISTS(SELECT 1 FROM game_bazaar_gold_burns b WHERE b.issuer=e.bazaar_issuer AND b.operation_id=e.bazaar_operation_id)
) OR EXISTS(
 SELECT 1 FROM game_account_bank_entries e
 WHERE EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.world_id=e.world_id
   AND (h.buyer_account_id=e.account_id OR h.seller_account_id=e.account_id))
 AND ((e.previous_entry_id IS NULL AND e.balance_before<>0)
  OR (e.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p
    WHERE p.entry_id=e.previous_entry_id AND p.account_id=e.account_id AND p.world_id=e.world_id AND p.balance_after=e.balance_before))
  OR NOT(EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=e.account_id
     AND b.world_id=e.world_id AND b.last_entry_id=e.entry_id AND b.balance=e.balance_after)
    OR EXISTS(SELECT 1 FROM game_account_bank_entries n WHERE n.previous_entry_id=e.entry_id
      AND n.account_id=e.account_id AND n.world_id=e.world_id AND n.balance_before=e.balance_after)))
) OR EXISTS(
 SELECT 1 FROM game_account_bank_balances b
 WHERE EXISTS(SELECT 1 FROM game_bazaar_gold_holds h WHERE h.world_id=b.world_id
   AND (h.buyer_account_id=b.account_id OR h.seller_account_id=b.account_id))
 AND ((b.last_entry_id IS NULL AND b.balance<>0)
  OR (b.last_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e
   WHERE e.entry_id=b.last_entry_id AND e.account_id=b.account_id AND e.world_id=b.world_id AND e.balance_after=b.balance)))
);
$recovery$;
REVOKE ALL ON FUNCTION game_bazaar_bank_invalid() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bazaar_bank_invalid() TO oteryn_game_runtime;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_bazaar_escrows_invalid','game_bazaar_holds_invalid','game_bazaar_bank_invalid'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $$;
