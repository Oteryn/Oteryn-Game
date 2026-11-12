-- GPROD-DELETION. Pending jobs, reversible private custody and terminal proof
-- are authoritative control state, separate from finite P90D audit retention.
CREATE TABLE game_account_closure_jobs (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 command TEXT NOT NULL CHECK(command IN ('CloseAccount','ResolveAccountClosure')),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding)=33),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 4096),
 account_id UUID NOT NULL CHECK(game_character_is_uuid_v7(account_id)),
 termination_id UUID NOT NULL CHECK(game_character_is_uuid_v7(termination_id)),
 lifecycle_generation NUMERIC(20,0) NOT NULL CHECK(lifecycle_generation BETWEEN 1 AND 18446744073709551615),
 source_revision NUMERIC(20,0) NOT NULL CHECK(source_revision BETWEEN 1 AND 18446744073709551615),
 recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
 receipt BYTEA CHECK(octet_length(receipt) BETWEEN 1 AND 2048), result_code TEXT,
 receipt_acknowledged BOOLEAN NOT NULL DEFAULT false, receipt_quarantined BOOLEAN NOT NULL DEFAULT false,
 scheduled_for_source BIGINT NOT NULL CHECK(scheduled_for_source>0),
 created_at BIGINT NOT NULL, decided_at BIGINT,
 CHECK((receipt IS NULL)=(result_code IS NULL)),
 CHECK((receipt IS NULL)=(decided_at IS NULL)), CHECK(NOT receipt_acknowledged OR receipt IS NOT NULL)
);
CREATE INDEX game_account_closure_jobs_account ON game_account_closure_jobs(account_id,termination_id);
CREATE INDEX game_account_closure_jobs_pending ON game_account_closure_jobs(created_at,operation_id) WHERE receipt IS NULL;
CREATE TABLE game_account_closures (
 account_id UUID PRIMARY KEY REFERENCES game_character_account_guards,
 termination_id UUID NOT NULL CHECK(game_character_is_uuid_v7(termination_id)),
 close_operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(close_operation_id)),
 lifecycle_generation NUMERIC(20,0) NOT NULL CHECK(lifecycle_generation BETWEEN 1 AND 18446744073709551615),
 close_generation NUMERIC(20,0) NOT NULL CHECK(close_generation BETWEEN 1 AND 18446744073709551615),
 resolution_operation_id UUID,
 decision TEXT CHECK(decision IN ('CANCEL','FINALIZE')),
 state TEXT NOT NULL CHECK(state IN ('CLOSING','CLOSED_DETACHED','CANCELLING','CANCELLED','FINALIZING','FINALIZED')),
 manifest BYTEA NOT NULL DEFAULT convert_to('[]','UTF8') CHECK(octet_length(manifest) BETWEEN 2 AND 32768),
 manifest_count SMALLINT NOT NULL DEFAULT 0 CHECK(manifest_count BETWEEN 0 AND 64),
 recovery_generation NUMERIC(20,0) NOT NULL,
 closed_at BIGINT NOT NULL,
 name_release_after_seconds NUMERIC(20,0),
 CHECK(lifecycle_generation>=close_generation),
 CHECK((decision IS NULL)=(state IN ('CLOSING','CLOSED_DETACHED')))
);
CREATE TABLE game_account_closure_members (
 close_operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(close_operation_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 account_id UUID NOT NULL REFERENCES game_account_closures,
 verified_recovery_generation NUMERIC(20,0), root_revision NUMERIC(20,0) NOT NULL, custody_sha256 BYTEA CHECK(custody_sha256 IS NULL OR octet_length(custody_sha256)=32),
 -- 0 waiting for owner/offline proof, 1 detached, 2 restored, 3 retired.
 step SMALLINT NOT NULL DEFAULT 0 CHECK(step BETWEEN 0 AND 3),
 PRIMARY KEY(close_operation_id,character_id)
);
CREATE INDEX game_account_closure_members_character ON game_account_closure_members(character_id,step);
CREATE INDEX game_account_closure_members_account ON game_account_closure_members(account_id,character_id);
CREATE TABLE game_account_closure_offline (
 game_session_id UUID PRIMARY KEY REFERENCES game_durability_reconnect_sessions,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 node_id UUID NOT NULL, lease_generation NUMERIC(20,0) NOT NULL,
 acknowledged_at BIGINT NOT NULL
);
CREATE INDEX game_account_closure_offline_character ON game_account_closure_offline(character_id);
-- This is an input seam for independently authenticated lifecycle reconciliation,
-- not data the runtime may seed from an immutable command. No observation is seeded.
CREATE TABLE game_account_closure_lifecycle (
 account_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(account_id)),
 termination_id UUID NOT NULL CHECK(game_character_is_uuid_v7(termination_id)),
 lifecycle_generation NUMERIC(20,0) NOT NULL CHECK(lifecycle_generation BETWEEN 1 AND 18446744073709551615),
 source_revision NUMERIC(20,0) NOT NULL CHECK(source_revision BETWEEN 1 AND 18446744073709551615),
 phase TEXT NOT NULL CHECK(phase IN ('CLOSE','CANCEL','FINALIZE')), financial_obligations_clear BOOLEAN NOT NULL DEFAULT false,
 recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
 usable BOOLEAN NOT NULL DEFAULT true, observed_at BIGINT NOT NULL CHECK(observed_at>0), valid_until BIGINT NOT NULL,
 CHECK(valid_until>observed_at AND valid_until<=observed_at+5)
);
CREATE FUNCTION game_account_closure_lifecycle_monotone() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND (to_jsonb(NEW)-'usable')=(to_jsonb(OLD)-'usable') THEN RETURN NEW; END IF;
 IF TG_OP<>'UPDATE' OR (NEW.account_id=OLD.account_id AND NEW.source_revision>OLD.source_revision
    AND NEW.lifecycle_generation>=OLD.lifecycle_generation AND NEW.observed_at>=OLD.observed_at) THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'nonmonotone account lifecycle evidence' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_account_closure_lifecycle_monotone BEFORE UPDATE ON game_account_closure_lifecycle
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_lifecycle_monotone();
CREATE TABLE game_account_closure_publications (
 account_id UUID PRIMARY KEY REFERENCES game_account_closures,
 projection_epoch BIGINT NOT NULL CHECK(projection_epoch>0), projection_revision BIGINT NOT NULL CHECK(projection_revision>0),
 receipt_operation_id UUID NOT NULL REFERENCES game_account_closure_jobs,
 body BYTEA NOT NULL CHECK(octet_length(body) BETWEEN 1 AND 2048), acknowledged BOOLEAN NOT NULL DEFAULT false, quarantined BOOLEAN NOT NULL DEFAULT false,
 created_at BIGINT NOT NULL
);
CREATE INDEX game_account_closure_publications_receipt ON game_account_closure_publications(receipt_operation_id);
CREATE TABLE game_account_closure_publication_clock (
 qualified_recovery_generation NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(qualified_recovery_generation BETWEEN 1 AND 18446744073709551615),
 singleton BOOLEAN PRIMARY KEY DEFAULT true CHECK(singleton), projection_epoch BIGINT NOT NULL CHECK(projection_epoch>0),
 quarantined BOOLEAN NOT NULL DEFAULT false, resync_after UUID,resync_pending BOOLEAN NOT NULL DEFAULT false, watermark_at BIGINT NOT NULL DEFAULT 0
);
INSERT INTO game_account_closure_publication_clock(singleton,projection_epoch) VALUES(true,1);
CREATE TABLE game_account_closure_audit_outbox (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)), transaction_id UUID NOT NULL UNIQUE,
 operation_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(operation_id)),
 event_type_id BIGINT NOT NULL DEFAULT 7 CHECK(event_type_id=7), schema_revision BIGINT NOT NULL DEFAULT 1 CHECK(schema_revision=1),
 retention_profile_id TEXT NOT NULL DEFAULT 'PLAT3_AUTHORITY_AUDIT_RETENTION_V1' CHECK(retention_profile_id='PLAT3_AUTHORITY_AUDIT_RETENTION_V1'),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 8192),
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096), payload_sha256 BYTEA NOT NULL CHECK(octet_length(payload_sha256)=32),
 occurred_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, legal_hold BOOLEAN NOT NULL DEFAULT false,
 publication_state SMALLINT NOT NULL DEFAULT 1 CHECK(publication_state IN (1,2)),
 CHECK(expires_at=occurred_at+7776000000)
);
CREATE FUNCTION game_account_closure_job_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND (NEW.operation_id,NEW.command,NEW.command_binding,NEW.intent,NEW.account_id,NEW.termination_id,
 NEW.lifecycle_generation,NEW.source_revision,NEW.scheduled_for_source,NEW.recovery_generation,NEW.created_at) IS NOT DISTINCT FROM
 (OLD.operation_id,OLD.command,OLD.command_binding,OLD.intent,OLD.account_id,OLD.termination_id,
 OLD.lifecycle_generation,OLD.source_revision,OLD.scheduled_for_source,OLD.recovery_generation,OLD.created_at)
 AND (OLD.receipt IS NULL OR (NEW.receipt,NEW.result_code,NEW.decided_at) IS NOT DISTINCT FROM (OLD.receipt,OLD.result_code,OLD.decided_at))
 AND (NOT OLD.receipt_acknowledged OR NEW.receipt_acknowledged OR EXISTS(SELECT 1 FROM game_account_closure_publication_clock WHERE singleton AND resync_pending)) THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'immutable account closure operation' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_account_closure_job_guard BEFORE UPDATE OR DELETE ON game_account_closure_jobs
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_job_guard();

