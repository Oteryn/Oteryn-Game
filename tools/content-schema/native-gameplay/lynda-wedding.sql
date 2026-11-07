-- Closed Lynda outfit acquisition, Crystal201189f lynda.lua166-210/244-260.
CREATE TABLE game_lynda_wedding_receipts(
 transaction_id UUID PRIMARY KEY,game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions,
 command_id NUMERIC(20,0)NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots,
 partner_character_id UUID NOT NULL REFERENCES game_character_roots,world_id UUID NOT NULL,
 operation TEXT NOT NULL CHECK(operation IN('propose','accept','cancel')),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),cost_transaction_id UUID REFERENCES game_srv7_item_costs,
 original_revision NUMERIC(20,0) NOT NULL,committed_revision NUMERIC(20,0)NOT NULL CHECK(committed_revision=original_revision+1),
 level_before BIGINT NOT NULL,level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL,experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 profile_revision TEXT NOT NULL,ruleset_revision TEXT NOT NULL,content_revision TEXT NOT NULL,
 simulation_revision TEXT NOT NULL,evidence_revision TEXT NOT NULL,declaration_revision TEXT NOT NULL,
 policy_revision TEXT NOT NULL,reward_revision TEXT NOT NULL,UNIQUE(character_id,committed_revision),
 prior_proposal_id UUID REFERENCES game_lynda_wedding_receipts,actor_account UUID NOT NULL,partner_account UUID NOT NULL,
 source_sha256 BYTEA NOT NULL CHECK(source_sha256=decode('c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9','hex')),
 outcome JSONB NOT NULL CHECK(jsonb_typeof(outcome)='object'AND octet_length(outcome::text)<=4096),created_xact_id XID8 NOT NULL,
 UNIQUE(game_session_id,command_id),CHECK(character_id<>partner_character_id),CHECK((operation='cancel')=(cost_transaction_id IS NULL)));
CREATE INDEX game_lynda_wedding_actor ON game_lynda_wedding_receipts(character_id);
CREATE INDEX game_lynda_wedding_partner ON game_lynda_wedding_receipts(partner_character_id);
CREATE INDEX game_lynda_wedding_cost ON game_lynda_wedding_receipts(cost_transaction_id);
CREATE INDEX game_lynda_wedding_prior ON game_lynda_wedding_receipts(prior_proposal_id);
CREATE TABLE game_lynda_wedding_states(character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 status SMALLINT NOT NULL CHECK(status IN(0,1,2,3)),partner_character_id UUID REFERENCES game_character_roots,
 last_transaction_id UUID NOT NULL REFERENCES game_lynda_wedding_receipts,
 CHECK((status=0)=(partner_character_id IS NULL)),CHECK(character_id IS DISTINCT FROM partner_character_id));
CREATE INDEX game_lynda_wedding_state_partner ON game_lynda_wedding_states(partner_character_id);
CREATE INDEX game_lynda_wedding_state_receipt ON game_lynda_wedding_states(last_transaction_id);
CREATE TABLE game_lynda_wedding_context(backend_pid INTEGER PRIMARY KEY,transaction_xid XID8 NOT NULL,
 node_id UUID NOT NULL,registration NUMERIC NOT NULL,world_id UUID NOT NULL,channel_id UUID NOT NULL,
 scope_generation NUMERIC NOT NULL,character_id UUID NOT NULL,partner_id UUID NOT NULL,
 actor_account UUID NOT NULL,partner_account UUID NOT NULL,game_session UUID NOT NULL,command_id NUMERIC NOT NULL,
 connection NUMERIC NOT NULL,lease NUMERIC NOT NULL,original_revision NUMERIC NOT NULL,parent UUID NOT NULL,
 operation TEXT NOT NULL,request_binding BYTEA NOT NULL,prior_proposal UUID,cost_transaction UUID,progression_before JSONB NOT NULL);
