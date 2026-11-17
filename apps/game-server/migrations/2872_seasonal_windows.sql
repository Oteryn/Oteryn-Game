-- TibiaWiki Annual Autumn Vintage, Annual Autumn Vintage/Spoiler,
-- Demon's Lullaby and Demon's Lullaby/Spoiler (fetched 2026-11-17).
-- Owner accepted slice: durable progress API; participation quests are separate.
CREATE TABLE game_seasonwin_rules (
    event text NOT NULL,
    slot smallint NOT NULL,
    start_month integer NOT NULL,
    start_day integer NOT NULL,
    end_month integer NOT NULL,
    end_day integer NOT NULL,
    threshold bigint NOT NULL CHECK(threshold>0),
    xp_percent integer NOT NULL,
    potion_percent integer NOT NULL,
    reward_seconds bigint,
    reward_end_month integer,
    reward_end_day integer,
    PRIMARY KEY(event,slot)
);
INSERT INTO game_seasonwin_rules VALUES
 ('autumn_vintage',1,10,1,10,8,4000,100,133,604800,NULL,NULL),
 ('autumn_vintage',2,10,17,10,24,4000,100,133,604800,NULL,NULL),
 ('demons_lullaby',1,5,7,5,14,500,150,100,NULL,5,21);

CREATE TABLE game_seasonwin_rounds (
    world_id uuid NOT NULL,
    event text NOT NULL,
    event_year integer NOT NULL,
    slot smallint NOT NULL,
    opens_at timestamptz NOT NULL,
    closes_at timestamptz NOT NULL CHECK(closes_at>opens_at),
    threshold bigint NOT NULL CHECK(threshold>0),
    progress bigint NOT NULL DEFAULT 0 CHECK(progress>=0 AND progress<=threshold),
    succeeded_at timestamptz,
    expires_at timestamptz,
    xp_percent integer NOT NULL CHECK(xp_percent>=100),
    potion_percent integer NOT NULL CHECK(potion_percent>=100),
    reward_seconds bigint,
    fixed_reward_end timestamptz,
    PRIMARY KEY(world_id,event,event_year,slot),
    FOREIGN KEY(event,slot) REFERENCES game_seasonwin_rules(event,slot),
    CHECK ((succeeded_at IS NULL AND expires_at IS NULL AND progress<threshold)
        OR (succeeded_at IS NOT NULL AND expires_at IS NOT NULL AND succeeded_at>=opens_at
            AND succeeded_at<closes_at AND expires_at>succeeded_at AND progress=threshold))
);
CREATE INDEX game_seasonwin_rounds_rule ON game_seasonwin_rounds(event,slot);
CREATE INDEX game_seasonwin_rounds_buffs ON game_seasonwin_rounds(world_id,expires_at) WHERE succeeded_at IS NOT NULL;
CREATE TABLE game_seasonwin_receipts (
    world_id uuid NOT NULL,
    receipt_id uuid NOT NULL,
    event text NOT NULL,
    event_year integer NOT NULL,
    slot smallint NOT NULL,
    amount bigint NOT NULL CHECK(amount>0),
    progress bigint NOT NULL,
    succeeded_at timestamptz,
    expires_at timestamptz,
    PRIMARY KEY(world_id,receipt_id),
    FOREIGN KEY(world_id,event,event_year,slot) REFERENCES game_seasonwin_rounds(world_id,event,event_year,slot)
);
CREATE INDEX game_seasonwin_receipts_round ON game_seasonwin_receipts(world_id,event,event_year,slot);
CREATE FUNCTION game_seasonwin_now() RETURNS timestamptz LANGUAGE sql STABLE AS $$ SELECT statement_timestamp() $$;

