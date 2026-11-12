-- SRV9B / NPC-MARRIAGE-1. Matching executable Lynda semantics; no legacy globals.
-- Character owns consent/relations; Item owns all ring/box instances and custody;
-- Text owns the inscription; account Appearance owns the permanent base grant.
CREATE TABLE game_character_marriage_receipts
 (LIKE game_character_npc_stage_receipts INCLUDING DEFAULTS);
ALTER TABLE game_character_marriage_receipts
 ADD PRIMARY KEY(purchase_occurrence_id),
 ADD FOREIGN KEY(character_id) REFERENCES game_character_roots,
 ADD FOREIGN KEY(account_id) REFERENCES game_character_account_guards,
 ADD COLUMN partner_id UUID NOT NULL REFERENCES game_character_roots,
 ADD COLUMN operation SMALLINT NOT NULL CHECK(operation BETWEEN 1 AND 5),
 ADD COLUMN wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 ADD COLUMN actor_before JSONB NOT NULL,
 ADD COLUMN actor_after JSONB NOT NULL,
 ADD COLUMN partner_before JSONB NOT NULL,
 ADD COLUMN partner_after JSONB,
 ADD COLUMN consent_receipt UUID REFERENCES game_character_marriage_receipts,
 ADD COLUMN reply TEXT NOT NULL CHECK(octet_length(reply) BETWEEN 1 AND 512),
 ADD COLUMN world_id UUID NOT NULL,
 ADD CHECK(game_character_is_uuid_v7(purchase_occurrence_id)),
 ADD CHECK(character_id<>partner_id AND npc_key='oteryn:npc.lynda'),
 ADD CHECK(octet_length(request_binding)=32 AND octet_length(npc_identity)=16),
 ADD CHECK(npc_generation BETWEEN 1 AND 18446744073709551615),
 ADD CHECK(game_character_is_uuid_v7(command_session_id) AND command_id BETWEEN 1 AND 18446744073709551615),
 ADD CHECK(committed_at>0),
 ADD CHECK(committed_character_revision=original_character_revision+1
   AND original_character_revision>=1 AND committed_character_revision<=18446744073709551615),
 ADD CHECK(level_after=level_before AND experience_after=experience_before),
 ADD CHECK(octet_length(actor_before::text)<=2048 AND octet_length(actor_after::text)<=2048
   AND octet_length(partner_before::text)<=2048 AND octet_length(partner_after::text)<=2048),
 ADD UNIQUE(character_id,command_session_id,command_id),
 ADD UNIQUE(character_id,committed_character_revision);
CREATE INDEX game_marriage_receipt_partner ON game_character_marriage_receipts(partner_id);
CREATE INDEX game_marriage_receipt_account ON game_character_marriage_receipts(account_id);
CREATE INDEX game_marriage_receipt_consent ON game_character_marriage_receipts(consent_receipt);

CREATE TABLE game_character_marriages (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 partner_id UUID REFERENCES game_character_roots,
 status SMALLINT NOT NULL CHECK(status BETWEEN 0 AND 3),
 version BIGINT NOT NULL CHECK(version>0),
 last_receipt UUID NOT NULL REFERENCES game_character_marriage_receipts,
 CHECK((status=0)=(partner_id IS NULL)), CHECK(character_id<>partner_id)
);
CREATE UNIQUE INDEX game_marriage_exclusive_partner ON game_character_marriages(partner_id) WHERE status IN(2,3);
CREATE INDEX game_marriage_partner ON game_character_marriages(partner_id);
CREATE INDEX game_marriage_last_receipt ON game_character_marriages(last_receipt);