REVOKE ALL ON game_lynda_wedding_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_lynda_wedding_current()RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT game_node_registration_history_valid()AND game_runtime_scope_assignment_history_valid()AND EXISTS(
 SELECT 1 FROM game_lynda_wedding_context c JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.game_session
 JOIN game_character_roots r ON r.character_id=c.character_id JOIN game_character_roots p ON p.character_id=c.partner_id
 JOIN game_durability_admission_character_guards cg ON cg.character_id=c.character_id
 JOIN game_durability_admission_account_guards ag ON ag.account_id=c.actor_account
 JOIN game_runtime_scope_assignments a ON a.scope_key='\x01'::bytea||uuid_send(c.world_id)||uuid_send(c.channel_id)
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key=a.scope_key
 JOIN game_node_registrations n ON n.node_id=c.node_id AND n.registration_revision=c.registration
 WHERE c.backend_pid=pg_backend_pid()AND c.transaction_xid=pg_current_xact_id()AND n.state=1
 AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings e WHERE e.node_id=c.node_id)
 AND a.state=1 AND a.scope_kind=1 AND a.ownership_generation=c.scope_generation AND a.holder_node_id=c.node_id AND a.holder_registration_revision=c.registration
 AND r.lifecycle=1 AND p.lifecycle=1 AND r.world_id=c.world_id AND p.world_id=c.world_id
 AND r.account_id=c.actor_account AND p.account_id=c.partner_account
 AND s.character_id=c.character_id AND s.account_id=c.actor_account AND s.world_id=c.world_id AND s.session_state=2 AND s.current_transport_ref IS NOT NULL
 AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=c.world_id AND s.runtime_scope_channel_id=c.channel_id
 AND s.current_generation=c.connection AND s.character_lease_generation=c.lease AND s.scope_ownership_generation=c.scope_generation
 AND cg.eligible AND cg.account_id=c.actor_account AND cg.world_id=c.world_id AND cg.lease_generation=c.lease AND cg.holder_game_session_id=c.game_session
 AND ag.presence_character_id=c.character_id AND ag.holder_game_session_id=c.game_session AND rg.ready AND rg.ownership_generation=c.scope_generation)
$$;
CREATE FUNCTION game_lynda_wedding_arm(p_node UUID,p_registration NUMERIC,p_secret BYTEA,p JSONB)RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE actor game_character_roots%ROWTYPE;partner game_character_roots%ROWTYPE;a game_lynda_wedding_states%ROWTYPE;b game_lynda_wedding_states%ROWTYPE;
 proposed UUID;operation TEXT;progress_before JSONB;BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret)IS DISTINCT FROM TRUE
 OR jsonb_typeof(p)IS DISTINCT FROM'object'OR(p->>'actor')::uuid=(p->>'partner')::uuid THEN
 RAISE EXCEPTION 'Lynda requires current Node and two real Characters' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_character_roots WHERE character_id IN((p->>'actor')::uuid,(p->>'partner')::uuid)ORDER BY character_id FOR UPDATE;
 SELECT*INTO actor FROM game_character_roots WHERE character_id=(p->>'actor')::uuid;
 SELECT*INTO partner FROM game_character_roots WHERE character_id=(p->>'partner')::uuid;
 SELECT to_jsonb(progression) INTO STRICT progress_before FROM game_character_progression_state progression WHERE progression.character_id=actor.character_id FOR UPDATE;
 SELECT*INTO a FROM game_lynda_wedding_states WHERE character_id=actor.character_id FOR UPDATE;
 SELECT*INTO b FROM game_lynda_wedding_states WHERE character_id=partner.character_id FOR UPDATE;
 IF actor.character_id IS NULL OR partner.character_id IS NULL OR actor.character_revision IS DISTINCT FROM(p->>'original_revision')::numeric THEN
 RAISE EXCEPTION 'Lynda pair/revision changed' USING ERRCODE='23514';END IF;
 IF p->>'operation'='cancel' THEN
  IF a.status IS DISTINCT FROM 1 OR a.partner_character_id IS DISTINCT FROM partner.character_id THEN RAISE EXCEPTION 'no actual pending proposal to cancel' USING ERRCODE='23514';END IF;
  operation:='cancel';proposed:=a.last_transaction_id;
 ELSE
  IF coalesce(a.status,0)IN(1,2)OR coalesce(b.status,0)IN(2,3)THEN RAISE EXCEPTION 'Lynda participant is already committed' USING ERRCODE='23514';END IF;
  IF b.status=1 THEN
   IF b.partner_character_id IS DISTINCT FROM actor.character_id THEN RAISE EXCEPTION 'candidate proposed elsewhere' USING ERRCODE='23514';END IF;
   operation:='accept';proposed:=b.last_transaction_id;
  ELSE operation:='propose';END IF;
 END IF;
 IF operation IS DISTINCT FROM p->>'operation' OR octet_length(decode(p->>'binding','hex'))IS DISTINCT FROM 32
 OR EXISTS(SELECT 1 FROM game_lynda_wedding_context WHERE backend_pid=pg_backend_pid()AND transaction_xid=pg_current_xact_id())THEN
 RAISE EXCEPTION 'Lynda exact pair observation can only be sealed once' USING ERRCODE='23514';END IF;
 DELETE FROM game_lynda_wedding_context c WHERE c.backend_pid=pg_backend_pid()OR NOT EXISTS(SELECT 1 FROM pg_stat_activity s WHERE s.pid=c.backend_pid);
 INSERT INTO game_lynda_wedding_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_node,p_registration,(p->>'world')::uuid,(p->>'channel')::uuid,
 (p->>'scope_generation')::numeric,actor.character_id,partner.character_id,actor.account_id,partner.account_id,(p->>'game_session')::uuid,(p->>'command')::numeric,
 (p->>'connection')::numeric,(p->>'lease')::numeric,actor.character_revision,(p->>'parent')::uuid,operation,decode(p->>'binding','hex'),proposed,NULL,progress_before);
 IF game_lynda_wedding_current()IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'Lynda stale current Character/session/scope' USING ERRCODE='23514';END IF;