-- Capture only through the real save transaction, never from a progress request.
CREATE FUNCTION game_seasonwin_capture(p_world uuid,p_epoch numeric) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE at_time timestamptz; y integer;
BEGIN
    SELECT activated_at INTO STRICT at_time FROM game_world_reset_epochs
      WHERE world_id=p_world AND reset_epoch=p_epoch;
    y := extract(year FROM at_time AT TIME ZONE 'Europe/Berlin');
    INSERT INTO game_seasonwin_rounds(world_id,event,event_year,slot,opens_at,closes_at,
        threshold,xp_percent,potion_percent,reward_seconds,fixed_reward_end)
    SELECT p_world,r.event,y,r.slot,w.opens,w.closes,r.threshold,r.xp_percent,r.potion_percent,r.reward_seconds,
        CASE WHEN r.reward_end_month IS NOT NULL THEN
            (make_date(y,r.reward_end_month,r.reward_end_day)+time '06:00') AT TIME ZONE 'Europe/Berlin' END
    FROM game_seasonwin_rules r CROSS JOIN LATERAL (
        SELECT (make_date(y,r.start_month,r.start_day)+time '06:00') AT TIME ZONE 'Europe/Berlin' AS opens,
        (make_date(y+CASE WHEN (r.end_month,r.end_day)<(r.start_month,r.start_day) THEN 1 ELSE 0 END,r.end_month,r.end_day)+time '06:00') AT TIME ZONE 'Europe/Berlin' AS closes
    ) w WHERE at_time>=w.opens AND at_time<w.closes
    ON CONFLICT DO NOTHING;
END $$;

-- Receipt and round locks serialize retries and threshold crossings across Channels.
CREATE FUNCTION game_seasonwin_record(p_world uuid,p_epoch numeric,p_event text,p_amount bigint,p_receipt uuid)
RETURNS TABLE(progress bigint,succeeded boolean,expires_ms bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r game_seasonwin_rounds; old game_seasonwin_receipts; at_time timestamptz;
BEGIN
    IF p_amount<=0 OR p_event NOT IN ('autumn_vintage','demons_lullaby') THEN
        RAISE EXCEPTION 'invalid seasonal progress' USING ERRCODE='22023';
    END IF;
    PERFORM 1 FROM game_world_reset_active WHERE world_id=p_world AND reset_epoch=p_epoch FOR SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'stale seasonal epoch' USING ERRCODE='23514'; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('seasonwin:'||p_world::text||':'||p_receipt::text,0));
    SELECT * INTO old FROM game_seasonwin_receipts WHERE world_id=p_world AND receipt_id=p_receipt;
    IF FOUND THEN
        IF old.event<>p_event OR old.amount<>p_amount THEN
            RAISE EXCEPTION 'conflicting seasonal receipt' USING ERRCODE='23514';
        END IF;
        RETURN QUERY SELECT old.progress,old.succeeded_at IS NOT NULL,(extract(epoch FROM old.expires_at)*1000)::bigint;
        RETURN;
    END IF;
    at_time := game_seasonwin_now();
    SELECT s.* INTO r FROM game_seasonwin_rounds s JOIN game_world_reset_epochs e ON e.world_id=s.world_id AND e.reset_epoch=p_epoch
      WHERE s.world_id=p_world AND s.event=p_event AND e.activated_at>=s.opens_at AND e.activated_at<s.closes_at
        AND at_time>=s.opens_at AND at_time<s.closes_at
      ORDER BY s.event_year DESC,s.slot DESC LIMIT 1 FOR UPDATE OF s;
    IF NOT FOUND THEN RETURN; END IF;
    r.progress := r.progress+least(p_amount,r.threshold-r.progress);
    IF r.progress=r.threshold AND r.succeeded_at IS NULL THEN
        r.succeeded_at := at_time;
        r.expires_at := coalesce(r.fixed_reward_end,at_time+make_interval(secs=>r.reward_seconds::double precision));
    END IF;
    UPDATE game_seasonwin_rounds s SET progress=r.progress,succeeded_at=r.succeeded_at,expires_at=r.expires_at
      WHERE s.world_id=r.world_id AND s.event=r.event AND s.event_year=r.event_year AND s.slot=r.slot;
    INSERT INTO game_seasonwin_receipts VALUES(p_world,p_receipt,p_event,r.event_year,r.slot,p_amount,r.progress,r.succeeded_at,r.expires_at);
    RETURN QUERY SELECT r.progress,r.succeeded_at IS NOT NULL,(extract(epoch FROM r.expires_at)*1000)::bigint;
END $$;

REVOKE ALL ON game_seasonwin_rules,game_seasonwin_rounds,game_seasonwin_receipts FROM PUBLIC;
GRANT SELECT ON game_seasonwin_rules,game_seasonwin_rounds,game_seasonwin_receipts TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_seasonwin_capture(uuid,numeric),game_seasonwin_record(uuid,numeric,text,bigint,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_seasonwin_capture(uuid,numeric),game_seasonwin_record(uuid,numeric,text,bigint,uuid) TO oteryn_game_runtime;
