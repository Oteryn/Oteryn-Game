-- PARITY-REPORT-1: Game-owned append-only player report queue (bug and rule-violation reports).
-- Stores only the reporter Character, the resolved reported Character (else the bounded name),
-- kind, reason, action, comment, positions and time: no account e-mail, IP or chat-log copy.
-- Platform export, staff UI and moderation actions are the Platform control-plane boundary.
-- Rollback is additive: revoke the EXECUTE grant and retain the queue and receipts.
CREATE TABLE game_player_reports (
 report_no BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 world_id UUID NOT NULL,
 reporter_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 kind SMALLINT NOT NULL CHECK(kind IN(1,2)),
 category SMALLINT NOT NULL CHECK(category BETWEEN 0 AND 3),
 reason SMALLINT NOT NULL CHECK(reason BETWEEN 0 AND 20),
 action SMALLINT NOT NULL CHECK(action BETWEEN 0 AND 2),
 reported_character_id UUID REFERENCES game_character_roots(character_id),
 reported_name TEXT CHECK(reported_name IS NULL OR game_character_is_name(reported_name)),
 comment TEXT NOT NULL CHECK(octet_length(comment)<=1024),
 map_x INTEGER NOT NULL CHECK(map_x BETWEEN 0 AND 65535),
 map_y INTEGER NOT NULL CHECK(map_y BETWEEN 0 AND 65535),
 map_floor SMALLINT NOT NULL CHECK(map_floor BETWEEN 0 AND 255),
 reporter_x INTEGER NOT NULL, reporter_y INTEGER NOT NULL, reporter_floor SMALLINT NOT NULL,
 reported_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 CHECK((kind=1 AND reason=0 AND action=0 AND reported_character_id IS NULL AND reported_name IS NULL
   AND (category=0 OR (map_x=0 AND map_y=0 AND map_floor=0)))
  OR (kind=2 AND category=0 AND map_x=0 AND map_y=0 AND map_floor=0
   AND (reported_character_id IS NULL)<>(reported_name IS NULL)
   AND reported_character_id IS DISTINCT FROM reporter_character_id))
);
CREATE INDEX game_player_report_pending_characters ON game_player_reports(reporter_character_id,action,reported_character_id) WHERE kind=2;
CREATE INDEX game_player_report_pending_names ON game_player_reports(reporter_character_id,action,game_character_name_key(reported_name)) WHERE kind=2 AND reported_name IS NOT NULL;
-- REPORT1-RL-03/04: one durable token bucket per reporting Character.
CREATE TABLE game_player_report_rate_buckets (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 tokens SMALLINT NOT NULL CHECK(tokens BETWEEN 0 AND 3),
 refilled_at BIGINT NOT NULL CHECK(refilled_at>=0)
);
-- Idempotent receipt keyed by CommandRef. The binding covers the client request only; the
-- server-observed position may move between a lost answer and its replay.
CREATE TABLE game_player_report_receipts (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32), result JSONB NOT NULL,
 report_no BIGINT REFERENCES game_player_reports(report_no),
 committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(game_session_id,command_id)
);
CREATE FUNCTION game_player_report_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'immutable player report evidence' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_player_report_append_only BEFORE UPDATE OR DELETE ON game_player_reports FOR EACH ROW EXECUTE FUNCTION game_player_report_immutable();
CREATE TRIGGER game_player_report_receipt_immutable BEFORE UPDATE OR DELETE ON game_player_report_receipts FOR EACH ROW EXECUTE FUNCTION game_player_report_immutable();

-- Expected recovery/session/node facts are independently compared to current durable truth.
-- `valid` is the protocol owner's semantic shape; the table CHECKs repeat every shape bound.
CREATE FUNCTION game_player_report_submit(p_recovery JSONB,p_fence JSONB,p_command NUMERIC,p_request JSONB)
RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 actor UUID := (p_fence->>'actor')::uuid; session UUID := (p_fence->>'session')::uuid;
 world UUID := (p_fence->>'world')::uuid; account UUID; target UUID;
 client JSONB := p_request->'client'; spot JSONB := p_request->'position';
 operation_binding BYTEA := sha256(convert_to((p_request->'client')::text,'UTF8'));
 v_kind SMALLINT := (p_request->'client'->>'kind')::smallint; label TEXT := p_request->'client'->>'target';
 prior RECORD; live RECORD; bucket RECORD; recovery JSONB; result JSONB; report BIGINT;
 now_s BIGINT; refills BIGINT; invalid CONSTANT JSONB := jsonb_build_object('code',2,'report',0,'wait',0);