END $$;
CREATE FUNCTION game_lynda_wedding_cost_admit()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_lynda_wedding_context%ROWTYPE;BEGIN
 IF NEW.cause_kind<>'lynda_wedding'THEN RETURN NEW;END IF;
 SELECT*INTO c FROM game_lynda_wedding_context WHERE backend_pid=pg_backend_pid()AND transaction_xid=pg_current_xact_id();
 IF game_lynda_wedding_current()IS DISTINCT FROM TRUE OR c.operation NOT IN('propose','accept')OR c.cost_transaction IS NOT NULL
 OR(NEW.character_id,NEW.world_id,NEW.channel_id,NEW.game_session_id,NEW.command_id,NEW.source_key)IS DISTINCT FROM(c.character_id,c.world_id,c.channel_id,c.game_session,c.command_id,'oteryn:lynda.wedding.materials'::text)
 OR NEW.transaction_id=c.parent OR NEW.requirements IS DISTINCT FROM '[{"family":"Item","key":"oteryn:item.tibia.i3004","revision":"definition-r1","count":1},{"family":"Item","key":"oteryn:item.tibia.i9586","revision":"definition-r1","count":1}]'::jsonb THEN
 RAISE EXCEPTION 'Lynda consumes exactly own wedding ring and outfit box' USING ERRCODE='23514';END IF;
 UPDATE game_lynda_wedding_context SET cost_transaction=NEW.transaction_id WHERE backend_pid=pg_backend_pid();RETURN NEW;
