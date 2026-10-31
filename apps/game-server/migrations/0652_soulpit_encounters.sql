-- W07 fight state is subordinate to the shared activity handoff owner (#2769).
-- It grants no Instance/session admission or return authority. Entry's existing CRAFT
-- receipt, this fight, and the shared group's COMMITTED decision must commit together.
-- Crystal 201189f soulpit_fight.lua:37-154: one core, four waves, survivors' Animus.
-- Forward rollback: stop new entries; retain handoff, item, event and mastery evidence.
CREATE TABLE game_soulpit_encounters (
    handoff_id UUID PRIMARY KEY REFERENCES game_activity_handoff_groups(handoff_id),
    species_key TEXT NOT NULL CHECK (species_key ~ '^oteryn:creature\.[a-z0-9_]+$'
        AND octet_length(species_key)<=160),
    source_transaction_id UUID NOT NULL UNIQUE
        REFERENCES game_item_craft_records(transaction_id) DEFERRABLE INITIALLY DEFERRED,
    started_at_micros BIGINT NOT NULL CHECK (started_at_micros>=0),
    revision BIGINT NOT NULL CHECK (revision>=0),
    checkpoint JSONB NOT NULL CHECK (jsonb_typeof(checkpoint)='object'),
    terminal BOOLEAN NOT NULL,
    last_occurrence UUID CHECK (game_character_is_uuid_v7(last_occurrence)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    updated_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK ((revision=0)=(last_occurrence IS NULL))
);

CREATE TABLE game_soulpit_participants (
    handoff_id UUID NOT NULL REFERENCES game_soulpit_encounters(handoff_id),
    character_id UUID NOT NULL,
    staging_slot SMALLINT NOT NULL CHECK (staging_slot BETWEEN 0 AND 4),
    PRIMARY KEY (handoff_id,character_id),
    UNIQUE (handoff_id,staging_slot),
    FOREIGN KEY (handoff_id,character_id)
        REFERENCES game_activity_handoff_participants(handoff_id,character_id)
);

-- A receipt of the shared owner's decision, not another decision/authority. A historical
-- COMMITTED group cannot be used later to admit a new debit or a replacement fight.
CREATE TABLE game_soulpit_entry_handoff_commits (
    handoff_id UUID PRIMARY KEY REFERENCES game_soulpit_encounters(handoff_id)
        DEFERRABLE INITIALLY DEFERRED,
    committed_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_soulpit_capture_shared_commit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_soulpit_entry_handoff_commits(handoff_id) VALUES(NEW.handoff_id);
    RETURN NULL;
END $$;
CREATE TRIGGER game_soulpit_capture_shared_commit AFTER UPDATE ON game_activity_handoff_groups
    FOR EACH ROW WHEN (OLD.state=1 AND NEW.state=2 AND NEW.activity_key='oteryn:activity.soulpit')
    EXECUTE FUNCTION game_soulpit_capture_shared_commit();

CREATE TABLE game_soulpit_events (
    handoff_id UUID NOT NULL REFERENCES game_soulpit_encounters(handoff_id),
    occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(occurrence_id)),
    revision BIGINT NOT NULL CHECK (revision>0),
    intent JSONB NOT NULL CHECK (jsonb_typeof(intent)='object'),
    checkpoint JSONB NOT NULL CHECK (jsonb_typeof(checkpoint)='object'),
    effect JSONB NOT NULL CHECK (jsonb_typeof(effect) IN ('string','object')),
    terminal BOOLEAN NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (handoff_id,occurrence_id),
    UNIQUE (handoff_id,revision)
);

CREATE TABLE game_soulpit_survival_rewards (
    handoff_id UUID NOT NULL,
    character_id UUID NOT NULL,
    species_key TEXT NOT NULL,
    occurrence_id UUID NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (handoff_id,character_id),
    FOREIGN KEY (handoff_id,character_id)
        REFERENCES game_soulpit_participants(handoff_id,character_id),
    FOREIGN KEY (handoff_id,occurrence_id)
        REFERENCES game_soulpit_events(handoff_id,occurrence_id),
    FOREIGN KEY (character_id,species_key)
        REFERENCES game_character_animus_masteries(character_id,species_key)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE FUNCTION game_soulpit_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN
        IF NEW.revision<>0 OR NEW.terminal OR NEW.last_occurrence IS NOT NULL
            OR NOT EXISTS (SELECT 1 FROM game_activity_handoff_groups g
                WHERE g.handoff_id=NEW.handoff_id AND g.state IN (1,2)
                    AND g.participant_count BETWEEN 1 AND 5)
        THEN RAISE EXCEPTION 'Soulpit entry requires shared group provenance' USING ERRCODE='23514'; END IF;
        NEW.created_xact_id:=pg_current_xact_id();
        NEW.updated_xact_id:=pg_current_xact_id();
        RETURN NEW;
    ELSIF TG_OP='UPDATE' THEN
        IF OLD.terminal OR NEW.revision<>OLD.revision+1
            OR (to_jsonb(NEW)-ARRAY['revision','checkpoint','terminal','last_occurrence','updated_xact_id'])
                IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['revision','checkpoint','terminal','last_occurrence','updated_xact_id'])
            OR NOT EXISTS (SELECT 1 FROM game_soulpit_events e
                WHERE e.handoff_id=NEW.handoff_id AND e.occurrence_id=NEW.last_occurrence
                    AND e.revision=NEW.revision AND e.checkpoint=NEW.checkpoint
                    AND e.terminal=NEW.terminal AND e.created_xact_id=pg_current_xact_id())
        THEN RAISE EXCEPTION 'Soulpit update requires one exact event; terminal state is final' USING ERRCODE='23514'; END IF;
        NEW.updated_xact_id:=pg_current_xact_id();
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Soulpit history cannot be deleted' USING ERRCODE='23514';
END $$;

CREATE FUNCTION game_soulpit_entry_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fight game_soulpit_encounters%ROWTYPE; expected_count INT;
BEGIN
    SELECT * INTO STRICT fight FROM game_soulpit_encounters WHERE handoff_id=NEW.handoff_id;
    SELECT participant_count INTO expected_count FROM game_activity_handoff_groups
        WHERE handoff_id=fight.handoff_id AND state=2;
    IF expected_count IS NULL OR expected_count NOT BETWEEN 1 AND 5
        OR NOT EXISTS (SELECT 1 FROM game_soulpit_entry_handoff_commits c
            WHERE c.handoff_id=fight.handoff_id AND c.committed_xact_id=fight.created_xact_id)
        OR (SELECT count(*) FROM game_soulpit_participants WHERE handoff_id=fight.handoff_id)<>expected_count
        OR NOT EXISTS (SELECT 1 FROM game_item_craft_records r
            JOIN game_activity_handoff_participants p ON p.handoff_id=fight.handoff_id
                AND p.character_id=r.character_id AND p.source_game_session_id=r.game_session_id
            WHERE r.transaction_id=fight.source_transaction_id AND r.recipe='oteryn:soulpit.entry'
                AND r.created_xact_id=fight.created_xact_id
                AND r.world_id=p.world_id AND jsonb_array_length(r.outputs)=0)
    THEN RAISE EXCEPTION 'Soulpit core debit and committed shared handoff must be atomic' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_soulpit_event_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id()
        OR NOT EXISTS (SELECT 1 FROM game_soulpit_encounters f
            WHERE f.handoff_id=NEW.handoff_id AND f.revision=NEW.revision
                AND f.last_occurrence=NEW.occurrence_id AND f.checkpoint=NEW.checkpoint
                AND f.terminal=NEW.terminal AND f.updated_xact_id=pg_current_xact_id())
    THEN RAISE EXCEPTION 'Soulpit event and projection must commit together' USING ERRCODE='23514'; END IF;
    IF NEW.effect ? 'Completed' AND (
        jsonb_typeof(NEW.effect->'Completed'->'surviving_slots') IS DISTINCT FROM 'array'
        OR jsonb_array_length(NEW.effect->'Completed'->'surviving_slots') NOT BETWEEN 1 AND 5
        OR (SELECT count(*) FROM game_soulpit_survival_rewards r
            WHERE r.handoff_id=NEW.handoff_id AND r.occurrence_id=NEW.occurrence_id)
            <>jsonb_array_length(NEW.effect->'Completed'->'surviving_slots'))
    THEN RAISE EXCEPTION 'every Soulpit survivor needs its atomic Animus receipt' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_soulpit_participant_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_soulpit_encounters f
        JOIN game_activity_handoff_participants p ON p.handoff_id=f.handoff_id
        WHERE f.handoff_id=NEW.handoff_id AND p.character_id=NEW.character_id
            AND f.revision=0 AND f.created_xact_id=pg_current_xact_id()
            AND p.entry_x=32375 AND p.entry_y=31158+NEW.staging_slot AND p.entry_floor=-8)
    THEN RAISE EXCEPTION 'Soulpit membership is the exact shared entry snapshot' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION game_soulpit_reward_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_soulpit_encounters f
        JOIN game_soulpit_events e ON e.handoff_id=f.handoff_id AND e.occurrence_id=NEW.occurrence_id
        JOIN game_soulpit_participants p ON p.handoff_id=f.handoff_id AND p.character_id=NEW.character_id
        WHERE f.handoff_id=NEW.handoff_id AND f.species_key=NEW.species_key
            AND f.terminal AND e.created_xact_id=pg_current_xact_id()
            AND jsonb_typeof(e.effect->'Completed'->'surviving_slots')='array'
            AND (e.effect->'Completed'->'surviving_slots') @> to_jsonb(p.staging_slot)
    ) THEN RAISE EXCEPTION 'Animus reward requires an actual Soulpit survivor' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER game_soulpit_state_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_soulpit_encounters FOR EACH ROW EXECUTE FUNCTION game_soulpit_state_guard();