BEGIN
 IF p_command<1 OR client IS NULL OR spot IS NULL THEN
  RAISE EXCEPTION 'invalid player report request' USING ERRCODE='23514';
 END IF;
 SELECT jsonb_build_object('scope',authority_scope_id,'generation',recovery_generation::text,
  'event',recovery_event_id::text,'predecessor',predecessor_generation::text,
  'digest',encode(COALESCE(predecessor_digest,decode(repeat('00',32),'hex')),'hex'),
  'issued',issued_at::text,'issuer',issuer_identity) INTO recovery
 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE;
 IF recovery IS NULL OR recovery IS DISTINCT FROM p_recovery THEN
  RAISE EXCEPTION 'stale player report recovery' USING ERRCODE='23514';
 END IF;
 SELECT account_id INTO account FROM game_character_roots WHERE character_id=actor AND world_id=world;
 IF account IS NULL THEN RAISE EXCEPTION 'player report actor binding' USING ERRCODE='23514'; END IF;
 -- One reporting Character owner, shared by every Channel and runtime instance.
 PERFORM pg_advisory_xact_lock(204,hashtext(actor::text));
 SELECT * INTO prior FROM game_player_report_receipts WHERE game_session_id=session AND command_id=p_command;
 IF FOUND THEN
  IF prior.character_id=actor AND prior.binding=operation_binding THEN RETURN prior.result; END IF;
  RETURN invalid;
 END IF;
 SELECT * INTO live FROM game_durability_reconnect_sessions WHERE game_session_id=session
  AND character_id=actor AND account_id=account AND world_id=world AND session_state IN(1,2)
  AND current_generation=(p_fence->>'connection')::numeric
  AND character_lease_generation=(p_fence->>'lease')::numeric
  AND scope_ownership_generation=(p_fence->>'scope_generation')::numeric
  AND runtime_scope_kind=1 AND runtime_scope_world_id=world
  AND runtime_scope_channel_id=(p_fence->>'channel')::uuid AND runtime_scope_instance_id IS NULL FOR SHARE;
 IF NOT FOUND OR NOT game_node_lock_current_registration((p_fence->>'node')::uuid,(p_fence->>'node_revision')::numeric) OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments s
  JOIN game_durability_admission_runtime_guards rg USING(scope_key)
  WHERE s.world_id=world AND s.channel_id=(p_fence->>'channel')::uuid AND s.state=1
   AND s.ownership_generation=live.scope_ownership_generation AND rg.ready AND rg.ownership_generation=s.ownership_generation
   AND s.holder_node_id=(p_fence->>'node')::uuid AND s.holder_registration_revision=(p_fence->>'node_revision')::numeric
 ) OR NOT EXISTS(
  SELECT 1 FROM game_durability_admission_character_guards c JOIN game_durability_admission_account_guards a USING(account_id)
  WHERE c.character_id=actor AND c.account_id=account AND c.world_id=world AND c.eligible
   AND c.lease_generation=live.character_lease_generation AND c.holder_game_session_id=session
   AND a.presence_character_id=actor AND a.holder_game_session_id=session
 ) THEN RAISE EXCEPTION 'stale player report session' USING ERRCODE='23514'; END IF;
 -- Native Rust assert_session_fence additionally holds the registered node incarnation.
 now_s:=floor(extract(epoch FROM clock_timestamp()));
 INSERT INTO game_player_report_rate_buckets(character_id,tokens,refilled_at) VALUES(actor,3,now_s) ON CONFLICT DO NOTHING;
 SELECT * INTO bucket FROM game_player_report_rate_buckets WHERE character_id=actor FOR UPDATE;
 IF bucket.refilled_at>now_s THEN RAISE EXCEPTION 'future player report bucket' USING ERRCODE='23514'; END IF;
 refills:=(now_s-bucket.refilled_at)/60;
 bucket.tokens:=least(3,bucket.tokens+refills);
 bucket.refilled_at:=bucket.refilled_at+refills*60;
 -- Every attempt past the fences spends a token, so refusals cannot flood the owner either.
 IF bucket.tokens=0 THEN
  result:=jsonb_build_object('code',5,'report',0,'wait',greatest(1,60-(now_s-bucket.refilled_at)));
 ELSE
  bucket.tokens:=bucket.tokens-1;
  IF NOT (p_request->>'valid')::boolean OR v_kind NOT IN(1,2) THEN
   result:=invalid;
  ELSE
   IF v_kind=2 THEN
    SELECT character_id INTO target FROM game_character_roots WHERE name_key=game_character_name_key(label) AND world_id=world AND lifecycle=1;
   END IF;
   IF v_kind=2 AND target=actor THEN
    result:=jsonb_build_object('code',3,'report',0,'wait',0);
   -- Canary hasPendingReport: same reporter, reported Character and report type.
   ELSIF v_kind=2 AND EXISTS(SELECT 1 FROM game_player_reports WHERE reporter_character_id=actor AND kind=2
     AND action=(client->>'action')::smallint
     AND (reported_character_id=target OR game_character_name_key(reported_name)=game_character_name_key(label))) THEN
    result:=jsonb_build_object('code',4,'report',0,'wait',0);
   ELSE
    INSERT INTO game_player_reports(world_id,reporter_character_id,kind,category,reason,action,reported_character_id,reported_name,
      comment,map_x,map_y,map_floor,reporter_x,reporter_y,reporter_floor)
     VALUES(world,actor,v_kind,(client->>'category')::smallint,(client->>'reason')::smallint,(client->>'action')::smallint,
      target,CASE WHEN v_kind=2 AND target IS NULL THEN label END,client->>'comment',
      (client->>'map_x')::integer,(client->>'map_y')::integer,(client->>'map_floor')::smallint,
      (spot->>'x')::integer,(spot->>'y')::integer,(spot->>'floor')::smallint)
     RETURNING report_no INTO report;
    result:=jsonb_build_object('code',1,'report',report,'wait',0);
   END IF;
  END IF;
 END IF;
 UPDATE game_player_report_rate_buckets SET tokens=bucket.tokens,refilled_at=bucket.refilled_at WHERE character_id=actor;
 INSERT INTO game_player_report_receipts(game_session_id,command_id,character_id,binding,result,report_no)
  VALUES(session,p_command,actor,operation_binding,result,report);
 RETURN result;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_player_report_submit(jsonb,jsonb,numeric,jsonb) SET search_path = %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_player_report_immutable() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_player_report_submit(jsonb,jsonb,numeric,jsonb),game_player_report_immutable() FROM PUBLIC;
REVOKE ALL ON game_player_reports,game_player_report_rate_buckets,game_player_report_receipts FROM PUBLIC;
REVOKE ALL ON SEQUENCE game_player_reports_report_no_seq FROM PUBLIC;
GRANT SELECT ON game_player_reports,game_player_report_rate_buckets,game_player_report_receipts TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_player_report_submit(jsonb,jsonb,numeric,jsonb) TO oteryn_game_runtime;
