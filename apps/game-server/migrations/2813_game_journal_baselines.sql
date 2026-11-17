-- An initial stream needs an explicitly accepted baseline. Installing this
-- migration accepts none. Runtime cannot manufacture or replace an approval.
CREATE TABLE game_journal_items_baselines (
 stream_key BYTEA PRIMARY KEY REFERENCES game_journal_streams(stream_key),
 approval_ref TEXT NOT NULL CHECK(approval_ref ~ '^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$'),
 accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON game_journal_items_baselines FROM PUBLIC;
GRANT SELECT ON game_journal_items_baselines TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_journal_items_baselines TO oteryn_game_control;

-- One live character/family cut per source generation. Preserve native IDs;
-- these indexed columns are derived only from the canonical stream binding.
ALTER TABLE game_journal_streams ADD COLUMN character_id UUID REFERENCES game_character_roots(character_id);
ALTER TABLE game_journal_streams ADD COLUMN family TEXT;
UPDATE game_journal_streams SET
 character_id=(convert_from(stream_key,'UTF8')::jsonb->>'character_id')::uuid,
 family=convert_from(stream_key,'UTF8')::jsonb->>'family';
ALTER TABLE game_journal_streams ALTER COLUMN family SET NOT NULL;
ALTER TABLE game_journal_streams ADD CHECK(family IN('ITEMS','BANK','MARKET','PLAYER_TRADE'));
ALTER TABLE game_journal_streams ADD CHECK((family='BANK')=(character_id IS NULL));
CREATE INDEX game_journal_stream_character ON game_journal_streams(character_id) WHERE character_id IS NOT NULL;
CREATE UNIQUE INDEX game_journal_active_character_cut ON game_journal_streams(source_authority,character_id,family,recovery_generation) WHERE active AND character_id IS NOT NULL;
CREATE FUNCTION game_journal_stream_binding() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE k JSONB;
BEGIN
 k:=convert_from(NEW.stream_key,'UTF8')::jsonb;
 NEW.character_id:=(k->>'character_id')::uuid;
 NEW.family:=k->>'family';
 -- Match native ownership/capture lock order: Character, generation, stream.
 IF NEW.character_id IS NOT NULL THEN
  PERFORM 1 FROM public.game_character_roots WHERE character_id=NEW.character_id FOR KEY SHARE;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_stream_binding() FROM PUBLIC;
CREATE TRIGGER game_journal_00_stream_binding BEFORE INSERT ON game_journal_streams
 FOR EACH ROW EXECUTE FUNCTION game_journal_stream_binding();

-- Ownership/World cutoff fences queued exports in the same native transaction.
CREATE FUNCTION game_journal_owner_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 UPDATE public.game_journal_streams SET active=false,privacy_fenced_at=coalesce(privacy_fenced_at,clock_timestamp()) WHERE character_id=NEW.character_id;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_owner_fence() FROM PUBLIC;
CREATE TRIGGER game_journal_owner_fence AFTER UPDATE OF account_id,world_id ON game_character_roots
 FOR EACH ROW WHEN(OLD.account_id IS DISTINCT FROM NEW.account_id OR OLD.world_id IS DISTINCT FROM NEW.world_id)
 EXECUTE FUNCTION game_journal_owner_fence();

-- Runtime has read authority, not write authority, on these private projections.
-- Restrict definer privileges to the exact boolean/counter/row-lock operations.
CREATE FUNCTION game_journal_projection_epoch() RETURNS TEXT
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT projection_epoch::text FROM public.game_character_account_projection_epoch WHERE epoch_scope=1 FOR SHARE
$$;
CREATE FUNCTION game_journal_account_projection(p_account UUID) RETURNS TEXT
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT projection_revision::text FROM public.game_character_account_projections WHERE account_id=p_account FOR SHARE
$$;
CREATE FUNCTION game_journal_unknown_house(p_character UUID) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT EXISTS(SELECT 1 FROM public.game_house_ownership WHERE owner_character_id=p_character)
 OR EXISTS(SELECT 1 FROM public.game_item_house_reclaim_provenance WHERE reclaim_subject_character_id=p_character)
$$;
REVOKE ALL ON FUNCTION game_journal_projection_epoch(),game_journal_account_projection(UUID),game_journal_unknown_house(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_projection_epoch(),game_journal_account_projection(UUID),game_journal_unknown_house(UUID) TO oteryn_game_runtime;

-- Use the native account-closure authorities, including detached members.
CREATE FUNCTION game_journal_deleted(p_account UUID,p_character UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT public.game_account_closure_closed(p_account)
 OR (p_character IS NOT NULL AND public.game_account_closure_detached(p_character))
$$;
REVOKE ALL ON FUNCTION game_journal_deleted(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_deleted(UUID,UUID) TO oteryn_game_runtime;
CREATE FUNCTION game_journal_closure_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 UPDATE public.game_journal_streams s SET active=false,privacy_fenced_at=coalesce(privacy_fenced_at,clock_timestamp())
 WHERE (s.account_id=NEW.account_id OR (TG_TABLE_NAME='game_account_closure_members' AND s.character_id=(to_jsonb(NEW)->>'character_id')::uuid))
 AND ((TG_TABLE_NAME='game_account_closures' AND to_jsonb(NEW)->>'state'<>'CANCELLED') OR public.game_journal_deleted(s.account_id,s.character_id));
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_closure_fence() FROM PUBLIC;
CREATE TRIGGER game_journal_account_closed AFTER INSERT OR UPDATE OF state ON game_account_closures
 FOR EACH ROW EXECUTE FUNCTION game_journal_closure_fence();
CREATE TRIGGER game_journal_member_detached AFTER INSERT OR UPDATE OF step ON game_account_closure_members
 FOR EACH ROW EXECUTE FUNCTION game_journal_closure_fence();

-- Ordinary expiry advances the floor without closing the stream. Privacy
-- fences erase all affected payloads; tombstones have their own indexed expiry.
CREATE FUNCTION game_journal_purge() RETURNS INTEGER
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE n INTEGER;
BEGIN
 WITH candidates AS MATERIALIZED (
  SELECT s.stream_key FROM public.game_journal_streams s
  WHERE EXISTS(SELECT 1 FROM public.game_journal_outbox o WHERE o.stream_key=s.stream_key AND o.export_bytes IS NOT NULL
   AND (o.retain_until<=clock_timestamp() OR NOT s.active OR public.game_journal_deleted(s.account_id,s.character_id)
    OR NOT EXISTS(SELECT 1 FROM public.game_journal_export_settings c WHERE c.singleton AND c.game_journal_items_export_enabled AND c.source_authority=s.source_authority)))
  ORDER BY s.stream_key LIMIT 256 FOR UPDATE SKIP LOCKED
 ), expired AS (
  SELECT o.stream_key,o.source_revision,
   CASE WHEN s.active AND NOT public.game_journal_deleted(s.account_id,s.character_id)
    AND EXISTS(SELECT 1 FROM public.game_journal_export_settings c WHERE c.singleton AND c.game_journal_items_export_enabled AND c.source_authority=s.source_authority)
   THEN 'EXPIRY' ELSE 'DELETION' END AS reason
  FROM public.game_journal_outbox o JOIN public.game_journal_streams s USING(stream_key) JOIN candidates c USING(stream_key)
  WHERE o.export_bytes IS NOT NULL AND (o.retain_until<=clock_timestamp() OR NOT s.active OR public.game_journal_deleted(s.account_id,s.character_id)
   OR NOT EXISTS(SELECT 1 FROM public.game_journal_export_settings c WHERE c.singleton AND c.game_journal_items_export_enabled AND c.source_authority=s.source_authority))
  ORDER BY o.retain_until LIMIT 256 FOR UPDATE OF o SKIP LOCKED
 ), erased AS (
  UPDATE public.game_journal_outbox o SET export_bytes=NULL,erased_at=statement_timestamp(),erasure_reason=e.reason,
   tombstone_until=statement_timestamp()+CASE WHEN e.reason='EXPIRY' THEN interval '31 days' ELSE interval '120 days' END
  FROM expired e WHERE (o.stream_key,o.source_revision)=(e.stream_key,e.source_revision)
  RETURNING o.stream_key,o.source_revision
 ), floors AS (
  UPDATE public.game_journal_streams s SET retained_floor=greatest(s.retained_floor,e.floor),
   active=CASE WHEN public.game_journal_deleted(s.account_id,s.character_id) THEN false ELSE s.active END
  FROM (SELECT stream_key,max(source_revision) AS floor FROM erased GROUP BY stream_key) e
  WHERE s.stream_key=e.stream_key RETURNING s.stream_key
 ) SELECT count(*) INTO n FROM erased;
 -- A later deletion extends existing ordinary tombstones without resurrecting
 -- any payload. Promotion and removal are independently bounded.
 WITH promote AS (
  SELECT o.stream_key,o.source_revision,s.privacy_fenced_at FROM public.game_journal_outbox o
  JOIN public.game_journal_streams s USING(stream_key)
  WHERE o.erased_at IS NOT NULL AND s.privacy_fenced_at IS NOT NULL AND o.tombstone_until<s.privacy_fenced_at+interval '120 days'
  ORDER BY o.tombstone_until LIMIT 256 FOR UPDATE OF o SKIP LOCKED
 ) UPDATE public.game_journal_outbox o SET erasure_reason='DELETION',tombstone_until=p.privacy_fenced_at+interval '120 days'
 FROM promote p WHERE (o.stream_key,o.source_revision)=(p.stream_key,p.source_revision);
 WITH expired AS (
  SELECT o.stream_key,o.source_revision FROM public.game_journal_outbox o JOIN public.game_journal_streams s USING(stream_key)
  WHERE o.erased_at IS NOT NULL AND o.tombstone_until<=clock_timestamp() AND (s.privacy_fenced_at IS NULL OR o.tombstone_until>=s.privacy_fenced_at+interval '120 days')
  ORDER BY o.tombstone_until LIMIT 256 FOR UPDATE OF o SKIP LOCKED
 ) DELETE FROM public.game_journal_outbox o USING expired e WHERE (o.stream_key,o.source_revision)=(e.stream_key,e.source_revision);
 RETURN n;
END $$;
REVOKE ALL ON FUNCTION game_journal_purge() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_purge() TO oteryn_game_runtime;

CREATE FUNCTION game_journal_claim(p_source TEXT,p_limit INTEGER) RETURNS TABLE(stream_key BYTEA,retry_attempt BIGINT,claim_token UUID)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 64 THEN RAISE EXCEPTION 'journal claim bound' USING ERRCODE='23514'; END IF;
 RETURN QUERY WITH due AS (
  SELECT s.stream_key FROM public.game_journal_streams s WHERE s.active AND NOT s.quarantined AND s.family='ITEMS' AND s.source_authority=p_source
   AND s.next_delivery_at<=clock_timestamp() AND (s.claim_until IS NULL OR s.claim_until<=clock_timestamp())
   AND NOT public.game_journal_deleted(s.account_id,s.character_id)
   AND EXISTS(SELECT 1 FROM public.game_journal_outbox o WHERE o.stream_key=s.stream_key)
  ORDER BY s.next_delivery_at,s.stream_key LIMIT p_limit FOR UPDATE SKIP LOCKED
 ) UPDATE public.game_journal_streams s SET claim_token=uuidv7(),claim_until=clock_timestamp()+interval '30 seconds'
 FROM due d WHERE s.stream_key=d.stream_key RETURNING s.stream_key,s.retry_attempt,s.claim_token;
END $$;
REVOKE ALL ON FUNCTION game_journal_claim(TEXT,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_claim(TEXT,INTEGER) TO oteryn_game_runtime;
CREATE FUNCTION game_journal_schedule(p_stream BYTEA,p_token UUID,p_attempt BIGINT,p_delay DOUBLE PRECISION,p_progress BOOLEAN,p_quarantine BOOLEAN) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE n INTEGER;
BEGIN
 UPDATE public.game_journal_streams SET retry_attempt=p_attempt,
 next_delivery_at=greatest(next_delivery_at,CASE WHEN p_delay>8000000000000 THEN 'infinity'::timestamptz ELSE clock_timestamp()+make_interval(secs=>greatest(0,p_delay)) END),
 quarantined=quarantined OR p_quarantine,claim_token=NULL,claim_until=NULL,
 last_progress_at=CASE WHEN p_progress THEN clock_timestamp() ELSE last_progress_at END
 WHERE stream_key=p_stream AND claim_token=p_token AND claim_until>clock_timestamp() AND p_attempt>=0;
 GET DIAGNOSTICS n=ROW_COUNT;RETURN n=1;
END $$;
REVOKE ALL ON FUNCTION game_journal_schedule(BYTEA,UUID,BIGINT,DOUBLE PRECISION,BOOLEAN,BOOLEAN) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_schedule(BYTEA,UUID,BIGINT,DOUBLE PRECISION,BOOLEAN,BOOLEAN) TO oteryn_game_runtime;
CREATE FUNCTION game_journal_release_claim(p_stream BYTEA,p_token UUID) RETURNS VOID
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 UPDATE public.game_journal_streams SET claim_token=NULL,claim_until=NULL WHERE stream_key=p_stream AND claim_token=p_token
$$;
REVOKE ALL ON FUNCTION game_journal_release_claim(BYTEA,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_release_claim(BYTEA,UUID) TO oteryn_game_runtime;
-- Explicit control-plane repair only. Preserve every outstanding Retry-After.
CREATE FUNCTION game_journal_repair(p_stream BYTEA) RETURNS VOID
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 UPDATE public.game_journal_streams SET quarantined=false WHERE stream_key=p_stream AND active AND claim_token IS NULL
$$;
REVOKE ALL ON FUNCTION game_journal_repair(BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_repair(BYTEA) TO oteryn_game_control;

CREATE FUNCTION game_journal_stalled() RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE n INTEGER;
BEGIN
 UPDATE public.game_journal_streams s SET last_diagnostic_at=clock_timestamp()
 WHERE s.stream_key IN(SELECT stream_key FROM public.game_journal_streams x WHERE active
  AND last_progress_at<=clock_timestamp()-interval '60 seconds'
  AND (last_diagnostic_at IS NULL OR last_diagnostic_at<=clock_timestamp()-interval '60 seconds')
  AND (x.retry_attempt>0 OR EXISTS(SELECT 1 FROM public.game_journal_outbox o WHERE o.stream_key=x.stream_key AND o.acknowledged_at IS NULL AND o.export_bytes IS NOT NULL))
  ORDER BY stream_key LIMIT 256);
 GET DIAGNOSTICS n=ROW_COUNT;RETURN n>0;
END $$;
REVOKE ALL ON FUNCTION game_journal_stalled() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_journal_stalled() TO oteryn_game_runtime;
