-- Journal foundation precedes the logout and baseline foreign keys.
-- Private journal export evidence. Registration is a control-plane operation after
-- verifying the authoritative ownership/baseline cut; enqueue never invents one.
CREATE TABLE game_journal_streams (
 stream_key BYTEA PRIMARY KEY CHECK(octet_length(stream_key) BETWEEN 1 AND 1024),
 source_authority TEXT NOT NULL CHECK(octet_length(source_authority) BETWEEN 1 AND 128),
 account_id UUID NOT NULL,
 recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
 retained_floor NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(retained_floor>=0),
 claim_token UUID,
 claim_until TIMESTAMPTZ,
 quarantined BOOLEAN NOT NULL DEFAULT false,
 privacy_fenced_at TIMESTAMPTZ,
 next_delivery_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 retry_attempt BIGINT NOT NULL DEFAULT 0 CHECK(retry_attempt>=0),
 last_progress_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 last_diagnostic_at TIMESTAMPTZ,
 next_revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(next_revision BETWEEN 1 AND 18446744073709551616),
 acknowledged_revision NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(acknowledged_revision>=0 AND acknowledged_revision<next_revision),
 active BOOLEAN NOT NULL DEFAULT true,
 UNIQUE(stream_key,source_authority,recovery_generation)
);
CREATE INDEX game_journal_stream_account ON game_journal_streams(source_authority,account_id,recovery_generation);
CREATE TABLE game_journal_outbox (
 stream_key BYTEA NOT NULL,
 source_authority TEXT NOT NULL,
 recovery_generation NUMERIC(20,0) NOT NULL,
 source_revision NUMERIC(20,0) NOT NULL CHECK(source_revision BETWEEN 1 AND 18446744073709551615),
 record_id UUID NOT NULL,
 export_bytes BYTEA CHECK(octet_length(export_bytes) BETWEEN 1 AND 65536),
 export_sha256 BYTEA NOT NULL CHECK(octet_length(export_sha256)=32),
 payload_expires_at TIMESTAMPTZ NOT NULL,
 retain_until TIMESTAMPTZ NOT NULL,
 erased_at TIMESTAMPTZ,
 erasure_reason TEXT CHECK(erasure_reason IN('EXPIRY','DELETION')),
 tombstone_until TIMESTAMPTZ,
 CHECK((erased_at IS NULL)=(erasure_reason IS NULL) AND (erased_at IS NULL)=(tombstone_until IS NULL)),
 CHECK((export_bytes IS NULL)=(erased_at IS NOT NULL)),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 acknowledged_at TIMESTAMPTZ,
 PRIMARY KEY(stream_key,source_revision),
 UNIQUE(source_authority,recovery_generation,record_id),
 FOREIGN KEY(stream_key,source_authority,recovery_generation)
   REFERENCES game_journal_streams(stream_key,source_authority,recovery_generation)
);
CREATE INDEX game_journal_expiry ON game_journal_outbox(retain_until) WHERE export_bytes IS NOT NULL;
CREATE INDEX game_journal_tombstone_expiry ON game_journal_outbox(tombstone_until) WHERE erased_at IS NOT NULL;
CREATE INDEX game_journal_due ON game_journal_streams(next_delivery_at) WHERE active AND NOT quarantined;
-- The primary key covers stream FK lookups, ordered replay and bounded publication.
REVOKE ALL ON game_journal_streams,game_journal_outbox FROM PUBLIC;
GRANT SELECT ON game_journal_streams,game_journal_outbox TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_journal_streams TO oteryn_game_control;
GRANT UPDATE(active) ON game_journal_streams TO oteryn_game_control;
GRANT SELECT ON game_journal_outbox TO oteryn_game_control;

CREATE TABLE game_journal_generations (
 source_authority TEXT NOT NULL,
 account_id UUID NOT NULL,
 recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(source_authority,account_id)
);
REVOKE ALL ON game_journal_generations FROM PUBLIC;
GRANT SELECT ON game_journal_generations TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_journal_register_generation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE generation NUMERIC(20,0); k JSONB;
BEGIN
 IF NEW.next_revision<>1 OR NEW.acknowledged_revision<>0 OR NOT NEW.active THEN
  RAISE EXCEPTION 'journal registration requires initial cursor' USING ERRCODE='23514';
 END IF;
 k:=convert_from(NEW.stream_key,'UTF8')::jsonb;
 IF k->>'source_authority' IS DISTINCT FROM NEW.source_authority
 OR (k->>'account_id')::uuid IS DISTINCT FROM NEW.account_id
 OR (k->>'recovery_generation')::numeric IS DISTINCT FROM NEW.recovery_generation THEN
  RAISE EXCEPTION 'journal registration binding mismatch' USING ERRCODE='23514';
 END IF;
 INSERT INTO public.game_journal_generations VALUES(NEW.source_authority,NEW.account_id,NEW.recovery_generation)
 ON CONFLICT DO NOTHING;
 SELECT recovery_generation INTO STRICT generation FROM public.game_journal_generations
 WHERE source_authority=NEW.source_authority AND account_id=NEW.account_id FOR UPDATE;
 IF NEW.recovery_generation<generation THEN RAISE EXCEPTION 'journal old generation' USING ERRCODE='23514'; END IF;
 IF NEW.recovery_generation>generation THEN
  UPDATE public.game_journal_generations SET recovery_generation=NEW.recovery_generation
  WHERE source_authority=NEW.source_authority AND account_id=NEW.account_id;
  UPDATE public.game_journal_streams SET active=false
  WHERE source_authority=NEW.source_authority AND account_id=NEW.account_id AND recovery_generation<NEW.recovery_generation;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_register_generation() FROM PUBLIC;
