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
 OR (p_after=0 AND p_id IS NOT NULL)
 OR (p_after>0 AND NOT EXISTS(SELECT 1 FROM public.game_journal_outbox
       WHERE stream_key=p_stream AND source_revision=p_after AND record_id=p_id)) THEN
  RAISE EXCEPTION 'journal cursor lacks retained evidence' USING ERRCODE='23514';
 END IF;
 UPDATE public.game_journal_outbox SET acknowledged_at=coalesce(acknowledged_at,clock_timestamp())
 WHERE stream_key=p_stream AND source_revision<=p_after AND acknowledged_at IS NULL;
 UPDATE public.game_journal_streams SET acknowledged_revision=greatest(acknowledged_revision,p_after)
 WHERE stream_key=p_stream;
END $$;
REVOKE ALL ON FUNCTION game_journal_reconcile(BYTEA,NUMERIC,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_reconcile(BYTEA,NUMERIC,UUID) TO oteryn_game_runtime;