END $$;
CREATE TRIGGER game_lynda_wedding_cost_admit BEFORE INSERT ON game_srv7_item_costs FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_cost_admit();
CREATE FUNCTION game_lynda_wedding_receipt_admit()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_lynda_wedding_context%ROWTYPE;BEGIN
 SELECT*INTO c FROM game_lynda_wedding_context WHERE backend_pid=pg_backend_pid()AND transaction_xid=pg_current_xact_id();
 IF game_lynda_wedding_current()IS DISTINCT FROM TRUE OR(NEW.transaction_id,NEW.game_session_id,NEW.command_id,NEW.character_id,NEW.partner_character_id,NEW.world_id,NEW.operation,NEW.request_binding,NEW.cost_transaction_id,NEW.original_revision,NEW.committed_revision)
 IS DISTINCT FROM(c.parent,c.game_session,c.command_id,c.character_id,c.partner_id,c.world_id,c.operation,c.request_binding,c.cost_transaction,c.original_revision,c.original_revision+1)THEN
 RAISE EXCEPTION 'Lynda receipt lacks exact sealed reciprocal source' USING ERRCODE='23514';END IF;
 IF (c.progression_before->>'character_revision')::numeric IS DISTINCT FROM c.original_revision
 OR NEW.level_before IS DISTINCT FROM (c.progression_before->>'level')::bigint
 OR NEW.experience_before IS DISTINCT FROM (c.progression_before->>'total_experience')::bigint
 OR NEW.profile_revision IS DISTINCT FROM c.progression_before->>'profile_revision'
 OR NEW.ruleset_revision IS DISTINCT FROM c.progression_before->>'ruleset_revision'
 OR NEW.content_revision IS DISTINCT FROM c.progression_before->>'content_revision'
 OR NEW.simulation_revision IS DISTINCT FROM c.progression_before->>'simulation_revision'
 OR NEW.evidence_revision IS DISTINCT FROM c.progression_before->>'evidence_revision'
 OR NEW.declaration_revision IS DISTINCT FROM c.progression_before->>'declaration_revision'
 OR NEW.policy_revision IS DISTINCT FROM c.progression_before->>'policy_revision'
 OR NEW.reward_revision IS DISTINCT FROM c.progression_before->>'reward_revision' THEN RAISE EXCEPTION 'Wedding receipt changed its locked native progression stamps' USING ERRCODE='23514';END IF;
 NEW.actor_account:=c.actor_account;NEW.partner_account:=c.partner_account;NEW.prior_proposal_id:=c.prior_proposal;
 NEW.source_sha256:=decode('c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9','hex');NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_lynda_wedding_receipt_admit BEFORE INSERT ON game_lynda_wedding_receipts FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_receipt_admit();
CREATE TRIGGER game_lynda_wedding_receipt_immutable BEFORE UPDATE OR DELETE ON game_lynda_wedding_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_lynda_wedding_receipt_no_truncate BEFORE TRUNCATE ON game_lynda_wedding_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_lynda_wedding_state_admit()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_lynda_wedding_context%ROWTYPE;BEGIN
 SELECT*INTO c FROM game_lynda_wedding_context WHERE backend_pid=pg_backend_pid()AND transaction_xid=pg_current_xact_id();
 IF TG_OP='DELETE'OR game_lynda_wedding_current()IS DISTINCT FROM TRUE OR NEW.last_transaction_id IS DISTINCT FROM c.parent
 OR NOT EXISTS(SELECT 1 FROM game_lynda_wedding_receipts r WHERE r.transaction_id=c.parent AND r.created_xact_id=pg_current_xact_id())THEN
 RAISE EXCEPTION 'Lynda state requires actual current source receipt' USING ERRCODE='23514';END IF;
 IF NEW.character_id=c.character_id AND((c.operation='propose'AND NEW.status=1 AND NEW.partner_character_id=c.partner_id)
 OR(c.operation='accept'AND NEW.status=3 AND NEW.partner_character_id=c.partner_id)OR(c.operation='cancel'AND NEW.status=0 AND NEW.partner_character_id IS NULL))THEN RETURN NEW;END IF;
 IF c.operation='accept'AND NEW.character_id=c.partner_id AND NEW.status=3 AND NEW.partner_character_id=c.character_id AND OLD.status=1 AND OLD.partner_character_id=c.character_id THEN RETURN NEW;END IF;
 RAISE EXCEPTION 'Lynda source cannot change another pair/phase' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_lynda_wedding_state_admit BEFORE INSERT OR UPDATE OR DELETE ON game_lynda_wedding_states FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_state_admit();