-- A bounded typed child of the existing Item craft/audit owner, not another inventory.
CREATE TABLE game_item_marriage_records (
 transaction_id UUID PRIMARY KEY REFERENCES game_character_marriage_receipts(purchase_occurrence_id),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs) BETWEEN 0 AND 2),
 outputs JSONB NOT NULL CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 0 AND 2),
 anchor_item_id UUID NOT NULL REFERENCES game_item_instances DEFERRABLE INITIALLY DEFERRED,
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 inscription_sha256 BYTEA CHECK(octet_length(inscription_sha256)=32),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((jsonb_array_length(inputs)=0)<>(jsonb_array_length(outputs)=0)),
 CHECK(octet_length(inputs::text)+octet_length(outputs::text)<=4096)
);
CREATE INDEX game_item_marriage_character ON game_item_marriage_records(character_id);
CREATE INDEX game_item_marriage_parent ON game_item_marriage_records(parent_item_instance_id);
CREATE INDEX game_item_marriage_anchor ON game_item_marriage_records(anchor_item_id);

CREATE FUNCTION game_marriage_after(p_before JSONB,p_character UUID,p_partner UUID,p_status INT,p_receipt UUID)
 RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$
 SELECT jsonb_build_object('character_id',p_character,'partner_id',p_partner,'status',p_status,
   'version',coalesce((p_before->>'version')::bigint,0)+1,'last_receipt',p_receipt)
$$;
CREATE FUNCTION game_marriage_receipt_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a JSONB; b JSONB; sa INT; sb INT; actor_next JSONB; partner_next JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT coalesce((SELECT to_jsonb(s) FROM game_character_marriages s WHERE character_id=NEW.character_id),'null'::jsonb) INTO a;
 SELECT coalesce((SELECT to_jsonb(s) FROM game_character_marriages s WHERE character_id=NEW.partner_id),'null'::jsonb) INTO b;
 sa:=coalesce((a->>'status')::int,0); sb:=coalesce((b->>'status')::int,0);
 IF EXISTS(SELECT 1 FROM game_character_npc_stage_receipts r WHERE r.character_id=NEW.character_id AND r.command_session_id=NEW.command_session_id AND r.command_id=NEW.command_id)
  OR EXISTS(SELECT 1 FROM game_character_npc_purchase_receipts r WHERE r.character_id=NEW.character_id AND r.command_session_id=NEW.command_session_id AND r.command_id=NEW.command_id) THEN
  RAISE EXCEPTION 'NPC command already belongs to another service receipt' USING ERRCODE='23514'; END IF;
 IF a IS DISTINCT FROM NEW.actor_before OR b IS DISTINCT FROM NEW.partner_before
  OR NOT EXISTS(SELECT 1 FROM game_character_roots r JOIN game_character_progression_state p USING(character_id)
    WHERE r.character_id=NEW.character_id AND r.account_id=NEW.account_id AND r.world_id=NEW.world_id
      AND r.character_revision=NEW.original_character_revision AND p.character_revision=r.character_revision
      AND p.level=NEW.level_before AND p.total_experience=NEW.experience_before)
  OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.partner_id AND r.world_id=NEW.world_id AND r.lifecycle=1)
 THEN RAISE EXCEPTION 'marriage requires current participants and exact relation snapshots' USING ERRCODE='23514'; END IF;
 IF NEW.operation=1 AND sa=0 AND sb=0 THEN
  actor_next:=game_marriage_after(a,NEW.character_id,NEW.partner_id,1,NEW.purchase_occurrence_id);
 ELSIF NEW.operation=2 AND sa=0 AND sb=1 AND (b->>'partner_id')::uuid=NEW.character_id
  AND NEW.consent_receipt=(b->>'last_receipt')::uuid AND EXISTS(
   SELECT 1 FROM game_character_marriage_receipts r WHERE r.purchase_occurrence_id=NEW.consent_receipt
    AND r.character_id=NEW.partner_id AND r.partner_id=NEW.character_id AND r.operation=1) THEN
  actor_next:=game_marriage_after(a,NEW.character_id,NEW.partner_id,3,NEW.purchase_occurrence_id);
  partner_next:=game_marriage_after(b,NEW.partner_id,NEW.character_id,3,NEW.purchase_occurrence_id);
 ELSIF NEW.operation=3 AND sa=1 AND (a->>'partner_id')::uuid=NEW.partner_id THEN
  actor_next:=game_marriage_after(a,NEW.character_id,NULL,0,NEW.purchase_occurrence_id);
 ELSIF NEW.operation IN(4,5) AND sa=(CASE NEW.operation WHEN 4 THEN 3 ELSE 2 END) AND sb=sa
  AND (a->>'partner_id')::uuid=NEW.partner_id AND (b->>'partner_id')::uuid=NEW.character_id THEN
  actor_next:=game_marriage_after(a,NEW.character_id,CASE WHEN NEW.operation=4 THEN NEW.partner_id END,CASE WHEN NEW.operation=4 THEN 2 ELSE 0 END,NEW.purchase_occurrence_id);
  partner_next:=game_marriage_after(b,NEW.partner_id,CASE WHEN NEW.operation=4 THEN NEW.character_id END,CASE WHEN NEW.operation=4 THEN 2 ELSE 0 END,NEW.purchase_occurrence_id);
 ELSE RAISE EXCEPTION 'invalid marriage consent transition' USING ERRCODE='23514'; END IF;
 IF NEW.actor_after IS DISTINCT FROM actor_next OR NEW.partner_after IS DISTINCT FROM partner_next
   OR ((NEW.operation=2)<>(NEW.consent_receipt IS NOT NULL)) THEN
  RAISE EXCEPTION 'marriage transition must have its exact participants and successor versions' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_marriage_receipt_admit BEFORE INSERT ON game_character_marriage_receipts
 FOR EACH ROW EXECUTE FUNCTION game_marriage_receipt_admit();

