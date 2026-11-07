CREATE OR REPLACE FUNCTION game_cake_world_complete(p_world UUID,p_event BIGINT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_cake_world_node_context%ROWTYPE; w game_seasonal_cake_worlds%ROWTYPE;
 old JSONB; expected JSONB; subst BIGINT; extracted BIGINT; bitten BIGINT; receipts BIGINT;
 stg INTEGER; current_layer INTEGER; progress INTEGER; start_at BIGINT; end_at BIGINT; bonus BIGINT;
 restore BOOLEAN:=FALSE; side game_cake_world_side_context%ROWTYPE; r game_seasonal_cake_bite_receipts%ROWTYPE;
 prior JSONB; after JSONB; expected_bites INTEGER;
 polls BIGINT; openings BIGINT; bridged BOOLEAN; new_window BIGINT; new_layer INTEGER;
BEGIN
 IF game_cake_world_context_is_current(p_world,p_event) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Cake completion lacks its exact current Node context' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT c FROM game_cake_world_node_context WHERE backend_pid=pg_backend_pid();
 IF c.validated THEN RETURN; END IF;
 SELECT * INTO STRICT w FROM game_seasonal_cake_worlds WHERE world_id=p_world AND event_start=p_event;
 old:=CASE WHEN c.captured THEN c.initial_state ELSE w.state END;
 SELECT coalesce(sum((result->>'accepted')::bigint),0),count(*) INTO subst,receipts
  FROM game_seasonal_cake_deliveries WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id();
 SELECT count(*) FILTER(WHERE counted) INTO extracted FROM game_seasonal_cake_extractions
  WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id();
 SELECT count(*) INTO bitten FROM game_seasonal_cake_bite_receipts
  WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id();
 SELECT count(*),coalesce(bool_or(opened_bridge),FALSE) INTO polls,bridged FROM game_seasonal_cake_stage_polls
  WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id();
 SELECT count(*),max(window_start),max(layer) INTO openings,new_window,new_layer FROM game_seasonal_cake_windows
  WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id();
 IF polls>1 OR openings>1 OR (polls>0 AND (bitten>0 OR receipts>0 OR extracted>0))
  OR (openings>0 AND (polls>0 OR bitten>0 OR receipts>0 OR extracted>0))
  OR receipts>1 OR extracted>1 OR bitten>1
  OR (SELECT count(*) FROM game_seasonal_cake_extractions WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id())>1
  OR ((subst>0 OR extracted>0 OR bitten>0 OR openings>0 OR bridged) AND NOT c.captured)
  OR (c.inserted AND (subst<>0 OR extracted<>0 OR bitten<>0 OR openings<>0 OR polls<>0)) THEN
  RAISE EXCEPTION 'Cake progress requires one physical source action and its World write' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_seasonal_cake_extractions e WHERE e.world_id=p_world AND e.event_start=p_event
  AND e.created_xact_id=pg_current_xact_id() AND e.counted IS DISTINCT FROM
   ((old->>'stage')::integer=1 AND (old->>'golems')::integer<200 AND c.now_seconds<w.event_end)) THEN
  RAISE EXCEPTION 'Cake extraction count disagrees with exact source stage' USING ERRCODE='23514'; END IF;
 IF polls>0 AND (old->>'stage')::integer<>1 THEN
  RAISE EXCEPTION 'Cake stage poll requires its stage-one predecessor' USING ERRCODE='23514'; END IF;
 IF NOT c.captured THEN
  IF EXISTS(SELECT 1 FROM game_cake_world_side_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Cake marks require their same-transaction World transition' USING ERRCODE='23514'; END IF;
  UPDATE game_cake_world_node_context SET validated=TRUE WHERE backend_pid=pg_backend_pid(); RETURN;
 END IF;
 expected:=old;
 stg:=(old->>'stage')::integer; current_layer:=(old->>'layer')::integer; progress:=(old->>'layer_progress')::integer;
 start_at:=(old->>'window_start')::bigint; end_at:=(old->>'window_end')::bigint;
 IF c.inserted THEN
  IF w.state<>old THEN RAISE EXCEPTION 'Cake insertion cannot carry progress' USING ERRCODE='23514'; END IF;
 ELSIF c.now_seconds>=w.event_end THEN
  IF subst<>0 OR extracted<>0 OR bitten<>0 OR openings<>0 OR polls<>0 THEN RAISE EXCEPTION 'ended Cake occurrence cannot earn progress' USING ERRCODE='23514'; END IF;
  expected:=expected||jsonb_build_object('stage',0,'window_start',NULL,'window_end',NULL);
 ELSIF stg=1 THEN
  IF bitten<>0 OR openings<>0 OR (old->>'substances')::bigint+subst>200 OR (old->>'golems')::bigint+extracted>200 THEN
   RAISE EXCEPTION 'Cake source counters exceed their stage or goals' USING ERRCODE='23514'; END IF;
  expected:=expected||jsonb_build_object('substances',(old->>'substances')::bigint+subst,'golems',(old->>'golems')::bigint+extracted);
  IF bridged THEN
   IF (old->>'substances')::integer<>200 OR (old->>'golems')::integer<>200 THEN
    RAISE EXCEPTION 'Cake bridge poll cannot anticipate a later delivery' USING ERRCODE='23514'; END IF;
   expected:=expected||jsonb_build_object('stage',2); END IF;
 ELSIF stg=2 THEN
  IF subst<>0 OR extracted<>0 THEN RAISE EXCEPTION 'Cake harvesting counters are closed' USING ERRCODE='23514'; END IF;
  IF bitten=1 THEN
   IF progress>=game_cake_edible_goal() OR start_at IS NULL OR end_at IS NULL OR c.now_seconds<start_at OR c.now_seconds>=end_at OR openings<>0 THEN
    RAISE EXCEPTION 'Cake bite requires its active fixed eating window' USING ERRCODE='23514'; END IF;
   expected:=expected||jsonb_build_object('layer_progress',progress+1);
  ELSE
   IF end_at IS NOT NULL AND (c.now_seconds>=end_at OR progress>=game_cake_edible_goal()) THEN
    restore:=progress<game_cake_edible_goal() OR current_layer<4;
    expected:=expected||jsonb_build_object('window_start',NULL,'window_end',NULL);
    IF progress>=game_cake_edible_goal() THEN
     IF current_layer=4 THEN
      bonus:=floor(extract(epoch FROM ((date_trunc('day',to_timestamp(c.now_seconds::double precision) AT TIME ZONE 'Europe/Berlin'-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin')))::bigint;
      expected:=expected||jsonb_build_object('stage',3,'bonus_from',bonus,'bonus_until',bonus+604800);
     ELSE expected:=expected||jsonb_build_object('layer',current_layer+1,'layer_progress',0); END IF;
    ELSE expected:=expected||jsonb_build_object('layer_progress',0); END IF;
   END IF;
   IF openings=1 THEN
    IF (expected->>'stage')::integer<>2 OR expected->>'window_start' IS NOT NULL
     OR new_layer<>(expected->>'layer')::integer THEN
     RAISE EXCEPTION 'Cake clock cannot replace a live or completed final round' USING ERRCODE='23514'; END IF;
    expected:=expected||jsonb_build_object('window_start',new_window,'window_end',new_window+1800);
   END IF;
  END IF;
 ELSE
  IF subst<>0 OR extracted<>0 OR bitten<>0 OR openings<>0 OR polls<>0 THEN RAISE EXCEPTION 'Cake progress is closed' USING ERRCODE='23514'; END IF;
 END IF;
 IF NOT c.inserted THEN
  IF expected-ARRAY['revision']=old-ARRAY['revision'] THEN
   RAISE EXCEPTION 'Cake World revision requires a real source transition' USING ERRCODE='23514'; END IF;
  expected:=expected||jsonb_build_object('revision',(old->>'revision')::numeric+1);
 END IF;
 IF w.state IS DISTINCT FROM expected THEN
  RAISE EXCEPTION 'Cake World final state does not conserve exact source receipts and deadlines' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_seasonal_cake_deliveries d WHERE d.world_id=p_world AND d.event_start=p_event
  AND d.created_xact_id=pg_current_xact_id() AND (d.result->'substances' IS DISTINCT FROM expected->'substances'
   OR d.result->'golems' IS DISTINCT FROM expected->'golems' OR d.result->'stage' IS DISTINCT FROM expected->'stage')) THEN
  RAISE EXCEPTION 'Cake delivery result differs from committed World counters' USING ERRCODE='23514'; END IF;
 FOR r IN SELECT * FROM game_seasonal_cake_bite_receipts WHERE world_id=p_world AND event_start=p_event AND created_xact_id=pg_current_xact_id() LOOP
  IF stg<>2 OR r.layer<>current_layer OR r.window_start IS DISTINCT FROM (old->>'window_start')::bigint
   OR r.world_revision_before<>(old->>'revision')::numeric OR r.world_revision_after<>(expected->>'revision')::numeric
   OR NOT EXISTS(SELECT 1 FROM game_cake_world_side_context s WHERE s.backend_pid=pg_backend_pid()
    AND s.transaction_xid=pg_current_xact_id() AND s.kind=1 AND s.identity=r.character_id::text)
   OR NOT EXISTS(SELECT 1 FROM game_cake_world_side_context s WHERE s.backend_pid=pg_backend_pid()
    AND s.transaction_xid=pg_current_xact_id() AND s.kind=2 AND s.identity=r.layer||':'||r.x||':'||r.y||':'||r.floor) THEN
   RAISE EXCEPTION 'Cake bite lacks its exact World/personal/physical marks' USING ERRCODE='23514'; END IF;
 END LOOP;
 FOR side IN SELECT * FROM game_cake_world_side_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() LOOP
  prior:=side.before_state; after:=side.after_state;
  IF (after->>'world_id')::uuid<>p_world OR (after->>'event_start')::bigint<>p_event THEN
   RAISE EXCEPTION 'Cake side mark names another occurrence' USING ERRCODE='23514'; END IF;
  IF side.kind=1 THEN
   SELECT * INTO r FROM game_seasonal_cake_bite_receipts b WHERE b.world_id=p_world AND b.event_start=p_event
    AND b.created_xact_id=pg_current_xact_id() AND b.character_id=(after->>'character_id')::uuid;
   expected_bites:=CASE WHEN prior IS NOT NULL AND (prior->>'window_start')::bigint=r.window_start THEN (prior->>'bites')::integer+1 ELSE 1 END;
   IF r.game_session_id IS NULL OR (after->>'window_start')::bigint<>r.window_start
    OR (after->>'last_bite')::bigint<>c.now_seconds OR (after->>'bites')::integer<>expected_bites OR expected_bites>12
    OR NOT EXISTS(SELECT 1 FROM game_seasonal_cake_bites b WHERE b.character_id=r.character_id
     AND b.world_id=p_world AND b.event_start=p_event AND to_jsonb(b)=after) THEN
    RAISE EXCEPTION 'Cake personal mark lacks its actual predecessor and food-backed bite' USING ERRCODE='23514'; END IF;
  ELSIF (after->>'removed')::boolean THEN
   SELECT * INTO r FROM game_seasonal_cake_bite_receipts b WHERE b.world_id=p_world AND b.event_start=p_event
    AND b.created_xact_id=pg_current_xact_id() AND b.layer=(after->>'layer')::smallint
    AND b.x=(after->>'x')::integer AND b.y=(after->>'y')::integer AND b.floor=(after->>'floor')::smallint;
   IF r.game_session_id IS NULL OR coalesce((prior->>'removed')::boolean,FALSE)
    OR (after->>'window_start')::bigint<>r.window_start OR (after->>'appearance')::integer<>r.appearance THEN
    RAISE EXCEPTION 'Cake removed square lacks its unique physical bite' USING ERRCODE='23514'; END IF;
  ELSE
   IF NOT restore OR prior IS NULL OR (prior->>'removed')::boolean IS DISTINCT FROM TRUE
    OR (after->>'layer')::integer<>current_layer
    OR (after-ARRAY['removed','last_xact_id']) IS DISTINCT FROM (prior-ARRAY['removed','last_xact_id']) THEN
    RAISE EXCEPTION 'Cake restore may touch only current failed/nonfinal completed layer' USING ERRCODE='23514'; END IF;
  END IF;
  IF side.kind=2 AND NOT EXISTS(SELECT 1 FROM game_seasonal_cake_tiles t WHERE t.world_id=p_world AND t.event_start=p_event
   AND t.layer=(after->>'layer')::smallint AND t.x=(after->>'x')::integer AND t.y=(after->>'y')::integer
   AND t.floor=(after->>'floor')::smallint AND to_jsonb(t)=after) THEN
   RAISE EXCEPTION 'Cake physical mark differs from its captured write' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF restore AND EXISTS(SELECT 1 FROM game_seasonal_cake_tiles WHERE world_id=p_world AND event_start=p_event AND game_seasonal_cake_tiles.layer=current_layer AND removed) THEN
  RAISE EXCEPTION 'Cake round restore must restore its entire current layer' USING ERRCODE='23514'; END IF;
 UPDATE game_cake_world_node_context SET validated=TRUE WHERE backend_pid=pg_backend_pid();
END $$;