CREATE TRIGGER game_lynda_wedding_state_no_truncate BEFORE TRUNCATE ON game_lynda_wedding_states FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_lynda_wedding_complete()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_lynda_wedding_context%ROWTYPE;BEGIN
 SELECT*INTO c FROM game_lynda_wedding_context WHERE backend_pid=pg_backend_pid()AND transaction_xid=pg_current_xact_id();
 IF game_lynda_wedding_current()IS DISTINCT FROM TRUE OR NEW.created_xact_id<>pg_current_xact_id()
 OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.character_revision=NEW.committed_revision AND r.xmin=pg_current_xact_id()::xid)
 OR NOT EXISTS(SELECT 1 FROM game_lynda_wedding_states s WHERE s.character_id=NEW.character_id AND s.last_transaction_id=NEW.transaction_id AND s.status=CASE NEW.operation WHEN'propose'THEN 1 WHEN'accept'THEN 3 ELSE 0 END)
 OR(NEW.operation<>'cancel'AND NOT EXISTS(SELECT 1 FROM game_srv7_item_costs k WHERE k.transaction_id=NEW.cost_transaction_id AND k.cause_kind='lynda_wedding'AND k.created_xact_id=pg_current_xact_id()AND k.character_id=NEW.character_id AND k.game_session_id=NEW.game_session_id AND k.command_id=NEW.command_id))THEN
 RAISE EXCEPTION 'Lynda source lacks one atomic cost/state/root result' USING ERRCODE='23514';END IF;
 IF NEW.operation IN('accept','cancel')AND NOT EXISTS(SELECT 1 FROM game_lynda_wedding_receipts p WHERE p.transaction_id=NEW.prior_proposal_id AND p.operation='propose'AND
 ((NEW.operation='accept'AND p.character_id=NEW.partner_character_id AND p.partner_character_id=NEW.character_id)OR(NEW.operation='cancel'AND p.character_id=NEW.character_id AND p.partner_character_id=NEW.partner_character_id)))THEN
 RAISE EXCEPTION 'Lynda requires actual earlier reciprocal/pending proposal' USING ERRCODE='23514';END IF;
 IF NEW.operation='accept'THEN
  IF NOT EXISTS(SELECT 1 FROM game_lynda_wedding_states s WHERE s.character_id=NEW.partner_character_id AND s.partner_character_id=NEW.character_id AND s.status=3 AND s.last_transaction_id=NEW.transaction_id)
  OR EXISTS(SELECT 1 FROM unnest(ARRAY[NEW.actor_account,NEW.partner_account])AS recipient(account_id) WHERE NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u WHERE u.account_id=recipient.account_id AND u.unlock_key='oteryn:appearance-grant/npc/lynda/newly_wed/base'))THEN
  RAISE EXCEPTION 'Lynda acceptance must retain both actual Account outfits' USING ERRCODE='23514';END IF;
 END IF;
 IF NEW.operation='cancel'THEN
  IF NOT EXISTS(SELECT 1 FROM game_reward_claim_inbox_mint_receipts m WHERE m.transaction_id=NEW.transaction_id AND m.character_id=NEW.character_id AND m.game_session_id=NEW.game_session_id AND m.command_id=NEW.command_id AND m.item_count=2 AND m.source_binding=NEW.request_binding AND m.created_xact_id=pg_current_xact_id())
  OR(SELECT count(*)FROM game_reward_claim_inbox_mint_lines l JOIN game_item_instances i USING(item_instance_id)WHERE l.transaction_id=NEW.transaction_id AND l.created_xact_id=pg_current_xact_id()AND i.minted_transaction_id=NEW.transaction_id AND i.quantity=1 AND i.lifecycle=1 AND i.definition_family='Item'AND i.definition_production_key IN('oteryn:item.tibia.i3004','oteryn:item.tibia.i9586'))<>2
  OR(SELECT count(DISTINCT i.definition_production_key)FROM game_reward_claim_inbox_mint_lines l JOIN game_item_instances i USING(item_instance_id)WHERE l.transaction_id=NEW.transaction_id)<>2 THEN
  RAISE EXCEPTION 'Lynda pending cancellation requires exact one ring and box refund' USING ERRCODE='23514';END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_lynda_wedding_complete AFTER INSERT ON game_lynda_wedding_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_complete();
