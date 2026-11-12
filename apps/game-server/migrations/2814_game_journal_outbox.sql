-- Journal outbox writers follow the stream, logout and baseline schema.
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