CREATE FUNCTION game_marriage_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.character_id<>OLD.character_id OR NEW.version<>OLD.version+1) THEN
  RAISE EXCEPTION 'marriage relation identity/version is immutable except one successor' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_marriage_receipts r WHERE r.purchase_occurrence_id=NEW.last_receipt
  AND r.created_xact_id=pg_current_xact_id() AND
   ((r.character_id=NEW.character_id AND r.actor_after=to_jsonb(NEW))
    OR (r.partner_id=NEW.character_id AND r.partner_after=to_jsonb(NEW)))) THEN
  RAISE EXCEPTION 'marriage relation requires its fresh stage receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_marriage_state_proven BEFORE INSERT OR UPDATE ON game_character_marriages
 FOR EACH ROW EXECUTE FUNCTION game_marriage_state_proven();

CREATE FUNCTION game_marriage_item_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_character_marriage_receipts%ROWTYPE; x JSONB; i game_item_instances%ROWTYPE;
 expected TEXT[]; actual TEXT[]; body TEXT;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT r FROM game_character_marriage_receipts WHERE purchase_occurrence_id=NEW.transaction_id;
 IF r.created_xact_id<>pg_current_xact_id() OR r.operation=5 OR r.character_id<>NEW.character_id
  OR r.world_id<>NEW.world_id OR r.committed_at<>NEW.occurred_at
  OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.character_id=NEW.character_id
    AND s.item_instance_id=NEW.parent_item_instance_id AND s.world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'marriage Item effects require their fresh Character stage and backpack' USING ERRCODE='23514'; END IF;
 expected:=CASE WHEN r.operation IN(1,2,3) THEN ARRAY['oteryn:item.tibia.i3004','oteryn:item.tibia.i9586'] ELSE ARRAY['oteryn:item.tibia.i9585'] END;
 SELECT array_agg(value->'before'->>'definition_production_key' ORDER BY value->'before'->>'definition_production_key') INTO actual FROM jsonb_array_elements(NEW.inputs);
 IF r.operation IN(1,2) THEN
  IF actual IS DISTINCT FROM expected OR jsonb_array_length(NEW.outputs)<>0 THEN
   RAISE EXCEPTION 'proposal/acceptance must burn exactly one ring and outfit box' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT array_agg(value->>'key' ORDER BY value->>'key') INTO actual FROM jsonb_array_elements(NEW.outputs);
  IF actual IS DISTINCT FROM expected OR jsonb_array_length(NEW.inputs)<>0 THEN
   RAISE EXCEPTION 'marriage refund/celebration has exact native outputs' USING ERRCODE='23514'; END IF;
 END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.inputs))<>jsonb_array_length(NEW.inputs)
  OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs)
  OR (SELECT count(DISTINCT value->>'ordinal') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs) THEN
  RAISE EXCEPTION 'duplicate marriage Item line' USING ERRCODE='23514'; END IF;
 FOR x IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
  SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=(x->>'item')::uuid FOR UPDATE;
  IF to_jsonb(i) IS DISTINCT FROM x->'before' OR i.world_id<>NEW.world_id OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item'
   OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
     AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=(x->>'ordinal')::numeric AND e.world_id=NEW.world_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
  THEN RAISE EXCEPTION 'marriage consumes unchanged plain carried inputs' USING ERRCODE='23514'; END IF;
 END LOOP;
 FOR x IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF NOT game_character_is_uuid_v7((x->>'item')::uuid) OR (x->>'count')::int<>1
   OR octet_length(x->>'revision') NOT BETWEEN 1 AND 512
   OR (x->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(x->>'item')::uuid) THEN
   RAISE EXCEPTION 'marriage output must be a fresh single native item' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.anchor_item_id IS DISTINCT FROM coalesce((NEW.inputs->0->>'item')::uuid,(NEW.outputs->0->>'item')::uuid) THEN
  RAISE EXCEPTION 'marriage audit anchor must be a real affected item' USING ERRCODE='23514'; END IF;
 IF r.operation=4 THEN
  SELECT a.name||' & '||b.name||' forever - married on '||to_char(to_timestamp(r.committed_at::double precision/1000) AT TIME ZONE 'UTC','FMMonth DD, YYYY.')
   INTO body FROM game_character_roots a,game_character_roots b WHERE a.character_id=r.character_id AND b.character_id=r.partner_id;
  IF NEW.inscription_sha256 IS DISTINCT FROM sha256(convert_to(body,'UTF8')) THEN
   RAISE EXCEPTION 'marriage inscription must bind committed names and UTC date' USING ERRCODE='23514'; END IF;
 ELSIF NEW.inscription_sha256 IS NOT NULL THEN
  RAISE EXCEPTION 'only the engraved ring carries a marriage inscription' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_marriage_item_admit BEFORE INSERT ON game_item_marriage_records
 FOR EACH ROW EXECUTE FUNCTION game_marriage_item_admit();