-- Exact reciprocal child of the general NPC entitlement namespace. A source_kind
-- label or a canonical key alone cannot confer any authority.
CREATE FUNCTION game_lynda_wedding_grant_current(proof JSONB)RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT game_lynda_wedding_current() AND
 proof->>'unlock_key'='oteryn:appearance-grant/npc/lynda/newly_wed/base'
 AND proof->>'source_kind'='npc_wedding'
 AND proof->>'content_revision'='crystal-201189f-npc-cosmetics-v1'
 AND EXISTS(SELECT 1 FROM game_lynda_wedding_receipts r
  WHERE r.operation='accept' AND r.created_xact_id=pg_current_xact_id()
  AND (proof->>'source_event_id')::bytea=uuid_send(r.transaction_id)||uuid_send((proof->>'granting_character_id')::uuid)
  AND (((proof->>'granting_character_id')::uuid=r.character_id AND (proof->>'account_id')::uuid=r.actor_account)
   OR ((proof->>'granting_character_id')::uuid=r.partner_character_id AND (proof->>'account_id')::uuid=r.partner_account)))
$$;
DO $namespace_child$
DECLARE body TEXT;needle TEXT;
BEGIN
 body:=pg_get_functiondef('game_npc_grant_namespace_proven()'::regprocedure);
 needle:=' IF npc_namespace_proof IS NULL OR npc_namespace_proof->>''source_kind'' IS DISTINCT FROM ''npc''';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Wedding NPC namespace proof boundary changed';END IF;
 EXECUTE replace(body,needle,$child$
 IF NEW.unlock_key='oteryn:appearance-grant/npc/lynda/newly_wed/base' THEN
  IF game_lynda_wedding_grant_current(npc_namespace_proof) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'Wedding grant requires its actual current reciprocal source child' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
 END IF;
 IF npc_namespace_proof IS NULL OR npc_namespace_proof->>'source_kind' IS DISTINCT FROM 'npc'$child$);
END $namespace_child$;
CREATE FUNCTION game_lynda_wedding_grant_guard()RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE entry game_account_appearance_unlock_receipts%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_account_appearance_unlocks'THEN
  SELECT*INTO entry FROM game_account_appearance_unlock_receipts WHERE grant_occurrence_id=NEW.grant_occurrence_id;
 ELSE entry:=NEW;END IF;
 IF entry.unlock_key='oteryn:appearance-grant/npc/lynda/newly_wed/base'OR entry.source_kind='npc_wedding'THEN
  IF entry.source_kind IS DISTINCT FROM'npc_wedding'OR entry.unlock_key IS DISTINCT FROM'oteryn:appearance-grant/npc/lynda/newly_wed/base'
  OR entry.content_revision IS DISTINCT FROM'crystal-201189f-npc-cosmetics-v1'
  OR NOT EXISTS(SELECT 1 FROM game_lynda_wedding_receipts r WHERE r.operation='accept'AND r.created_xact_id=pg_current_xact_id()
   AND entry.source_event_id=uuid_send(r.transaction_id)||uuid_send(entry.granting_character_id)
   AND((entry.granting_character_id=r.character_id AND entry.account_id=r.actor_account)OR(entry.granting_character_id=r.partner_character_id AND entry.account_id=r.partner_account)))THEN
   RAISE EXCEPTION 'Lynda outfit grant lacks actual reciprocal pair receipt' USING ERRCODE='23514';
  END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_lynda_wedding_grant_guard AFTER INSERT ON game_account_appearance_unlock_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_grant_guard();
CREATE CONSTRAINT TRIGGER game_lynda_wedding_fact_guard AFTER INSERT ON game_account_appearance_unlocks DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_lynda_wedding_grant_guard();
-- Add only this source cost kind. All existing physical conservation/audit guards run.
DO $cause_kind$
DECLARE existing_check TEXT;
BEGIN
 SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT existing_check
 FROM pg_constraint c WHERE c.conrelid='game_srv7_item_costs'::regclass
 AND c.conname='game_srv7_item_costs_cause_kind_check';
 ALTER TABLE game_srv7_item_costs DROP CONSTRAINT game_srv7_item_costs_cause_kind_check;
 EXECUTE format('ALTER TABLE game_srv7_item_costs ADD CONSTRAINT game_srv7_item_costs_cause_kind_check CHECK ((%s) OR cause_kind = %L)',existing_check,'lynda_wedding');