CREATE TRIGGER game_soulpit_participant_admit BEFORE INSERT
    ON game_soulpit_participants FOR EACH ROW EXECUTE FUNCTION game_soulpit_participant_admit();
CREATE CONSTRAINT TRIGGER game_soulpit_entry_consistent AFTER INSERT ON game_soulpit_encounters
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soulpit_entry_consistent();
CREATE CONSTRAINT TRIGGER game_soulpit_event_consistent AFTER INSERT ON game_soulpit_events
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soulpit_event_consistent();
CREATE CONSTRAINT TRIGGER game_soulpit_reward_consistent AFTER INSERT ON game_soulpit_survival_rewards
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soulpit_reward_consistent();

DO $$ DECLARE name TEXT; BEGIN
    FOREACH name IN ARRAY ARRAY['game_soulpit_participants','game_soulpit_events','game_soulpit_survival_rewards','game_soulpit_entry_handoff_commits'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_character_immutable()',name||'_immutable',name);
    END LOOP;
    FOREACH name IN ARRAY ARRAY['game_soulpit_encounters','game_soulpit_participants','game_soulpit_events','game_soulpit_survival_rewards','game_soulpit_entry_handoff_commits'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_character_reject_truncate()',name||'_no_truncate',name);
        EXECUTE format('REVOKE ALL ON %I FROM PUBLIC',name);
        EXECUTE format('GRANT SELECT,INSERT ON %I TO oteryn_game_runtime',name);
    END LOOP;
    FOREACH name IN ARRAY ARRAY['game_soulpit_state_guard','game_soulpit_entry_consistent','game_soulpit_event_consistent','game_soulpit_reward_consistent','game_soulpit_participant_admit','game_soulpit_capture_shared_commit'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $$;
GRANT UPDATE ON game_soulpit_encounters TO oteryn_game_runtime;
-- Only the shared group's decision trigger may capture this receipt.
REVOKE INSERT ON game_soulpit_entry_handoff_commits FROM oteryn_game_runtime;
