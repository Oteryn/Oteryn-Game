-- VIP-1: Account + World metadata and Character-specific consent. No presence publication.
-- Rollback is additive: revoke the operation grants and retain durable relationships/receipts.
ALTER TABLE game_character_social_settings
 ADD COLUMN vip_status SMALLINT NOT NULL DEFAULT 1 CHECK(vip_status IN(1,2)),
 ADD COLUMN vip_consent_revision BIGINT NOT NULL DEFAULT 0 CHECK(vip_consent_revision>=0);
CREATE TABLE game_vip_lists (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id), world_id UUID NOT NULL,
 revision BIGINT NOT NULL DEFAULT 0 CHECK(revision>=0), next_entry_no BIGINT NOT NULL DEFAULT 1 CHECK(next_entry_no>0),
 rate_tokens SMALLINT NOT NULL DEFAULT 20 CHECK(rate_tokens BETWEEN 0 AND 20),
 rate_at BIGINT NOT NULL DEFAULT floor(extract(epoch FROM clock_timestamp())) CHECK(rate_at>=0),
 PRIMARY KEY(account_id,world_id)
);
CREATE TABLE game_vip_entries (
 account_id UUID NOT NULL, world_id UUID NOT NULL, character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 entry_no BIGINT NOT NULL CHECK(entry_no>0), description TEXT NOT NULL DEFAULT '' CHECK(char_length(description)<=128 AND octet_length(description)<=512),
 icon SMALLINT NOT NULL DEFAULT 0 CHECK(icon BETWEEN 0 AND 10), notify BOOLEAN NOT NULL DEFAULT FALSE,
 added_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(account_id,world_id,character_id), UNIQUE(account_id,world_id,entry_no),
 FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE
);
CREATE TABLE game_vip_groups (
 account_id UUID NOT NULL,world_id UUID NOT NULL,group_no SMALLINT NOT NULL CHECK(group_no BETWEEN 4 AND 8),
 name TEXT NOT NULL CHECK(char_length(name) BETWEEN 1 AND 25 AND octet_length(name)<=100),
 PRIMARY KEY(account_id,world_id,group_no), FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE
);
CREATE UNIQUE INDEX game_vip_group_names ON game_vip_groups(account_id,world_id,lower(name));
CREATE TABLE game_vip_group_members (
 account_id UUID NOT NULL,world_id UUID NOT NULL,character_id UUID NOT NULL,group_no SMALLINT NOT NULL CHECK(group_no BETWEEN 1 AND 8),
 PRIMARY KEY(account_id,world_id,character_id,group_no),
 FOREIGN KEY(account_id,world_id,character_id) REFERENCES game_vip_entries ON DELETE CASCADE
);
CREATE SEQUENCE game_vip_invitation_id NO CYCLE;
CREATE TABLE game_vip_consents (
 account_id UUID NOT NULL,world_id UUID NOT NULL,inviter_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 character_id UUID NOT NULL,invitation_id BIGINT NOT NULL DEFAULT nextval('game_vip_invitation_id') UNIQUE CHECK(invitation_id>0),
 consent SMALLINT NOT NULL CHECK(consent IN(1,2)),invite_expires_at TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(account_id,world_id,inviter_character_id,character_id), CHECK(inviter_character_id<>character_id),
 FOREIGN KEY(account_id,world_id,character_id) REFERENCES game_vip_entries ON DELETE CASCADE
);
CREATE INDEX game_vip_target_consents ON game_vip_consents(character_id,consent);
CREATE INDEX game_vip_inviter_consents ON game_vip_consents(inviter_character_id);
-- Handles are independent of invitation IDs and bound to one GameSession and target.
-- Old mappings remain immutable so lost-answer replays keep their exact invitation binding.
CREATE TABLE game_vip_handles (
 game_session_id UUID NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 handle BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE CHECK(handle>0),invitation_id BIGINT NOT NULL,
 PRIMARY KEY(game_session_id,invitation_id)
);
CREATE TABLE game_vip_command_receipts (
 account_id UUID NOT NULL,world_id UUID NOT NULL,game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),result JSONB NOT NULL,
 committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(account_id,world_id,game_session_id,command_id),
 FOREIGN KEY(account_id,world_id) REFERENCES game_vip_lists ON DELETE CASCADE
);
CREATE FUNCTION game_vip_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'immutable VIP evidence' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_vip_receipt_immutable BEFORE UPDATE OR DELETE ON game_vip_command_receipts FOR EACH ROW EXECUTE FUNCTION game_vip_immutable();
CREATE TRIGGER game_vip_handle_immutable BEFORE UPDATE OR DELETE ON game_vip_handles FOR EACH ROW EXECUTE FUNCTION game_vip_immutable();

