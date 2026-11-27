-- Global Server Save begins at 10:00 Europe/Berlin. Convert the local date
-- plus its civil time to an instant so adjacent saves may be 23 or 25 hours apart.
CREATE FUNCTION game_server_save_at(save_day DATE, legacy_six_am BOOLEAN DEFAULT FALSE)
RETURNS TIMESTAMPTZ LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT (save_day + CASE WHEN legacy_six_am THEN TIME '06:00' ELSE TIME '10:00' END)
        AT TIME ZONE 'Europe/Berlin'
$$;

CREATE FUNCTION game_server_save_day(at_time TIMESTAMPTZ, legacy_six_am BOOLEAN DEFAULT FALSE)
RETURNS DATE LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT civil_day - (at_time < game_server_save_at(civil_day, legacy_six_am))::integer
    FROM (SELECT (at_time AT TIME ZONE 'Europe/Berlin')::date AS civil_day) berlin
$$;

-- Do not schedule a second save on the cutover day of an already-published
-- legacy epoch. In the normal case the civil date of the current clock wins.
CREATE FUNCTION game_server_save_next(at_time TIMESTAMPTZ, published_day DATE)
RETURNS TIMESTAMPTZ LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT game_server_save_at(GREATEST(game_server_save_day(at_time),published_day)+1)
$$;

-- Preserve the already-authored annual event windows of committed legacy epochs.
CREATE FUNCTION game_server_save_event_day(at_time TIMESTAMPTZ, legacy_six_am BOOLEAN)
RETURNS DATE LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT CASE WHEN legacy_six_am THEN (at_time AT TIME ZONE 'Europe/Berlin')::date
                ELSE game_server_save_day(at_time) END
$$;

-- Keep the persisted calendar identifier accepted by existing control clients.
-- The publication flag distinguishes historical 06:00 labels from new 10:00
-- labels. Existing epochs, their daily claim FKs, and task-board receipts are
-- immutable. Their labels retain their original meaning.
ALTER TABLE game_world_reset_epochs
    ADD COLUMN legacy_six_am BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE game_world_reset_epochs ALTER COLUMN legacy_six_am SET DEFAULT FALSE;

ALTER TABLE game_world_reset_history
    ADD COLUMN legacy_six_am BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE game_world_reset_history ALTER COLUMN legacy_six_am SET DEFAULT FALSE;
ALTER TABLE game_world_reset_history
    DROP CONSTRAINT game_world_reset_history_check1,
    ADD CONSTRAINT game_world_reset_history_check1
    CHECK (reset_day = game_server_save_day(to_timestamp(reset_at_unix_ms / 1000.0), legacy_six_am)
                       - DATE '1970-01-01');

CREATE FUNCTION game_world_reset_server_save_policy() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    save_day DATE := game_server_save_day(NEW.activated_at);
    previous_day DATE;
BEGIN
    SELECT game_server_save_day(e.activated_at, TRUE) INTO previous_day
    FROM game_world_reset_epochs e
    WHERE e.world_id=NEW.world_id AND e.reset_epoch=NEW.reset_epoch-1 AND e.legacy_six_am;

    -- An operational publication between 06:00 and 10:00 on cutover day can
    -- follow an already-issued legacy day. Preserve that day through the gap.
    -- Once a 10:00 label is published there is no legacy predecessor to carry.
    NEW.legacy_six_am := COALESCE(previous_day > save_day
        AND previous_day = game_server_save_day(NEW.activated_at, TRUE), FALSE);
    NEW.reset_month := to_char(game_server_save_day(NEW.activated_at, NEW.legacy_six_am), 'YYYY-MM');
    RETURN NEW;
END $$;
CREATE TRIGGER world_reset_server_save_policy BEFORE INSERT ON game_world_reset_epochs
    FOR EACH ROW EXECUTE FUNCTION game_world_reset_server_save_policy();