END $cause_kind$;
DO $cost$DECLARE body TEXT;needle TEXT;BEGIN
 body:=pg_get_functiondef('game_srv7_cost_complete()'::regprocedure);needle:=' IF NEW.cause_kind=''npc_cosmetic'' THEN';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Lynda cost completion boundary changed';END IF;
 EXECUTE replace(body,needle,$new$
 IF NEW.cause_kind='lynda_wedding'THEN
  PERFORM 1 FROM game_lynda_wedding_receipts r WHERE r.cost_transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()AND r.character_id=NEW.character_id AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id;
 ELSIF NEW.cause_kind='npc_cosmetic' THEN$new$);
END $cost$;
REVOKE ALL ON game_lynda_wedding_receipts,game_lynda_wedding_states FROM PUBLIC;
GRANT SELECT,INSERT ON game_lynda_wedding_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_lynda_wedding_states TO oteryn_game_runtime;

DO $privileges$DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY['game_lynda_wedding_current()','game_lynda_wedding_arm(uuid,numeric,bytea,jsonb)','game_lynda_wedding_grant_current(jsonb)']LOOP
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
END $privileges$;

DO $wedding_chain$ DECLARE definition TEXT;BEGIN
 definition:=rtrim(pg_get_viewdef('game_character_hazard_revision_chain'::regclass,true),E';\n ');
 IF position('game_lynda_wedding_receipts' IN definition)>0 THEN RAISE EXCEPTION 'Wedding edge already installed';END IF;
 EXECUTE 'CREATE OR REPLACE VIEW game_character_hazard_revision_chain WITH (security_invoker=true) AS '
 ||definition||' UNION ALL SELECT character_id,original_revision,committed_revision,level_before,level_after,'
 ||'experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,'
 ||'evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_lynda_wedding_receipts';
END $wedding_chain$;
CREATE CONSTRAINT TRIGGER game_lynda_wedding_character_chain AFTER INSERT ON game_lynda_wedding_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();

-- Read-only permanent fact ancestry. Unlike current write admission this is
-- valid across reconnect and Character lifecycle changes; immutable reciprocal
-- consent and its exact Account/Character/source-event binding remain required.
CREATE FUNCTION game_lynda_wedding_grant_historical(p_account UUID,p_key TEXT,p_revision TEXT,p_character UUID,p_event BYTEA)
 RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT p_key='oteryn:appearance-grant/npc/lynda/newly_wed/base'
 AND p_revision='crystal-201189f-npc-cosmetics-v1'
 AND EXISTS(SELECT 1 FROM game_lynda_wedding_receipts accepted
 JOIN game_lynda_wedding_receipts proposal ON proposal.transaction_id=accepted.prior_proposal_id
 WHERE accepted.operation='accept' AND proposal.operation='propose'
 AND proposal.character_id=accepted.partner_character_id AND proposal.partner_character_id=accepted.character_id
 AND proposal.world_id=accepted.world_id AND proposal.actor_account=accepted.partner_account AND proposal.partner_account=accepted.actor_account
 AND p_event=uuid_send(accepted.transaction_id)||uuid_send(p_character)
 AND ((p_character=accepted.character_id AND p_account=accepted.actor_account)
   OR(p_character=accepted.partner_character_id AND p_account=accepted.partner_account)))
$$;
DO $history_path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_lynda_wedding_grant_historical(uuid,text,text,uuid,bytea) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $history_path$;
REVOKE ALL ON FUNCTION game_lynda_wedding_grant_historical(uuid,text,text,uuid,bytea) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_lynda_wedding_grant_historical(uuid,text,text,uuid,bytea) TO oteryn_game_runtime;
