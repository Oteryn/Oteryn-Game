-- Private journal export evidence. Registration is a control-plane operation after
-- verifying the authoritative ownership/baseline cut; enqueue never invents one.
CREATE TABLE game_journal_streams (
 stream_key BYTEA PRIMARY KEY CHECK(octet_length(stream_key) BETWEEN 1 AND 1024),
 source_authority TEXT NOT NULL CHECK(octet_length(source_authority) BETWEEN 1 AND 128),
 account_id UUID NOT NULL,
 recovery_generation NUMERIC(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
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
 export_bytes BYTEA NOT NULL CHECK(octet_length(export_bytes) BETWEEN 1 AND 65536),
 export_sha256 BYTEA GENERATED ALWAYS AS (sha256(export_bytes)) STORED,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 acknowledged_at TIMESTAMPTZ,
 PRIMARY KEY(stream_key,source_revision),
 UNIQUE(source_authority,recovery_generation,record_id),
 FOREIGN KEY(stream_key,source_authority,recovery_generation)
   REFERENCES game_journal_streams(stream_key,source_authority,recovery_generation)
);
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

CREATE FUNCTION game_journal_enqueue(p_stream BYTEA,p_bytes BYTEA) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s public.game_journal_streams%ROWTYPE; previous BYTEA; b JSONB; k JSONB;
 n NUMERIC(20,0); rid UUID;
BEGIN
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream;
 PERFORM 1 FROM public.game_journal_generations WHERE source_authority=s.source_authority
 AND account_id=s.account_id AND recovery_generation=s.recovery_generation FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'journal old generation' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream FOR UPDATE;
 IF NOT s.active THEN RAISE EXCEPTION 'journal stream fenced' USING ERRCODE='23514'; END IF;
 b:=convert_from(p_bytes,'UTF8')::jsonb;
 k:=convert_from(p_stream,'UTF8')::jsonb;
 IF b->>'operation'<>'PublishGameJournalRecordV1' OR k->>'operation'<>'ReadGameJournalCursorV1'
 OR b->>'source_authority' IS DISTINCT FROM s.source_authority
 OR (b->>'account_id')::uuid IS DISTINCT FROM s.account_id
 OR (b->>'recovery_generation')::numeric IS DISTINCT FROM s.recovery_generation
 OR (b - ARRAY['operation','source_revision','record_id','occurred_at','coverage_start_at','kind','payload'])
    IS DISTINCT FROM (k - 'operation') THEN
  RAISE EXCEPTION 'journal stream binding mismatch' USING ERRCODE='23514';
 END IF;
 n:=(b->>'source_revision')::numeric; rid:=(b->>'record_id')::uuid;
 SELECT export_bytes INTO previous FROM public.game_journal_outbox WHERE stream_key=p_stream AND source_revision=n;
 IF FOUND THEN
  IF previous<>p_bytes THEN RAISE EXCEPTION 'journal conflicting replay' USING ERRCODE='23514'; END IF;
  RETURN false;
 END IF;
 IF n IS DISTINCT FROM s.next_revision THEN
  RAISE EXCEPTION 'journal noncontiguous revision' USING ERRCODE='23514';
 END IF;
 INSERT INTO public.game_journal_outbox(stream_key,source_authority,recovery_generation,source_revision,record_id,export_bytes)
 VALUES(p_stream,s.source_authority,s.recovery_generation,n,rid,p_bytes);
 UPDATE public.game_journal_streams SET next_revision=n+1 WHERE stream_key=p_stream;
 RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_journal_enqueue(BYTEA,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_enqueue(BYTEA,BYTEA) TO oteryn_game_runtime;

-- ACKs are applied only after the caller verifies an exact contract response.
-- Contiguous ACKs and fixed IDs prevent stale or unrelated responses deleting work.
CREATE FUNCTION game_journal_ack(p_stream BYTEA,p_revision NUMERIC,p_id UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s public.game_journal_streams%ROWTYPE;
BEGIN
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream;
 PERFORM 1 FROM public.game_journal_generations WHERE source_authority=s.source_authority
 AND account_id=s.account_id AND recovery_generation=s.recovery_generation FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'journal old generation' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT s FROM public.game_journal_streams WHERE stream_key=p_stream FOR UPDATE;
 IF NOT s.active THEN RAISE EXCEPTION 'journal stream fenced' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM public.game_journal_outbox WHERE stream_key=p_stream AND source_revision=p_revision AND record_id=p_id) THEN
  RAISE EXCEPTION 'journal unknown acknowledgement' USING ERRCODE='23514';
 END IF;
 IF p_revision<=s.acknowledged_revision THEN RETURN false; END IF;
 IF p_revision<>s.acknowledged_revision+1 THEN RAISE EXCEPTION 'journal acknowledgement gap' USING ERRCODE='23514'; END IF;
 UPDATE public.game_journal_outbox SET acknowledged_at=clock_timestamp() WHERE stream_key=p_stream AND source_revision=p_revision;
 UPDATE public.game_journal_streams SET acknowledged_revision=p_revision WHERE stream_key=p_stream;
 RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_journal_ack(BYTEA,NUMERIC,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_ack(BYTEA,NUMERIC,UUID) TO oteryn_game_runtime;