-- Internal owner: only the narrow per-operation wrappers below have runtime EXECUTE.
-- Expected recovery/session/node facts are independently compared to current durable truth.
CREATE FUNCTION game_vip_execute(p_recovery JSONB,p_fence JSONB,p_command NUMERIC,p_request JSONB,p_cap INTEGER,p_groups INTEGER)
RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 actor UUID := (p_fence->>'actor')::uuid; session UUID := (p_fence->>'session')::uuid;
 world UUID := (p_fence->>'world')::uuid; account UUID; target UUID; watcher UUID;
 op INTEGER := (p_request->>'op')::integer; n BIGINT := CASE WHEN (p_request->>'number')::numeric BETWEEN 0 AND 9223372036854775807 THEN (p_request->>'number')::bigint ELSE 0 END;
 label TEXT := p_request->>'text'; operation_binding BYTEA := sha256(convert_to(p_request::text,'UTF8'));
 prior RECORD; live RECORD; consent_row RECORD; list_row RECORD;
 result_code INTEGER := 1; result_number BIGINT := 0; seconds INTEGER := 0; now_s BIGINT;
 existing BIGINT; count_rows BIGINT; g INTEGER; changed BOOLEAN := FALSE; view JSONB;
 recovery JSONB;
BEGIN
 IF op NOT BETWEEN 1 AND 12 OR p_cap NOT IN(20,100) OR p_groups NOT IN(0,5) OR p_command<1 THEN
  RAISE EXCEPTION 'invalid VIP request' USING ERRCODE='23514';
 END IF;
 SELECT jsonb_build_object('scope',authority_scope_id,'generation',recovery_generation::text,
  'event',recovery_event_id::text,'predecessor',predecessor_generation::text,
  'digest',encode(COALESCE(predecessor_digest,decode(repeat('00',32),'hex')),'hex'),
  'issued',issued_at::text,'issuer',issuer_identity) INTO recovery
 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE;
 IF recovery IS NULL OR recovery IS DISTINCT FROM p_recovery THEN
  RAISE EXCEPTION 'stale VIP recovery' USING ERRCODE='23514';
 END IF;
 SELECT account_id INTO account FROM game_character_roots WHERE character_id=actor AND world_id=world;
 IF account IS NULL THEN RAISE EXCEPTION 'VIP actor binding' USING ERRCODE='23514'; END IF;
 IF op<>12 THEN
  SELECT * INTO prior FROM game_vip_command_receipts WHERE account_id=account AND world_id=world AND game_session_id=session AND command_id=p_command;
  IF FOUND THEN
   IF prior.character_id=actor AND prior.binding=operation_binding THEN RETURN prior.result; END IF;
   RETURN jsonb_build_object('code',10,'revision',0,'number',0,'wait',0);
  END IF;
 END IF;
 -- One World social owner before roots/lists, shared by every Channel and runtime instance.
 PERFORM pg_advisory_xact_lock(163,hashtext(world::text));
 IF op<>12 THEN
  SELECT * INTO prior FROM game_vip_command_receipts WHERE account_id=account AND world_id=world AND game_session_id=session AND command_id=p_command;
  IF FOUND THEN
   IF prior.character_id=actor AND prior.binding=operation_binding THEN RETURN prior.result; END IF;
   RETURN jsonb_build_object('code',10,'revision',0,'number',0,'wait',0);
  END IF;
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
 ) THEN RAISE EXCEPTION 'stale VIP session' USING ERRCODE='23514'; END IF;
 -- Native Rust assert_session_fence additionally holds the registered node incarnation.
 IF op=1 AND game_character_is_name(label) THEN
  SELECT character_id INTO target FROM game_character_roots WHERE name_key=game_character_name_key(label) AND world_id=world AND lifecycle=1;
 ELSIF op IN(2,3,10) THEN
  SELECT character_id INTO target FROM game_vip_entries WHERE account_id=account AND world_id=world AND entry_no=n;
 ELSIF op IN(7,8,9) THEN
  SELECT c.* INTO consent_row FROM game_vip_handles h JOIN game_vip_consents c USING(invitation_id)
   WHERE h.game_session_id=session AND h.character_id=actor AND h.handle=n AND c.character_id=actor AND c.world_id=world;
  IF FOUND THEN target:=actor; watcher:=consent_row.account_id; END IF;
 END IF;
 PERFORM 1 FROM game_character_roots WHERE character_id IN(actor,target) ORDER BY character_id FOR UPDATE;
 INSERT INTO game_vip_lists(account_id,world_id) VALUES(account,world) ON CONFLICT DO NOTHING;
 PERFORM 1 FROM game_vip_lists WHERE world_id=world AND account_id IN(account,watcher) ORDER BY account_id,world_id FOR UPDATE;
 SELECT * INTO list_row FROM game_vip_lists WHERE account_id=account AND world_id=world;
 now_s:=floor(extract(epoch FROM clock_timestamp()));
 IF list_row.rate_at>now_s THEN RAISE EXCEPTION 'future VIP bucket' USING ERRCODE='23514'; END IF;
 IF op=12 THEN
  IF NOT (p_request->>'valid')::boolean THEN
   RETURN jsonb_build_object('code',10,'revision',list_row.revision,'number',0,'wait',0);
  END IF;
  -- A query cannot grow the handle cache without bound during a long session.
  SELECT count(*) INTO count_rows FROM game_vip_handles WHERE game_session_id=session;
  IF count_rows+(SELECT count(*) FROM game_vip_consents c WHERE c.character_id=actor AND c.world_id=world AND (c.consent=2 OR c.invite_expires_at>clock_timestamp())
   AND NOT EXISTS(SELECT 1 FROM game_vip_handles h WHERE h.game_session_id=session AND h.invitation_id=c.invitation_id))>4096 THEN
   RETURN jsonb_build_object('code',9,'revision',list_row.revision,'number',0,'wait',0);
  END IF;
  INSERT INTO game_vip_handles(game_session_id,character_id,invitation_id)
   SELECT session,actor,c.invitation_id FROM game_vip_consents c WHERE c.character_id=actor AND c.world_id=world AND (c.consent=2 OR c.invite_expires_at>clock_timestamp()) ON CONFLICT DO NOTHING;
  SELECT jsonb_build_object('next_entry',(SELECT CASE WHEN count(*)=26 THEN (array_agg(entry_no ORDER BY entry_no))[25] ELSE 0 END FROM (SELECT entry_no FROM game_vip_entries WHERE account_id=account AND world_id=world AND entry_no>n ORDER BY entry_no LIMIT 26) page),'entries',COALESCE((SELECT jsonb_agg(jsonb_build_object(
   'entry',e.entry_no,'name',r.name,'description',e.description,'icon',e.icon,'notify',e.notify,
   'groups',COALESCE((SELECT jsonb_agg(m.group_no ORDER BY m.group_no) FROM game_vip_group_members m WHERE m.account_id=account AND m.world_id=world AND m.character_id=e.character_id),'[]'::jsonb),
   'confirmed',EXISTS(SELECT 1 FROM game_vip_consents c WHERE c.account_id=account AND c.world_id=world AND c.character_id=e.character_id AND c.inviter_character_id=actor AND c.consent=2)) ORDER BY e.entry_no)
   FROM (SELECT * FROM game_vip_entries WHERE account_id=account AND world_id=world AND entry_no>n ORDER BY entry_no LIMIT 25) e JOIN game_character_roots r USING(character_id)),'[]'::jsonb),
   'groups',COALESCE((SELECT jsonb_agg(jsonb_build_object('group',group_no,'name',name) ORDER BY group_no) FROM game_vip_groups WHERE account_id=account AND world_id=world),'[]'::jsonb),
   'consents',COALESCE((SELECT jsonb_agg(jsonb_build_object('handle',h.handle,'name',r.name,'accepted',c.consent=2,'expires',floor(extract(epoch FROM c.invite_expires_at))) ORDER BY h.handle)
   FROM game_vip_consents c JOIN game_vip_handles h USING(invitation_id) JOIN game_character_roots r ON r.character_id=c.inviter_character_id
   WHERE c.character_id=actor AND c.world_id=world AND h.game_session_id=session AND h.character_id=actor AND (c.consent=2 OR c.invite_expires_at>clock_timestamp())),'[]'::jsonb),
   'shown',COALESCE((SELECT vip_status=1 FROM game_character_social_settings WHERE character_id=actor),TRUE),
   'consent_revision',COALESCE((SELECT vip_consent_revision FROM game_character_social_settings WHERE character_id=actor),0)) INTO view;
  RETURN jsonb_build_object('code',1,'revision',list_row.revision,'number',0,'wait',0,'view',view);
 END IF;
 list_row.rate_tokens:=least(20,list_row.rate_tokens+(now_s-list_row.rate_at)/3);
 list_row.rate_at:=list_row.rate_at+((now_s-list_row.rate_at)/3)*3;
 IF list_row.rate_tokens=0 THEN result_code:=8; seconds:=greatest(1,3-(now_s-list_row.rate_at));
 ELSE
  list_row.rate_tokens:=list_row.rate_tokens-1;
  IF NOT (p_request->>'valid')::boolean THEN result_code:=10;
  ELSE
  CASE op
  WHEN 1 THEN
   IF NOT game_character_is_name(label) OR target=actor THEN result_code:=10;
   ELSIF target IS NULL THEN result_code:=2;
   ELSE
    SELECT entry_no INTO existing FROM game_vip_entries WHERE account_id=account AND world_id=world AND character_id=target;
    IF EXISTS(SELECT 1 FROM game_vip_consents WHERE account_id=account AND world_id=world AND character_id=target AND inviter_character_id=actor AND (consent=2 OR invite_expires_at>clock_timestamp())) THEN
     result_code:=3;
    ELSIF existing IS NULL AND (SELECT count(*) FROM game_vip_entries WHERE account_id=account AND world_id=world)>=p_cap THEN result_code:=4;
    ELSE
     IF existing IS NULL THEN
      existing:=list_row.next_entry_no; list_row.next_entry_no:=existing+1;
      INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(account,world,target,existing);
     END IF;
     result_number:=existing;
     DELETE FROM game_vip_consents WHERE account_id=account AND world_id=world AND character_id=target AND inviter_character_id=actor AND consent=1 AND invite_expires_at<=clock_timestamp();
     IF NOT EXISTS(SELECT 1 FROM game_character_social_blocks WHERE (character_id=target AND blocked_character_id=actor) OR (character_id=actor AND blocked_character_id=target))
       AND (SELECT count(*) FROM game_vip_consents WHERE character_id=target AND consent=1 AND invite_expires_at>clock_timestamp())<20 THEN
      INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) VALUES(account,world,actor,target,1,clock_timestamp()+interval '7 days');
      changed:=TRUE;
     END IF;
    END IF;
   END IF;
  WHEN 2 THEN
   IF target IS NULL THEN result_code:=7;
   ELSE
    changed:=EXISTS(SELECT 1 FROM game_vip_consents WHERE account_id=account AND world_id=world AND character_id=target);
    DELETE FROM game_vip_entries WHERE account_id=account AND world_id=world AND character_id=target;
   END IF;
  WHEN 3 THEN
   IF target IS NULL THEN result_code:=7;
   ELSIF jsonb_array_length(p_request->'groups')>8 OR (SELECT count(*)<>count(DISTINCT x) FROM jsonb_array_elements_text(p_request->'groups') x) OR char_length(label)>128 OR octet_length(label)>512 OR label ~ '[[:cntrl:]]' OR (p_request->>'icon')::integer NOT BETWEEN 0 AND 10 THEN result_code:=10;
   ELSIF EXISTS(SELECT 1 FROM jsonb_array_elements_text(p_request->'groups') x WHERE x::integer NOT BETWEEN 1 AND 8 OR (x::integer>3 AND NOT EXISTS(SELECT 1 FROM game_vip_groups WHERE account_id=account AND world_id=world AND group_no=x::integer))) THEN result_code:=7;
   ELSE
    UPDATE game_vip_entries SET description=label,icon=(p_request->>'icon')::smallint,notify=(p_request->>'notify')::boolean WHERE account_id=account AND world_id=world AND character_id=target;
    DELETE FROM game_vip_group_members WHERE account_id=account AND world_id=world AND character_id=target;
    INSERT INTO game_vip_group_members SELECT account,world,target,x::smallint FROM jsonb_array_elements_text(p_request->'groups') x;
   END IF;
  WHEN 4,5 THEN
   IF char_length(label) NOT BETWEEN 1 AND 25 OR octet_length(label)>100 OR label ~ '[[:cntrl:]]' THEN result_code:=10;
   ELSIF op=5 AND n NOT BETWEEN 4 AND 8 THEN result_code:=10;
   ELSIF op=5 AND NOT EXISTS(SELECT 1 FROM game_vip_groups WHERE account_id=account AND world_id=world AND group_no=n) THEN result_code:=7;
   ELSIF (SELECT count(*) FROM game_vip_groups WHERE account_id=account AND world_id=world)>p_groups OR (op=4 AND (SELECT count(*) FROM game_vip_groups WHERE account_id=account AND world_id=world)>=p_groups) OR (SELECT count(*) FROM game_vip_entries WHERE account_id=account AND world_id=world)>p_cap THEN result_code:=5;
   ELSIF EXISTS(SELECT 1 FROM game_vip_groups WHERE account_id=account AND world_id=world AND lower(name)=lower(label) AND (op=4 OR group_no<>n)) THEN result_code:=6;
   ELSE
    IF op=4 THEN SELECT min(x) INTO g FROM generate_series(4,8) x WHERE NOT EXISTS(SELECT 1 FROM game_vip_groups WHERE account_id=account AND world_id=world AND group_no=x);
     INSERT INTO game_vip_groups VALUES(account,world,g,label); result_number:=g;
    ELSE UPDATE game_vip_groups SET name=label WHERE account_id=account AND world_id=world AND group_no=n; result_number:=n;
    END IF;
   END IF;
  WHEN 6 THEN
   IF n NOT BETWEEN 4 AND 8 THEN result_code:=10;
   ELSE DELETE FROM game_vip_groups WHERE account_id=account AND world_id=world AND group_no=n;
    IF NOT FOUND THEN result_code:=7;
    ELSE DELETE FROM game_vip_group_members WHERE account_id=account AND world_id=world AND group_no=n; END IF;
   END IF;
  WHEN 7,8,9 THEN
   IF watcher IS NULL OR (consent_row.consent=1 AND consent_row.invite_expires_at<=clock_timestamp()) OR
    (op IN(7,8) AND consent_row.consent<>1) OR (op=9 AND consent_row.consent<>2) THEN result_code:=7;
   ELSIF op=7 AND (SELECT count(*) FROM game_vip_consents WHERE character_id=actor AND consent=2)>=100 THEN result_code:=4;
   ELSE
    IF op=7 THEN UPDATE game_vip_consents SET consent=2 WHERE invitation_id=consent_row.invitation_id;
    ELSE DELETE FROM game_vip_consents WHERE invitation_id=consent_row.invitation_id; END IF;
    changed:=TRUE;
    IF watcher<>account THEN UPDATE game_vip_lists SET revision=revision+1 WHERE account_id=watcher AND world_id=world; END IF;
   END IF;
  WHEN 10 THEN
   IF target IS NULL THEN result_code:=7;
   ELSE DELETE FROM game_vip_consents WHERE account_id=account AND world_id=world AND character_id=target AND inviter_character_id=actor AND (consent=2 OR invite_expires_at>clock_timestamp());
    IF NOT FOUND THEN result_code:=7; ELSE changed:=TRUE; END IF;
   END IF;
  WHEN 11 THEN
   IF n NOT IN(1,2) THEN result_code:=10;
   ELSE target:=actor; changed:=TRUE; END IF;
  ELSE result_code:=10;
  END CASE;
  END IF;
 END IF;
 IF changed THEN
  INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy,vip_status,vip_consent_revision)
   VALUES(target,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1',CASE WHEN op=11 THEN n ELSE 1 END,1)
   ON CONFLICT(character_id) DO UPDATE SET vip_consent_revision=game_character_social_settings.vip_consent_revision+1,
    vip_status=CASE WHEN op=11 THEN n ELSE game_character_social_settings.vip_status END;
 END IF;
 UPDATE game_vip_lists SET revision=revision+1,next_entry_no=list_row.next_entry_no,rate_tokens=list_row.rate_tokens,rate_at=list_row.rate_at WHERE account_id=account AND world_id=world RETURNING revision INTO existing;
 view:=jsonb_build_object('code',result_code,'revision',existing,'number',result_number,'wait',seconds);
 INSERT INTO game_vip_command_receipts VALUES(account,world,session,p_command,actor,operation_binding,view,clock_timestamp());
 RETURN view;
END $$;

DO $$
DECLARE operation TEXT; operation_no INTEGER:=0; signature TEXT;
BEGIN
 FOREACH operation IN ARRAY ARRAY['add','remove','edit','group_add','group_rename','group_remove','accept','reject','revoke','withdraw','status','query'] LOOP
  operation_no:=operation_no+1;
  EXECUTE format('CREATE FUNCTION game_vip_%I(r JSONB,f JSONB,c NUMERIC,q JSONB,cap INTEGER,groups INTEGER) RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $fn$ BEGIN IF (q->>''op'')::integer <> %s THEN RAISE EXCEPTION ''wrong VIP operation'' USING ERRCODE=''23514''; END IF; RETURN game_vip_execute(r,f,c,q,cap,groups); END $fn$',operation,operation_no);
  signature:=format('game_vip_%I(jsonb,jsonb,numeric,jsonb,integer,integer)',operation);
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_vip_execute(jsonb,jsonb,numeric,jsonb,integer,integer) SET search_path = %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_vip_immutable() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_vip_execute(jsonb,jsonb,numeric,jsonb,integer,integer),game_vip_immutable() FROM PUBLIC;
REVOKE ALL ON game_vip_lists,game_vip_entries,game_vip_groups,game_vip_group_members,game_vip_consents,game_vip_handles,game_vip_command_receipts FROM PUBLIC;
REVOKE ALL ON SEQUENCE game_vip_invitation_id,game_vip_handles_handle_seq FROM PUBLIC;
GRANT SELECT ON game_vip_lists,game_vip_entries,game_vip_groups,game_vip_group_members,game_vip_consents,game_vip_handles,game_vip_command_receipts TO oteryn_game_runtime;

-- Preserve PARTY-1's existing column writes without granting direct VIP privacy writes.
REVOKE INSERT,UPDATE,DELETE ON game_character_social_settings FROM oteryn_game_runtime;
GRANT INSERT(character_id,party_invites,channel_visibility,source_policy),
 UPDATE(party_invites,channel_visibility) ON game_character_social_settings TO oteryn_game_runtime;

-- PARTY-1's existing block producer also terminates VIP consent in the same transaction.
-- Acquire the VIP owner before the new block's FK checks. No shared PARTY writer changes.
CREATE FUNCTION game_vip_on_block() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE world UUID; affected RECORD;
BEGIN
 SELECT world_id INTO world FROM game_character_roots WHERE character_id=NEW.character_id;
 PERFORM pg_advisory_xact_lock(163,hashtext(world::text));
 PERFORM 1 FROM game_character_roots WHERE character_id IN(NEW.character_id,NEW.blocked_character_id) ORDER BY character_id FOR UPDATE;
 FOR affected IN SELECT l.account_id,l.world_id FROM game_vip_lists l
  WHERE EXISTS(SELECT 1 FROM game_vip_consents c WHERE c.account_id=l.account_id AND c.world_id=l.world_id
   AND ((c.character_id=NEW.character_id AND c.inviter_character_id=NEW.blocked_character_id)
     OR (c.character_id=NEW.blocked_character_id AND c.inviter_character_id=NEW.character_id)))
  ORDER BY l.account_id,l.world_id FOR UPDATE
 LOOP
  UPDATE game_vip_lists SET revision=revision+1 WHERE account_id=affected.account_id AND world_id=affected.world_id;
 END LOOP;
 FOR affected IN SELECT DISTINCT character_id FROM game_vip_consents
  WHERE (character_id=NEW.character_id AND inviter_character_id=NEW.blocked_character_id)
    OR (character_id=NEW.blocked_character_id AND inviter_character_id=NEW.character_id)
 LOOP
  UPDATE game_character_social_settings SET vip_consent_revision=vip_consent_revision+1 WHERE character_id=affected.character_id;
 END LOOP;
 DELETE FROM game_vip_consents
  WHERE (character_id=NEW.character_id AND inviter_character_id=NEW.blocked_character_id)
    OR (character_id=NEW.blocked_character_id AND inviter_character_id=NEW.character_id);
 RETURN NEW;
END $$;
CREATE TRIGGER game_vip_block_ends_consent BEFORE INSERT ON game_character_social_blocks
 FOR EACH ROW EXECUTE FUNCTION game_vip_on_block();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_vip_on_block() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_vip_on_block() FROM PUBLIC;