CREATE FUNCTION game_marriage_item_complete(p_transaction UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_marriage_records%ROWTYPE; x JSONB; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_item_marriage_records WHERE transaction_id=p_transaction;
 IF r.created_xact_id<>pg_current_xact_id()
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>jsonb_array_length(r.inputs)
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>jsonb_array_length(r.outputs)
  OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_transaction AND a.event_id=r.event_id
    AND a.item_instance_id=r.anchor_item_id AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
    AND a.created_xact_id=r.created_xact_id AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL) THEN
  RAISE EXCEPTION 'marriage Item ledger and audit must commit together' USING ERRCODE='23514'; END IF;
 FOR x IN SELECT value FROM jsonb_array_elements(r.inputs) LOOP
  SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=(x->>'item')::uuid;
  IF i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id IS DISTINCT FROM p_transaction
   OR i.state_revision<>(x->'before'->>'state_revision')::numeric+1
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
      ((x->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) THEN
   RAISE EXCEPTION 'marriage debit must retire exactly its proven input' USING ERRCODE='23514'; END IF;
 END LOOP;
 FOR x IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)
    WHERE n.item_instance_id=(x->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1 AND n.quantity=1
      AND n.definition_family='Item' AND n.definition_production_key=x->>'key' AND n.definition_revision_ref=x->>'revision'
      AND n.minted_transaction_id=p_transaction AND n.last_transaction_id IS NULL AND n.state_revision=1
      AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.parent_item_instance_id=r.parent_item_instance_id
      AND e.placement_ordinal=(x->>'ordinal')::numeric AND e.placed_transaction_id=p_transaction) THEN
   RAISE EXCEPTION 'marriage output must have exact native state and custody' USING ERRCODE='23514'; END IF;
  IF r.inscription_sha256 IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_texts t
    JOIN game_item_text_revisions v USING(item_instance_id) WHERE t.item_instance_id=(x->>'item')::uuid
    AND sha256(convert_to(t.text,'UTF8'))=r.inscription_sha256 AND t.writer_character_id IS NULL
    AND t.text_revision=1 AND v.last_revision=1 AND t.written_at=to_timestamp(r.occurred_at::double precision/1000)) THEN
   RAISE EXCEPTION 'engraved ring requires its atomic Item Text inscription' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_marriage_item_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_marriage_item_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_marriage_item_proven AFTER INSERT ON game_item_marriage_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_marriage_item_proven();