CREATE TRIGGER game_journal_generation_registration BEFORE INSERT ON game_journal_streams
 FOR EACH ROW EXECUTE FUNCTION game_journal_register_generation();

-- No producer or peer is enabled by installing this migration.
CREATE TABLE game_journal_export_settings (
 singleton BOOLEAN PRIMARY KEY DEFAULT true CHECK(singleton),
 game_journal_items_export_enabled BOOLEAN NOT NULL DEFAULT false,
 source_authority TEXT CHECK(source_authority ~ '^[A-Za-z0-9._:/-]{1,128}$')
);
INSERT INTO game_journal_export_settings(singleton) VALUES(true);
CREATE TABLE game_journal_logout_cuts (
 game_session_id UUID PRIMARY KEY REFERENCES game_durability_reconnect_sessions(game_session_id),
 stream_key BYTEA NOT NULL REFERENCES game_journal_streams(stream_key),
 snapshot_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(snapshot_id)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 character_revision_at_logout NUMERIC(20,0) NOT NULL CHECK(character_revision_at_logout>=1),
 first_revision NUMERIC(20,0) NOT NULL CHECK(first_revision>=1),
 record_count INTEGER NOT NULL CHECK(record_count BETWEEN 2 AND 16386)
);
CREATE INDEX game_journal_logout_cut_stream ON game_journal_logout_cuts(stream_key);
REVOKE ALL ON game_journal_export_settings,game_journal_logout_cuts FROM PUBLIC;
GRANT SELECT ON game_journal_export_settings,game_journal_logout_cuts TO oteryn_game_runtime;
GRANT INSERT ON game_journal_logout_cuts TO oteryn_game_runtime;
GRANT SELECT,UPDATE ON game_journal_export_settings TO oteryn_game_control;
GRANT SELECT ON game_journal_logout_cuts TO oteryn_game_control;
CREATE FUNCTION game_journal_lock_stream(p_stream BYTEA) RETURNS TEXT
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s public.game_journal_streams%ROWTYPE;
BEGIN
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream;
 PERFORM 1 FROM public.game_journal_generations WHERE source_authority=s.source_authority
 AND account_id=s.account_id AND recovery_generation=s.recovery_generation FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'journal old generation' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream FOR UPDATE;
 IF NOT s.active THEN RAISE EXCEPTION 'journal stream fenced' USING ERRCODE='23514'; END IF;
 RETURN s.next_revision::text;
END $$;
REVOKE ALL ON FUNCTION game_journal_lock_stream(BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_lock_stream(BYTEA) TO oteryn_game_runtime;
CREATE FUNCTION game_journal_reconcile(p_stream BYTEA,p_after NUMERIC,p_id UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE next NUMERIC(20,0);
BEGIN
 next:=public.game_journal_lock_stream(p_stream)::numeric;
 IF p_after IS NULL OR p_after<0 OR p_after>=next OR p_after<>trunc(p_after)
 OR p_after<(SELECT retained_floor FROM public.game_journal_streams WHERE stream_key=p_stream)
 OR (p_after=0 AND p_id IS NOT NULL)
 OR (p_after>0 AND NOT EXISTS(SELECT 1 FROM public.game_journal_outbox
       WHERE stream_key=p_stream AND source_revision=p_after AND record_id=p_id)) THEN
  RAISE EXCEPTION 'journal cursor lacks retained evidence' USING ERRCODE='23514';
 END IF;
 UPDATE public.game_journal_outbox SET retain_until=CASE WHEN acknowledged_at IS NULL THEN statement_timestamp()+interval '30 days' ELSE retain_until END,acknowledged_at=coalesce(acknowledged_at,statement_timestamp())
 WHERE stream_key=p_stream AND source_revision<=p_after AND acknowledged_at IS NULL;
 UPDATE public.game_journal_streams SET acknowledged_revision=greatest(acknowledged_revision,p_after)
 WHERE stream_key=p_stream;
END $$;
REVOKE ALL ON FUNCTION game_journal_reconcile(BYTEA,NUMERIC,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_reconcile(BYTEA,NUMERIC,UUID) TO oteryn_game_runtime;