-- The preceding epoch policy trigger supplies reset_month to both owner paths.
-- Replace the old calculation without duplicating their publication fences.
DO $migration$
DECLARE
    signature REGPROCEDURE;
    definition TEXT;
    previous TEXT := $old$to_char((instant AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM')$old$;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_world_reset_initialize(uuid,bytea,text,bytea,bytea,bytea,text,numeric)'::regprocedure,
        'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure
    ] LOOP
        SELECT pg_get_functiondef(signature) INTO definition;
        IF strpos(definition, previous)=0 THEN
            RAISE EXCEPTION 'unexpected World reset publication body: %', signature;
        END IF;
        EXECUTE replace(definition, previous, 'NULL');
    END LOOP;
END $migration$;

CREATE OR REPLACE FUNCTION game_daily_reward_reset_window() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE instant BIGINT;
BEGIN
    IF NEW.reset_epoch=18446744073709551615 THEN RETURN NEW; END IF;
    instant:=floor(extract(epoch FROM NEW.activated_at)*1000)::bigint;
    INSERT INTO public.game_world_reset_history
        (world_id,reset_epoch,reset_day,reset_at_unix_ms,committed_at_unix_ms,legacy_six_am)
    VALUES (NEW.world_id,NEW.reset_epoch+1,
        game_server_save_day(NEW.activated_at, NEW.legacy_six_am)-DATE '1970-01-01',
        instant,instant,NEW.legacy_six_am);
    RETURN NEW;
END $$;

-- Week IDs remain the civil Monday's signed day offset from 1970-01-01.
-- E.g. the stored 20780 remains 20780 and is re-used at the new boundary.
-- Holding the active row keeps board loads and their receipt guard serialized
-- against reset publication, including the one-time compatibility window.
CREATE OR REPLACE FUNCTION game_task_board_period(p_world UUID) RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE epoch NUMERIC; save_day DATE;
BEGIN
    SELECT reset_epoch INTO epoch FROM game_world_reset_active WHERE world_id=p_world FOR SHARE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT game_server_save_day(e.activated_at, e.legacy_six_am) INTO save_day
    FROM game_world_reset_epochs e WHERE e.world_id=p_world AND e.reset_epoch=epoch;
    RETURN (date_trunc('week', save_day::timestamp)::date-DATE '1970-01-01')::bigint;
END $$;

CREATE OR REPLACE FUNCTION game_task_board_reset_day(p_world UUID) RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT game_server_save_day(e.activated_at, e.legacy_six_am)
    FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
    WHERE a.world_id=p_world
$$;

-- Cake's annual custody and continuation checks use the same date as the
-- seasonal World event projection. Preserve the settled guards and grants.
DO $migration$
DECLARE
    signature REGPROCEDURE;
    definition TEXT;
    previous TEXT := $old$r.activated_at AT TIME ZONE 'Europe/Berlin'$old$;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_cake_lure_admit()'::regprocedure,
        'game_cake_donation_record()'::regprocedure,
        'game_cake_reset_continuation(uuid,numeric)'::regprocedure
    ] LOOP
        SELECT pg_get_functiondef(signature) INTO definition;
        IF strpos(definition, previous)=0 THEN
            RAISE EXCEPTION 'unexpected Cake reset calendar body: %', signature;
        END IF;
        EXECUTE replace(definition, previous, 'game_server_save_event_day(r.activated_at,r.legacy_six_am)');
    END LOOP;
END $migration$;

-- CREATE OR REPLACE retains existing ownership and grants. Pin search paths on
-- the new helpers and the existing board functions after replacing their bodies.
DO $migration$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_server_save_at(date,boolean)', 'game_server_save_day(timestamptz,boolean)',
        'game_server_save_next(timestamptz,date)', 'game_server_save_event_day(timestamptz,boolean)',
        'game_world_reset_server_save_policy()', 'game_task_board_period(uuid)',
        'game_task_board_reset_day(uuid)'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
    END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_server_save_at(DATE,BOOLEAN), game_server_save_day(TIMESTAMPTZ,BOOLEAN),
    game_server_save_next(TIMESTAMPTZ,DATE), game_server_save_event_day(TIMESTAMPTZ,BOOLEAN),
    game_world_reset_server_save_policy() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_server_save_at(DATE,BOOLEAN), game_server_save_day(TIMESTAMPTZ,BOOLEAN),
    game_server_save_next(TIMESTAMPTZ,DATE), game_server_save_event_day(TIMESTAMPTZ,BOOLEAN)
    TO oteryn_game_runtime,oteryn_game_control;