CREATE FUNCTION game_account_closure_closed(p_account UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ SELECT EXISTS(SELECT 1 FROM public.game_account_closures
 WHERE account_id=p_account AND state<>'CANCELLED') $$;
CREATE FUNCTION game_account_closure_detached(p_character UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ SELECT EXISTS(SELECT 1 FROM public.game_account_closure_members
 WHERE character_id=p_character AND (step IN (1,3) OR (step=2 AND EXISTS(SELECT 1 FROM public.game_account_closures c WHERE c.account_id=game_account_closure_members.account_id AND c.close_operation_id=game_account_closure_members.close_operation_id AND c.state<>'CANCELLED')))) $$;

-- This additive owner guard executes for all existing root writers, including
-- bootstrap and incoming transfer. Both accounts are locked in UUID order.
CREATE FUNCTION game_account_closure_root_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE a UUID; BEGIN
 IF TG_OP='INSERT' OR NEW.account_id IS DISTINCT FROM OLD.account_id OR NEW.world_id IS DISTINCT FROM OLD.world_id THEN
 FOR a IN SELECT DISTINCT v FROM unnest(ARRAY[NEW.account_id,CASE WHEN TG_OP='UPDATE' THEN OLD.account_id ELSE NULL END]) v
 WHERE v IS NOT NULL ORDER BY v LOOP
  PERFORM 1 FROM public.game_character_account_guards WHERE account_id=a FOR UPDATE;
  IF public.game_account_closure_closed(a) AND (TG_OP='INSERT' OR NEW.account_id IS DISTINCT FROM OLD.account_id OR NEW.world_id IS DISTINCT FROM OLD.world_id) THEN
   RAISE EXCEPTION 'account portfolio closing' USING ERRCODE='55000'; END IF;
 END LOOP;
 END IF;
 IF public.game_account_closure_detached(NEW.character_id) THEN
  RAISE EXCEPTION 'character in private quarantine' USING ERRCODE='55000'; END IF;
 RETURN NEW; END $$;
CREATE TRIGGER account_closure_root_guard BEFORE INSERT OR UPDATE ON game_character_roots
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_root_guard();
CREATE FUNCTION game_account_closure_admission_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ BEGIN
 IF NEW.session_state IN (1,2)
 AND (TG_OP='INSERT' OR OLD.session_state=3 OR NEW.game_session_id IS DISTINCT FROM OLD.game_session_id OR (OLD.session_state=1 AND NEW.session_state=2) OR NEW.current_generation>OLD.current_generation OR (NEW.current_transport_ref IS DISTINCT FROM OLD.current_transport_ref AND NEW.current_transport_ref IS NOT NULL)) THEN
  PERFORM 1 FROM public.game_character_account_guards WHERE account_id=NEW.account_id FOR UPDATE;
  IF public.game_account_closure_closed(NEW.account_id) THEN RAISE EXCEPTION 'account admission closed' USING ERRCODE='55000'; END IF;
 END IF;
 RETURN NEW; END $$;
CREATE TRIGGER account_closure_admission_guard BEFORE INSERT OR UPDATE ON game_durability_reconnect_sessions
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_admission_guard();

-- Freeze exact custody rows in place after detach. Existing owner transactions
-- continue to own their conservation/fence checks; this adds a refusal only.
CREATE FUNCTION game_account_closure_custody_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE j JSONB; c UUID; i UUID; BEGIN
 FOR j IN SELECT x FROM unnest(ARRAY[CASE WHEN TG_OP<>'INSERT' THEN to_jsonb(OLD) END,
 CASE WHEN TG_OP<>'DELETE' THEN to_jsonb(NEW) END]) x WHERE x IS NOT NULL LOOP
  c:=coalesce(j->>'character_id',j->>'owner_character_id')::uuid;
  IF c IS NOT NULL THEN PERFORM 1 FROM public.game_character_roots WHERE character_id=c FOR UPDATE; END IF;
  IF c IS NOT NULL AND public.game_account_closure_detached(c) THEN
   RAISE EXCEPTION 'quarantined character custody' USING ERRCODE='55000'; END IF;
  i:=(j->>'item_instance_id')::uuid;
  IF i IS NOT NULL THEN
   FOR c IN SELECT DISTINCT character_id FROM (
    SELECT character_id FROM public.game_item_container_slots WHERE item_instance_id=i UNION ALL
    SELECT character_id FROM public.game_item_container_entries WHERE item_instance_id=i UNION ALL
    SELECT character_id FROM public.game_character_equipment_slots WHERE item_instance_id=i UNION ALL
    SELECT character_id FROM public.game_item_character_depot_locations WHERE item_instance_id=i UNION ALL
    SELECT character_id FROM public.game_item_character_inbox_locations WHERE item_instance_id=i
   ) owners ORDER BY character_id LOOP
    PERFORM 1 FROM public.game_character_roots WHERE character_id=c FOR UPDATE;
    IF public.game_account_closure_detached(c) THEN RAISE EXCEPTION 'quarantined item custody' USING ERRCODE='55000'; END IF;
   END LOOP;
  END IF;
 END LOOP;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW; END $$;
DO $$ DECLARE t RECORD; BEGIN
 FOR t IN SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname=current_schema() AND c.relkind='r' AND (c.relname LIKE 'game_item_%' OR c.relname LIKE 'game_character_%')
 AND c.relname NOT LIKE '%audit%' AND c.relname NOT IN ('game_character_roots','game_character_account_guards','game_character_name_reservations')
 AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname IN ('character_id','item_instance_id') AND NOT a.attisdropped)
 LOOP EXECUTE format('CREATE TRIGGER account_closure_custody_guard BEFORE INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_account_closure_custody_guard()',t.relname); END LOOP;
END $$;
CREATE FUNCTION game_account_closure_domain_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE j JSONB; a UUID; g UUID; ch UUID; BEGIN
 FOR j IN SELECT x FROM unnest(ARRAY[CASE WHEN TG_OP<>'INSERT' THEN to_jsonb(OLD) END,
 CASE WHEN TG_OP<>'DELETE' THEN to_jsonb(NEW) END]) x WHERE x IS NOT NULL LOOP
 a:=coalesce(j->>'owner_account_id',j->>'account_id')::uuid; g:=(j->>'guild_id')::uuid;
 IF a IS NOT NULL THEN PERFORM 1 FROM public.game_character_account_guards WHERE account_id=a FOR UPDATE; END IF;
 IF g IS NOT NULL THEN PERFORM 1 FROM public.game_guilds WHERE guild_id=g FOR UPDATE; END IF;
 FOREACH ch IN ARRAY ARRAY[(j->>'character_id')::uuid,(j->>'owner_character_id')::uuid,(j->>'inviter')::uuid] LOOP
 IF ch IS NOT NULL THEN PERFORM 1 FROM public.game_character_roots WHERE character_id=ch FOR UPDATE; END IF;
 IF ch IS NOT NULL AND EXISTS(SELECT 1 FROM public.game_character_roots r WHERE r.character_id=ch AND public.game_account_closure_closed(r.account_id)) THEN
 RAISE EXCEPTION 'account domain participant quarantined' USING ERRCODE='55000'; END IF;
 END LOOP;
 IF (a IS NOT NULL AND public.game_account_closure_closed(a)) OR (g IS NOT NULL AND EXISTS(
 SELECT 1 FROM public.game_guild_members m JOIN public.game_character_roots r USING(character_id)
 WHERE m.guild_id=g AND public.game_account_closure_closed(r.account_id))) THEN
 RAISE EXCEPTION 'reversible account closure domain prepare pending' USING ERRCODE='55000'; END IF;
 END LOOP;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW; END $$;
CREATE TRIGGER account_closure_house_freeze BEFORE UPDATE OR DELETE ON game_house_properties FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_guild_freeze BEFORE UPDATE OR DELETE ON game_guilds FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_market_new BEFORE INSERT ON game_market_offers FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();

-- The private ownership override changes public revisions without changing the
-- immutable creation root or the original ItemInstance/binding facts.
CREATE FUNCTION game_account_closure_member_projection() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE r public.game_character_roots%ROWTYPE; cat SMALLINT; BEGIN
 IF NEW.step<>OLD.step AND (NEW.step IN(1,2)) THEN
  SELECT * INTO STRICT r FROM public.game_character_roots WHERE character_id=NEW.character_id;
  PERFORM public.game_character_account_projection_touch(NEW.account_id);
  PERFORM public.game_public_profile_touch(NEW.character_id,CASE WHEN NEW.step=1 THEN r.name_key END,r.world_id);
  FOR cat IN SELECT category FROM public.game_world_highscore_heads WHERE world_id=r.world_id ORDER BY category LIMIT 32 LOOP
   PERFORM public.game_public_highscores_touch(r.world_id,cat);
  END LOOP;
 END IF; RETURN NULL; END $$;
CREATE TRIGGER account_closure_member_projection AFTER UPDATE ON game_account_closure_members FOR EACH ROW EXECUTE FUNCTION game_account_closure_member_projection();

REVOKE ALL ON game_account_closure_jobs,game_account_closures,game_account_closure_members,
 game_account_closure_offline,game_account_closure_lifecycle,game_account_closure_publications,
 game_account_closure_publication_clock,game_account_closure_audit_outbox FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_account_closure_jobs,game_account_closures,game_account_closure_members,
 game_account_closure_offline,game_account_closure_publications TO oteryn_game_runtime;
GRANT SELECT ON game_account_closure_publication_clock TO oteryn_game_runtime;
GRANT UPDATE(resync_after,resync_pending,watermark_at,quarantined) ON game_account_closure_publication_clock TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_account_closure_audit_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_account_closure_lifecycle TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_account_closure_lifecycle TO oteryn_game_control;
GRANT SELECT ON game_account_closure_jobs,game_account_closures,game_account_closure_members,
 game_account_closure_offline,game_account_closure_publications,game_account_closure_publication_clock,
 game_account_closure_audit_outbox TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_account_closure_closed(UUID),game_account_closure_detached(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_closed(UUID),game_account_closure_detached(UUID) TO oteryn_game_runtime,oteryn_game_control;
-- The isolated highscore source cut also filters private closure custody. Expose
-- only the definer's boolean predicate, never the closure tables themselves.
GRANT EXECUTE ON FUNCTION game_account_closure_detached(UUID) TO oteryn_game_highscore_job;
CREATE FUNCTION game_account_closure_touch_portfolio(a UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.game_account_closures WHERE account_id=a) THEN RAISE EXCEPTION 'closure missing'; END IF;
 PERFORM public.game_character_account_projection_touch(a);
END $$;
REVOKE ALL ON FUNCTION game_account_closure_touch_portfolio(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_touch_portfolio(UUID) TO oteryn_game_runtime;

-- The original immutable name reservation permits this one terminal, publicly
-- tombstoned disposition only. Creation roots remain immutable and private.
DROP TRIGGER game_character_name_reservation_immutable ON game_character_name_reservations;
CREATE FUNCTION game_account_closure_name_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM public.game_account_closure_members m
 JOIN public.game_account_closures c USING(account_id) WHERE m.character_id=OLD.character_id
 AND m.step=3 AND c.close_operation_id=m.close_operation_id AND c.state='FINALIZING' AND c.decision='FINALIZE'
 AND c.name_release_after_seconds IS NOT NULL AND game_account_closure_now()>=c.closed_at+c.name_release_after_seconds)
 AND NOT EXISTS(SELECT 1 FROM public.game_public_character_profile_outbox WHERE character_id=OLD.character_id)
 AND NOT EXISTS(SELECT 1 FROM public.game_public_name_handoffs WHERE character_id=OLD.character_id) THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'immutable name reservation' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_character_name_reservation_immutable BEFORE UPDATE OR DELETE ON game_character_name_reservations
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_name_guard();
CREATE FUNCTION game_account_closure_release_name(c UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ BEGIN
 DELETE FROM public.game_character_name_reservations WHERE character_id=c;
END $$;
REVOKE ALL ON FUNCTION game_account_closure_release_name(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_release_name(UUID) TO oteryn_game_runtime;
GRANT INSERT,UPDATE ON game_account_closure_lifecycle TO oteryn_game_runtime;
CREATE FUNCTION game_account_closure_now() RETURNS BIGINT LANGUAGE sql STABLE
 SET search_path=pg_catalog AS $$ SELECT floor(extract(epoch FROM statement_timestamp()))::bigint $$;
REVOKE ALL ON FUNCTION game_account_closure_now() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_now() TO oteryn_game_runtime,oteryn_game_control;
-- New account-scoped obligations cannot enter a closing portfolio. Existing
-- market sagas retain their own settlement path; ambiguous escrow blocks proof.
CREATE FUNCTION game_account_closure_bank_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$
DECLARE j JSONB; a UUID; closed BIGINT; e public.game_account_bank_entries%ROWTYPE;
BEGIN
 j:=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 a:=(j->>'account_id')::uuid;
 PERFORM 1 FROM public.game_character_account_guards WHERE account_id=a FOR UPDATE;
 IF NOT public.game_account_closure_closed(a) THEN
  IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
 END IF;
 SELECT closed_at INTO closed FROM public.game_account_closures WHERE account_id=a AND state='CLOSING';
 -- Existing Market writers seed a zero balance with ON CONFLICT DO NOTHING.
 -- An existing row makes that seed harmless; its real UPDATE is checked below.
 IF closed IS NOT NULL AND TG_TABLE_NAME='game_account_bank_balances' AND TG_OP='INSERT'
 AND (j->>'balance')::numeric=0 AND j->>'last_entry_id' IS NULL
 AND EXISTS(SELECT 1 FROM public.game_account_bank_balances b WHERE b.account_id=a AND b.world_id=(j->>'world_id')::uuid) THEN RETURN NEW; END IF;
 IF closed IS NOT NULL AND TG_TABLE_NAME='game_account_bank_entries' AND TG_OP='INSERT' THEN
  SELECT * INTO e FROM jsonb_populate_record(NULL::public.game_account_bank_entries,j);
 ELSIF closed IS NOT NULL AND TG_TABLE_NAME='game_account_bank_balances' AND TG_OP='UPDATE'
  AND (to_jsonb(NEW)-'balance'-'last_entry_id')=(to_jsonb(OLD)-'balance'-'last_entry_id') THEN
  SELECT * INTO e FROM public.game_account_bank_entries WHERE entry_id=(j->>'last_entry_id')::uuid;
  IF e.balance_before IS DISTINCT FROM (to_jsonb(OLD)->>'balance')::bigint
   OR e.balance_after IS DISTINCT FROM (j->>'balance')::bigint
   OR e.previous_entry_id IS DISTINCT FROM (to_jsonb(OLD)->>'last_entry_id')::uuid THEN
   RAISE EXCEPTION 'closing bank chain mismatch' USING ERRCODE='55000';
  END IF;
 ELSE RAISE EXCEPTION 'new bank work on closing account' USING ERRCODE='55000';
 END IF;
 -- Market remains SELECT-only to runtime. Only its existing fenced definer
 -- owners can create same-xact settlement/value/receipt conservation evidence.
 IF e.account_id=a AND e.world_id=(j->>'world_id')::uuid
 AND e.created_xact_id=pg_current_xact_id() AND NOT public.game_account_closure_detached(e.acting_character_id)
 AND EXISTS(SELECT 1 FROM public.game_market_settlements s JOIN public.game_market_offers origin ON origin.offer_id=s.offer_id
 JOIN public.game_market_offers owned ON owned.account_id=a AND owned.world_id=s.world_id
  AND owned.definition_key=origin.definition_key AND owned.definition_revision=origin.definition_revision
 WHERE s.transaction_id=e.market_transaction_id AND s.world_id=e.world_id AND s.created_xact_id=pg_current_xact_id()
 AND owned.character_id=e.acting_character_id
 AND ((s.kind=6 AND s.account_id=a AND owned.offer_id=s.offer_id AND e.market_kind='MARKET_HELD_CREDIT')
  OR (s.kind IN(3,4) AND s.account_id=a AND owned.offer_id=s.offer_id AND e.market_kind='MARKET_ESCROW_RETURN')
  OR (s.kind=5 AND ((owned.side=2 AND e.market_kind='MARKET_SALE') OR (owned.side=1 AND e.market_kind='MARKET_ESCROW_RETURN')))))
 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'unproven closing market settlement' USING ERRCODE='55000';
END $$;
DO $$ DECLARE t RECORD; BEGIN
 FOR t IN SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname=current_schema() AND c.relkind='r' AND c.relname LIKE 'game_account_bank_%'
 AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname='account_id' AND NOT a.attisdropped)
 LOOP EXECUTE format('CREATE TRIGGER account_closure_bank_guard BEFORE INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_account_closure_bank_guard()',t.relname); END LOOP;
END $$;
CREATE TRIGGER account_closure_house_bid_guard BEFORE INSERT ON game_house_bids FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_housing_slot_guard BEFORE INSERT ON game_account_housing_slots FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TABLE game_account_closure_erasure_queue (
 account_id UUID PRIMARY KEY REFERENCES game_account_closures,
 operation_id UUID NOT NULL UNIQUE REFERENCES game_account_closure_jobs,
 -- NULL means pending domain-retention policy, never erase immediately.
 not_before BIGINT, authoritative_proof_retention_seconds NUMERIC(20,0),
 state TEXT NOT NULL DEFAULT 'PENDING_DOMAIN_RETENTION' CHECK(state='PENDING_DOMAIN_RETENTION')
);
REVOKE ALL ON game_account_closure_erasure_queue FROM PUBLIC;
GRANT SELECT,INSERT ON game_account_closure_erasure_queue TO oteryn_game_runtime;
GRANT SELECT ON game_account_closure_erasure_queue TO oteryn_game_control;
CREATE FUNCTION game_account_closure_audit_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='DELETE' AND NOT OLD.legal_hold AND OLD.expires_at<=game_account_closure_now()*1000 THEN RETURN OLD; END IF;
 IF TG_OP='UPDATE' AND (NEW.event_id,NEW.transaction_id,NEW.operation_id,NEW.envelope,NEW.payload,NEW.payload_sha256,
 NEW.occurred_at,NEW.expires_at,NEW.event_type_id,NEW.schema_revision,NEW.retention_profile_id) IS NOT DISTINCT FROM
 (OLD.event_id,OLD.transaction_id,OLD.operation_id,OLD.envelope,OLD.payload,OLD.payload_sha256,
 OLD.occurred_at,OLD.expires_at,OLD.event_type_id,OLD.schema_revision,OLD.retention_profile_id)
 AND NEW.publication_state>=OLD.publication_state THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'immutable finite account closure audit' USING ERRCODE='23514'; END $$;
CREATE TRIGGER account_closure_audit_guard BEFORE UPDATE OR DELETE ON game_account_closure_audit_outbox
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_audit_guard();
GRANT UPDATE(publication_state),DELETE ON game_account_closure_audit_outbox TO oteryn_game_runtime;
GRANT UPDATE(legal_hold) ON game_account_closure_audit_outbox TO oteryn_game_control;
CREATE FUNCTION game_account_closure_resync(e BIGINT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ BEGIN
 UPDATE public.game_account_closure_publication_clock SET qualified_recovery_generation=(SELECT max(recovery_generation) FROM public.game_character_recovery_admissions),projection_epoch=e,resync_after=NULL,resync_pending=true,quarantined=false,watermark_at=0
 WHERE singleton AND e>projection_epoch;
 IF NOT FOUND THEN RAISE EXCEPTION 'closure epoch must advance' USING ERRCODE='23514'; END IF;
END $$;
REVOKE ALL ON FUNCTION game_account_closure_resync(BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_resync(BIGINT) TO oteryn_game_control;
CREATE TRIGGER account_closure_guild_member_guard BEFORE INSERT OR UPDATE OR DELETE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_guild_invitation_guard BEFORE INSERT OR UPDATE OR DELETE ON game_guild_invitations FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_guild_claim_guard BEFORE INSERT OR UPDATE OR DELETE ON game_guild_disband_claims FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();
CREATE TRIGGER account_closure_guild_leadership_guard BEFORE INSERT OR UPDATE OR DELETE ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION game_account_closure_domain_guard();

-- Hash exact private custody/gameplay state without copying it into proof or
-- audit. A portfolio beyond the bounded preparation envelope remains PENDING.
CREATE FUNCTION game_account_closure_custody_hash(ch UUID) RETURNS BYTEA LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$
DECLARE t RECORD; row_value JSONB; chain BYTEA:=sha256(''::bytea); used BIGINT:=0; owned UUID[]; n BIGINT; query TEXT;
BEGIN
 SELECT array_agg(item_instance_id),count(*) INTO owned,n FROM (
  SELECT item_instance_id FROM public.game_item_container_slots WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_container_entries WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_character_equipment_slots WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_character_depot_locations WHERE character_id=ch UNION ALL
  SELECT item_instance_id FROM public.game_item_character_inbox_locations WHERE character_id=ch LIMIT 65537
 ) items;
 IF n>65536 THEN RAISE EXCEPTION 'closure custody bound' USING ERRCODE='54000'; END IF;
 FOR t IN SELECT c.relname,
 EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname='character_id' AND NOT a.attisdropped) AS direct
 FROM pg_class c JOIN pg_namespace ns ON ns.oid=c.relnamespace
 WHERE ns.nspname='public' AND c.relkind='r'
 AND (c.relname LIKE 'game_character_%' OR c.relname LIKE 'game_item_%')
 AND c.relname NOT LIKE '%audit%' AND c.relname NOT LIKE '%receipt%'
 AND c.relname NOT LIKE '%reservation%' AND c.relname NOT LIKE '%history%'
 AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname IN ('character_id','item_instance_id') AND NOT a.attisdropped)
 ORDER BY c.relname LOOP
 chain:=sha256(chain||convert_to(t.relname,'UTF8'));
 IF t.direct THEN
  query:=format('SELECT j FROM (SELECT to_jsonb(r) j FROM public.%I r WHERE character_id=$1 LIMIT 65537) bounded ORDER BY j::text',t.relname);
 ELSE
  query:=format('SELECT j FROM (SELECT to_jsonb(r) j FROM public.%I r WHERE item_instance_id=ANY($2) LIMIT 65537) bounded ORDER BY j::text',t.relname);
 END IF;
 FOR row_value IN EXECUTE query USING ch,owned LOOP
  used:=used+1;
  IF used>65536 OR octet_length(row_value::text)>1048576 THEN RAISE EXCEPTION 'closure custody bound' USING ERRCODE='54000'; END IF;
  chain:=sha256(chain||convert_to(row_value::text,'UTF8'));
 END LOOP;
 END LOOP;
 RETURN chain;
END $$;
REVOKE ALL ON FUNCTION game_account_closure_custody_hash(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_custody_hash(UUID) TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_account_closure_member_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND (NEW.close_operation_id,NEW.character_id,NEW.account_id) IS NOT DISTINCT FROM
 (OLD.close_operation_id,OLD.character_id,OLD.account_id) AND EXISTS(SELECT 1 FROM game_account_closures c
 WHERE c.account_id=NEW.account_id AND c.close_operation_id=NEW.close_operation_id AND
 ((OLD.step=2 AND NEW.step=2 AND c.state='CANCELLING' AND NEW.verified_recovery_generation=(SELECT max(recovery_generation) FROM game_character_recovery_admissions)) OR
 (OLD.step=1 AND NEW.step=1 AND c.state IN('CLOSING','CLOSED_DETACHED') AND NEW.verified_recovery_generation=(SELECT max(recovery_generation) FROM game_character_recovery_admissions)) OR
 (OLD.step=0 AND NEW.step=1 AND c.state='CLOSING') OR
 (OLD.step IN(0,1) AND NEW.step=2 AND c.state='CANCELLING' AND c.decision='CANCEL') OR
 (OLD.step=1 AND NEW.step=3 AND c.state='FINALIZING' AND c.decision='FINALIZE')))
 AND (OLD.step=0 OR (NEW.root_revision,NEW.custody_sha256) IS NOT DISTINCT FROM (OLD.root_revision,OLD.custody_sha256))
 AND (NEW.step<>1 OR NEW.custody_sha256 IS NOT NULL) THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'account closure member transition refused' USING ERRCODE='23514'; END $$;
CREATE TRIGGER account_closure_member_guard BEFORE UPDATE OR DELETE ON game_account_closure_members
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_member_guard();
CREATE FUNCTION game_account_closure_state_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND NEW.account_id=OLD.account_id THEN
  IF OLD.state='CANCELLED' AND NEW.state='CLOSING' AND NEW.termination_id<>OLD.termination_id
   AND NEW.lifecycle_generation>OLD.lifecycle_generation AND NEW.close_generation=NEW.lifecycle_generation AND NEW.decision IS NULL THEN RETURN NEW; END IF;
  IF (NEW.termination_id,NEW.close_operation_id,NEW.close_generation,NEW.closed_at) IS NOT DISTINCT FROM
   (OLD.termination_id,OLD.close_operation_id,OLD.close_generation,OLD.closed_at)
   AND (OLD.decision IS NULL OR NEW.decision=OLD.decision)
   AND ((NEW.decision IS NULL AND NEW.lifecycle_generation=NEW.close_generation) OR (NEW.decision IS NOT NULL AND NEW.lifecycle_generation=NEW.close_generation+1))
   AND (NEW.manifest=OLD.manifest OR OLD.state='CLOSING')
   AND ((NEW.state=OLD.state) OR (OLD.state='CLOSING' AND NEW.state IN('CLOSED_DETACHED','CANCELLING'))
     OR (OLD.state='CLOSED_DETACHED' AND NEW.state IN('CANCELLING','FINALIZING'))
     OR (OLD.state='CANCELLING' AND NEW.state='CANCELLED') OR (OLD.state='FINALIZING' AND NEW.state='FINALIZED'))
   THEN RETURN NEW; END IF;
 END IF;
 RAISE EXCEPTION 'account closure state transition refused' USING ERRCODE='23514'; END $$;
CREATE TRIGGER account_closure_state_guard BEFORE UPDATE OR DELETE ON game_account_closures
 FOR EACH ROW EXECUTE FUNCTION game_account_closure_state_guard();
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_account_closure_jobs','game_account_closures','game_account_closure_members','game_account_closure_offline',
 'game_account_closure_lifecycle','game_account_closure_publications','game_account_closure_publication_clock','game_account_closure_audit_outbox','game_account_closure_erasure_queue'] LOOP
 EXECUTE format('CREATE TRIGGER account_closure_no_truncate BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_character_reject_truncate()',t);
 END LOOP;
END $$;
-- Legal holds require an explicit case/reason and the authenticated control
-- principal. Release never refreshes the audit's original P90D expiry.
CREATE TABLE game_account_closure_audit_holds (
 hold_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7(),
 event_id UUID NOT NULL REFERENCES game_account_closure_audit_outbox ON DELETE CASCADE,
 case_ref TEXT NOT NULL CHECK(length(case_ref) BETWEEN 1 AND 128 AND btrim(case_ref)<>''),
 reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 512 AND btrim(reason)<>''),
 held_by TEXT NOT NULL,held_at BIGINT NOT NULL,
 released_by TEXT,released_at BIGINT,
 CHECK((released_by IS NULL)=(released_at IS NULL)),CHECK(released_at IS NULL OR released_at>=held_at)
);
CREATE INDEX game_account_closure_audit_holds_event ON game_account_closure_audit_holds(event_id);
CREATE FUNCTION game_account_closure_hold_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF (NEW.hold_id,NEW.event_id,NEW.case_ref,NEW.reason,NEW.held_by,NEW.held_at) IS NOT DISTINCT FROM
 (OLD.hold_id,OLD.event_id,OLD.case_ref,OLD.reason,OLD.held_by,OLD.held_at)
 AND OLD.released_at IS NULL AND NEW.released_at IS NOT NULL AND NEW.released_by=session_user THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'immutable legal hold provenance' USING ERRCODE='23514'; END $$;
CREATE TRIGGER account_closure_hold_guard BEFORE UPDATE ON game_account_closure_audit_holds FOR EACH ROW EXECUTE FUNCTION game_account_closure_hold_guard();
CREATE TRIGGER account_closure_hold_no_truncate BEFORE TRUNCATE ON game_account_closure_audit_holds EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_account_closure_audit_holds FROM PUBLIC;
GRANT SELECT ON game_account_closure_audit_holds TO oteryn_game_runtime,oteryn_game_control;
REVOKE UPDATE(legal_hold) ON game_account_closure_audit_outbox FROM oteryn_game_control;
CREATE FUNCTION game_account_closure_hold_audit(e UUID,c TEXT,r TEXT) RETURNS UUID LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE h UUID; BEGIN
 PERFORM 1 FROM public.game_account_closure_audit_outbox WHERE event_id=e FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'audit event not retained' USING ERRCODE='23514'; END IF;
 INSERT INTO public.game_account_closure_audit_holds(event_id,case_ref,reason,held_by,held_at)
 VALUES(e,c,r,session_user,public.game_account_closure_now()*1000) RETURNING hold_id INTO h;
 UPDATE public.game_account_closure_audit_outbox SET legal_hold=true WHERE event_id=e;
 RETURN h; END $$;
CREATE FUNCTION game_account_closure_release_audit_hold(h UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog,public AS $$ DECLARE e UUID; BEGIN
 SELECT event_id INTO e FROM public.game_account_closure_audit_holds WHERE hold_id=h;
 IF NOT FOUND THEN RETURN; END IF;
 PERFORM 1 FROM public.game_account_closure_audit_outbox WHERE event_id=e FOR UPDATE;
 UPDATE public.game_account_closure_audit_holds SET released_by=session_user,
 released_at=greatest(held_at,public.game_account_closure_now()*1000) WHERE hold_id=h AND released_at IS NULL;
 UPDATE public.game_account_closure_audit_outbox SET legal_hold=EXISTS(SELECT 1 FROM public.game_account_closure_audit_holds WHERE event_id=e AND released_at IS NULL) WHERE event_id=e;
 END $$;
REVOKE ALL ON FUNCTION game_account_closure_hold_audit(UUID,TEXT,TEXT),game_account_closure_release_audit_hold(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_account_closure_hold_audit(UUID,TEXT,TEXT),game_account_closure_release_audit_hold(UUID) TO oteryn_game_control;

CREATE FUNCTION game_account_closure_clock_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF TG_OP='UPDATE' AND NEW.singleton=OLD.singleton AND NEW.projection_epoch>=OLD.projection_epoch
 AND NEW.qualified_recovery_generation>=OLD.qualified_recovery_generation
 AND (NEW.qualified_recovery_generation=OLD.qualified_recovery_generation OR NEW.projection_epoch>OLD.projection_epoch)
 AND (NEW.projection_epoch>OLD.projection_epoch OR NEW.watermark_at>=OLD.watermark_at) THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'account closure epoch/watermark regressed' USING ERRCODE='23514'; END $$;
CREATE TRIGGER account_closure_clock_guard BEFORE UPDATE OR DELETE ON game_account_closure_publication_clock FOR EACH ROW EXECUTE FUNCTION game_account_closure_clock_guard();
REVOKE UPDATE ON game_account_closure_offline FROM oteryn_game_runtime;
CREATE TRIGGER account_closure_offline_immutable BEFORE UPDATE OR DELETE ON game_account_closure_offline
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE INDEX game_account_closure_session_lookup ON game_durability_reconnect_sessions(character_id,character_lease_generation DESC,game_session_id);

-- Match bounded delivery, expiry, source-identity and owner-work reads without
-- scanning indefinitely retained terminal history.
CREATE INDEX game_account_closure_jobs_command_pending ON game_account_closure_jobs(command,created_at,operation_id) WHERE receipt IS NULL;
CREATE INDEX game_account_closure_jobs_source_revision ON game_account_closure_jobs(command,source_revision,operation_id);
CREATE INDEX game_account_closure_jobs_delivery ON game_account_closure_jobs(decided_at,operation_id) WHERE receipt IS NOT NULL AND NOT receipt_acknowledged AND NOT receipt_quarantined;
CREATE INDEX game_account_closure_jobs_unacknowledged ON game_account_closure_jobs(operation_id) WHERE receipt IS NOT NULL AND NOT receipt_acknowledged;
CREATE INDEX game_account_closure_publications_delivery ON game_account_closure_publications(created_at,account_id) WHERE NOT acknowledged AND NOT quarantined;
CREATE INDEX game_account_closure_publications_unacknowledged ON game_account_closure_publications(account_id) WHERE NOT acknowledged;
CREATE INDEX game_account_closure_audit_expiry ON game_account_closure_audit_outbox(expires_at,event_id) WHERE NOT legal_hold;
CREATE INDEX game_account_closure_audit_delivery ON game_account_closure_audit_outbox(occurred_at,event_id) WHERE publication_state=1;
CREATE INDEX game_account_closure_owner_work ON game_account_closures(state,account_id,close_operation_id) WHERE state IN('CLOSING','CANCELLING');
CREATE INDEX game_account_closure_jobs_deadline ON game_account_closure_jobs(account_id,termination_id,lifecycle_generation,scheduled_for_source,operation_id) WHERE command='CloseAccount' AND result_code IS DISTINCT FROM 'CHAR_CMD_INTENT_INVALID';

-- Restate 0929's strict job-role guard for the one new closure predicate.
-- No other function, table privilege, membership, ownership or grant option is
-- accepted; the original migration remains immutable.
DO $highscore_role_guard$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job') THEN
        -- Roles are cluster-global: a concurrent migration of another database may create it.
        BEGIN
            CREATE ROLE oteryn_game_highscore_job NOLOGIN NOINHERIT;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job'
          AND (rolcanlogin OR rolinherit OR rolsuper OR rolbypassrls OR rolcreaterole OR rolcreatedb
               OR rolreplication)
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job is not a plain NOLOGIN NOINHERIT group'
            USING ERRCODE = '42501';
    END IF;
    -- A membership of its own would let the job's transaction inherit, or SET ROLE to,
    -- privileges beyond the highscore grants below: the group must be a member of no role.
    IF EXISTS (
        SELECT 1 FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member
        WHERE r.rolname = 'oteryn_game_highscore_job'
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job is a member of another role'
            USING ERRCODE = '42501';
    END IF;
    -- A member of the group could inherit, SET ROLE to or grant the highscore privileges below.
    -- Its only members may be the runtime group WITH INHERIT FALSE and without ADMIN (granted
    -- below, or already by another Game database of this cluster) and the ADMIN-only grant that
    -- PostgreSQL gives the migration role when a non-superuser CREATEROLE role creates the group.
    IF EXISTS (
        SELECT 1 FROM pg_auth_members m
        JOIN pg_roles g ON g.oid = m.roleid
        JOIN pg_roles r ON r.oid = m.member
        WHERE g.rolname = 'oteryn_game_highscore_job'
          AND NOT (
              (r.rolname = 'oteryn_game_runtime' AND NOT m.inherit_option AND NOT m.admin_option)
              OR (r.rolname = current_user AND NOT m.inherit_option AND NOT m.set_option)
          )
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job has a member other than the runtime group'
            USING ERRCODE = '42501';
    END IF;
    -- A direct privilege would reach beyond the highscore grants below. pg_shdepend records
    -- every object the group owns and every ACL, default ACL or policy naming it (as grantee or
    -- grantor), in this database and on shared objects (databases, tablespaces, parameters);
    -- another Game database's own highscore grants are its business. The group may own nothing
    -- and appear in no ACL except the exact table-level grants below, without GRANT OPTION and
    -- never as a grantor; the highscore tables and their sequence do not exist yet on a first
    -- run.
    IF EXISTS (
        WITH job AS (
            SELECT oid FROM pg_roles WHERE rolname = 'oteryn_game_highscore_job'
        ), expected AS (
            SELECT to_regclass(t.name)::oid AS relation, t.privileges
            FROM (VALUES
                ('game_world_highscore_snapshots', ARRAY['SELECT', 'INSERT', 'DELETE']),
                ('game_world_highscore_rows', ARRAY['SELECT', 'INSERT', 'DELETE']),
                ('game_world_highscore_heads', ARRAY['SELECT', 'INSERT', 'UPDATE']),
                ('game_character_roots', ARRAY['SELECT']),
                ('game_character_build_state', ARRAY['SELECT']),
                ('game_character_progression_state', ARRAY['SELECT']),
                ('game_account_achievements', ARRAY['SELECT']),
                ('game_account_achievement_grant_requests', ARRAY['SELECT']),
                ('game_character_bestiary_progress', ARRAY['SELECT']),
                ('game_character_bosstiary_progress', ARRAY['SELECT'])
            ) AS t(name, privileges)
            WHERE to_regclass(t.name) IS NOT NULL
            UNION ALL
            SELECT d.objid, ARRAY['USAGE']
            FROM pg_depend d
            WHERE d.classid = 'pg_class'::regclass
              AND d.refclassid = 'pg_class'::regclass
              AND d.refobjid = to_regclass('game_world_highscore_snapshots')::oid
              AND d.deptype = 'i'
              AND EXISTS (SELECT 1 FROM pg_class q WHERE q.oid = d.objid AND q.relkind = 'S')
        )
        SELECT 1 FROM pg_shdepend s, job
        WHERE s.refclassid = 'pg_authid'::regclass AND s.refobjid = job.oid
          AND s.dbid IN (0, (SELECT oid FROM pg_database WHERE datname = current_database()))
          AND NOT (s.deptype = 'a' AND s.objsubid = 0 AND (
                   (s.classid = 'pg_class'::regclass
                    AND EXISTS (SELECT 1 FROM expected e WHERE e.relation = s.objid))
                   OR (s.classid = 'pg_proc'::regclass
                       AND s.objid = 'game_account_closure_detached(uuid)'::regprocedure)))
        UNION ALL
        SELECT 1 FROM expected e JOIN pg_class c ON c.oid = e.relation
        CROSS JOIN LATERAL aclexplode(c.relacl) a, job
        WHERE a.grantor = job.oid
           OR (a.grantee = job.oid
               AND (a.is_grantable OR NOT (a.privilege_type = ANY (e.privileges))))
        UNION ALL
        SELECT 1 FROM pg_proc p CROSS JOIN LATERAL aclexplode(p.proacl) a, job
        WHERE p.oid = 'game_account_closure_detached(uuid)'::regprocedure
          AND (a.grantor = job.oid
               OR (a.grantee = job.oid AND (a.is_grantable OR a.privilege_type <> 'EXECUTE')))
    ) THEN
        RAISE EXCEPTION 'pre-existing Game group role oteryn_game_highscore_job holds a privilege outside the highscore grants'
            USING ERRCODE = '42501';
    END IF;
END $highscore_role_guard$;