-- Extend every existing Item proof chain, preserving all previous causes verbatim.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven','game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure); at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing marriage Item proof anchor %',n; END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_marriage_records r,LATERAL jsonb_array_elements(r.inputs) x
    WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
      AND (x->>'item')::uuid=OLD.item_instance_id AND x->'before'=to_jsonb(OLD)) THEN
    IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR OLD.quantity<>1 OR OLD.lifecycle<>1
      OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
      OR NEW.state_revision<>OLD.state_revision THEN
     RAISE EXCEPTION 'marriage debit may only retire one proven item' USING ERRCODE='23514'; END IF;
    NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
   END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_marriage_records r,LATERAL jsonb_array_elements(r.inputs) x
    WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND (x->>'item')::uuid=NEW.item_instance_id) THEN
    PERFORM game_marriage_item_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_marriage_records r,LATERAL jsonb_array_elements(r.inputs) x
    JOIN game_item_instances i ON i.item_instance_id=(x->>'item')::uuid
    WHERE r.created_xact_id=pg_current_xact_id() AND (x->>'item')::uuid=OLD.item_instance_id
      AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id AND r.parent_item_instance_id=OLD.parent_item_instance_id
      AND (x->>'ordinal')::numeric=OLD.placement_ordinal AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN arm:=$arm$
   IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(SELECT 1 FROM game_item_marriage_records r,LATERAL jsonb_array_elements(r.outputs) x
     WHERE r.created_xact_id=pg_current_xact_id() AND r.transaction_id=NEW.placed_transaction_id AND (x->>'item')::uuid=NEW.item_instance_id
       AND r.world_id=NEW.world_id AND r.character_id=NEW.character_id AND r.parent_item_instance_id=NEW.parent_item_instance_id
       AND (x->>'ordinal')::numeric=NEW.placement_ordinal) THEN RETURN NULL; END IF;
   END IF;
$arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_marriage_records r,LATERAL jsonb_array_elements(r.outputs) x
    WHERE r.created_xact_id=pg_current_xact_id() AND r.transaction_id=NEW.minted_transaction_id AND (x->>'item')::uuid=NEW.item_instance_id) THEN
    PERFORM game_marriage_item_complete(NEW.minted_transaction_id); RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from at for 5);
 END LOOP;
END $migration$;

CREATE FUNCTION game_marriage_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_character_marriages s WHERE s.character_id=NEW.character_id AND to_jsonb(s)=NEW.actor_after)
  OR (NEW.partner_after IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_marriages s WHERE s.character_id=NEW.partner_id AND to_jsonb(s)=NEW.partner_after))
  OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.character_revision=NEW.committed_character_revision)
  OR ((NEW.operation<>5)<>EXISTS(SELECT 1 FROM game_item_marriage_records i WHERE i.transaction_id=NEW.purchase_occurrence_id AND i.created_xact_id=NEW.created_xact_id)) THEN
  RAISE EXCEPTION 'marriage stage, Character revision and Item effect must commit atomically' USING ERRCODE='23514'; END IF;
 IF NEW.operation=2 AND EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id IN(NEW.character_id,NEW.partner_id)
   AND NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id)
    WHERE u.account_id=c.account_id AND u.unlock_key='oteryn:appearance-grant/npc/lynda/newly-wed-base'
      AND r.source_kind='npc' AND r.content_revision='crystal-201189f-lynda-marriage-v1')) THEN
  RAISE EXCEPTION 'engagement requires both permanent native appearance grants' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_marriage_complete AFTER INSERT ON game_character_marriage_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_marriage_complete();

-- Reuse the standing Character revision-chain view without removing any owner arm.
DO $$ DECLARE body TEXT; BEGIN
 body:=regexp_replace(pg_get_viewdef('game_character_premium_revision_chain'::regclass,true),';[[:space:]]*$','');
 EXECUTE 'CREATE OR REPLACE VIEW game_character_premium_revision_chain WITH(security_invoker=true) AS '||body||
 ' UNION ALL SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_marriage_receipts';
END $$;
CREATE CONSTRAINT TRIGGER game_marriage_revision_chain AFTER INSERT ON game_character_marriage_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();

-- Command7 has one native receipt namespace across free stages, purchases and marriage.
DO $$ DECLARE body TEXT; at INT; BEGIN
 body:=pg_get_functiondef('game_s10_phoenix_guard()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing native NPC command receipt guard anchor'; END IF;
 body:=overlay(body placing E'BEGIN\n'||$arm$
  IF TG_TABLE_NAME IN('game_character_npc_stage_receipts','game_character_npc_purchase_receipts') THEN
   IF EXISTS(SELECT 1 FROM game_character_marriage_receipts r WHERE r.character_id=NEW.character_id
    AND r.command_session_id=NEW.command_session_id AND r.command_id=NEW.command_id) THEN
    RAISE EXCEPTION 'NPC command already belongs to marriage receipt' USING ERRCODE='23514'; END IF;
  END IF;
$arm$ from at for 5);
 EXECUTE body;
END $$;

-- The Text owner keeps marriage inscriptions immutable until its normal retired cleanup.
CREATE FUNCTION game_marriage_text_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP IN('UPDATE','DELETE') AND EXISTS(SELECT 1 FROM game_item_marriage_records r
    WHERE r.inscription_sha256 IS NOT NULL AND r.anchor_item_id=OLD.item_instance_id)
  AND NOT (TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=OLD.item_instance_id AND i.lifecycle=2)) THEN
  RAISE EXCEPTION 'marriage engraving is immutable Item Text' USING ERRCODE='23514'; END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER game_marriage_text_guard BEFORE UPDATE OR DELETE ON game_item_texts
 FOR EACH ROW EXECUTE FUNCTION game_marriage_text_guard();

DO $$ DECLARE t TEXT; n TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_character_marriage_receipts','game_item_marriage_records'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_character_immutable()',t||'_immutable',t);
 END LOOP;
 CREATE TRIGGER game_marriage_no_delete BEFORE DELETE ON game_character_marriages FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
 FOREACH t IN ARRAY ARRAY['game_character_marriage_receipts','game_character_marriages','game_item_marriage_records'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_character_reject_truncate()',t||'_no_truncate',t);
  EXECUTE format('REVOKE ALL ON %I FROM PUBLIC',t);
  EXECUTE format('GRANT SELECT ON %I TO oteryn_game_control',t);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_marriage_after(jsonb,uuid,uuid,integer,uuid)','game_marriage_receipt_admit()','game_marriage_state_proven()','game_marriage_item_admit()','game_marriage_item_complete(uuid)','game_marriage_item_proven()','game_marriage_complete()','game_marriage_text_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;
GRANT SELECT,INSERT ON game_character_marriage_receipts,game_item_marriage_records TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_marriages TO oteryn_game_runtime;
