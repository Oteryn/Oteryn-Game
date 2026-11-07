-- Reuse QENGINE2 6ad33a4b1/e9d98947f; preserve L2 atomic eleven-track admissions.
-- Source-authored foreign quest participants retain their original owners and pins.
-- One primary Quest receipt/revision remains the sole Character revision writer.
CREATE TABLE game_quest_reward_participants (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    transition_key TEXT NOT NULL,
    participant_index SMALLINT NOT NULL CHECK(participant_index BETWEEN 0 AND 2),
    quest_key TEXT NOT NULL CHECK(quest_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(quest_key)<=128),
    plan JSONB NOT NULL CHECK(jsonb_typeof(plan)='object' AND octet_length(plan::text)<=4096),
    pinned_content_revision TEXT NOT NULL,
    definition_hash BYTEA NOT NULL CHECK(octet_length(definition_hash)=32),
    track_keys TEXT[] NOT NULL,
    values_before BIGINT[] NOT NULL,
    values_after BIGINT[] NOT NULL,
    committed_character_revision NUMERIC(20,0) NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key,participant_index),
    UNIQUE(character_id,committed_character_revision,quest_key),
    CHECK(cardinality(track_keys) BETWEEN 1 AND 16
        AND cardinality(values_before)=cardinality(track_keys)
        AND cardinality(values_after)=cardinality(track_keys)
        AND array_ndims(track_keys)=1 AND array_lower(track_keys,1)=1
        AND array_ndims(values_before)=1 AND array_lower(values_before,1)=1
        AND array_ndims(values_after)=1 AND array_lower(values_after,1)=1
        AND array_position(track_keys,NULL) IS NULL
        AND array_position(values_before,NULL) IS NULL AND array_position(values_after,NULL) IS NULL),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_reward_participant_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE s game_quest_reward_sources%ROWTYPE; r game_character_quest_receipts%ROWTYPE; e JSONB; i INTEGER;
BEGIN
    SELECT * INTO s FROM game_quest_reward_sources WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR NEW.created_xact_id<>pg_current_xact_id() OR s.created_xact_id<>pg_current_xact_id()
        OR s.plan->'participants'->NEW.participant_index IS DISTINCT FROM NEW.plan
        OR NEW.plan->>'quest' IS DISTINCT FROM NEW.quest_key THEN
        RAISE EXCEPTION 'quest participant lacks exact same-transaction source' USING ERRCODE='23514';
    END IF;
    SELECT * INTO r FROM game_character_quest_receipts WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR r.cause_kind<>'command' OR r.quest_key=NEW.quest_key
        OR r.created_xact_id<>pg_current_xact_id()
        OR r.committed_character_revision<>NEW.committed_character_revision
        OR jsonb_array_length(NEW.plan->'storage')<>cardinality(NEW.track_keys)
        OR cardinality(NEW.track_keys)<>(SELECT count(DISTINCT t) FROM unnest(NEW.track_keys) t)
        OR cardinality(NEW.track_keys)<>(SELECT count(DISTINCT declared_storage.value->>'track') FROM jsonb_array_elements(NEW.plan->'storage') AS declared_storage(value))
        OR NEW.track_keys && r.track_keys THEN
        RAISE EXCEPTION 'quest participant is not bound to primary revision' USING ERRCODE='23514';
    END IF;
    FOR e IN SELECT value FROM jsonb_array_elements(NEW.plan->'storage') LOOP
        i:=array_position(NEW.track_keys,e->>'track');
        IF i IS NULL OR NEW.values_before[i] NOT BETWEEN (e->>'minimum')::bigint AND (e->>'maximum')::bigint
            OR NEW.values_after[i] IS DISTINCT FROM (CASE WHEN coalesce((e->>'preserve')::boolean,false)
                THEN NEW.values_before[i] ELSE (e->>'set')::bigint END)
            OR NOT EXISTS(SELECT 1 FROM game_character_quest_tracks t WHERE t.character_id=NEW.character_id
                AND t.quest_key=NEW.quest_key AND t.track_key=e->>'track' AND t.value=NEW.values_after[i]
                AND t.committed_character_revision=NEW.committed_character_revision) THEN
            RAISE EXCEPTION 'quest participant does not equal exact source effects' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM game_character_quest_states q WHERE q.character_id=NEW.character_id
        AND q.quest_key=NEW.quest_key AND q.pinned_content_revision=NEW.pinned_content_revision
        AND q.definition_hash=NEW.definition_hash AND q.committed_character_revision=NEW.committed_character_revision) THEN
        RAISE EXCEPTION 'quest participant lacks its original owner state' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE FUNCTION game_quest_reward_participants_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE n INTEGER;
BEGIN
    n:=jsonb_array_length(coalesce(NEW.plan->'participants','[]'::jsonb));
    IF n>3 OR jsonb_array_length(NEW.plan->'storage') + coalesce((SELECT sum(jsonb_array_length(p->'storage')) FROM jsonb_array_elements(coalesce(NEW.plan->'participants','[]'::jsonb)) p),0)>16
        OR n<>(SELECT count(*) FROM game_quest_reward_participants p WHERE p.character_id=NEW.character_id
        AND p.cause_id=NEW.cause_id AND p.cause_ordinal=NEW.cause_ordinal AND p.transition_key=NEW.transition_key
        AND p.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest source has incomplete participants' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_participant_complete AFTER INSERT ON game_quest_reward_participants
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_participant_complete();
CREATE CONSTRAINT TRIGGER game_quest_reward_participants_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_participants_complete();
CREATE TRIGGER game_quest_reward_participants_immutable BEFORE UPDATE OR DELETE ON game_quest_reward_participants
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_reward_participants_no_truncate BEFORE TRUNCATE ON game_quest_reward_participants
    EXECUTE FUNCTION game_item_reject_truncate();

-- Keep the original single-owner proof arm, adding only exact participant proofs.
CREATE OR REPLACE FUNCTION game_character_quest_track_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_receipt game_character_quest_receipts%ROWTYPE; v_index INTEGER;
BEGIN
    SELECT * INTO v_receipt FROM game_character_quest_receipts
        WHERE character_id=NEW.character_id AND committed_character_revision=NEW.committed_character_revision;
    v_index:=array_position(v_receipt.track_keys,NEW.track_key);
    IF (NOT FOUND OR v_index IS NULL OR v_receipt.quest_key<>NEW.quest_key
        OR v_receipt.values_after[v_index]<>NEW.value
        OR (TG_OP='UPDATE' AND v_receipt.values_before[v_index]<>OLD.value))
        AND NOT EXISTS(SELECT 1 FROM game_quest_reward_participants p
            WHERE p.character_id=NEW.character_id AND p.committed_character_revision=NEW.committed_character_revision
                AND p.quest_key=NEW.quest_key AND array_position(p.track_keys,NEW.track_key) IS NOT NULL
                AND p.values_after[array_position(p.track_keys,NEW.track_key)]=NEW.value
                AND (TG_OP<>'UPDATE' OR p.values_before[array_position(p.track_keys,NEW.track_key)]=OLD.value)
                AND p.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest track row does not equal the effect of its receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_character_quest_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r WHERE r.character_id=NEW.character_id
            AND r.committed_character_revision=NEW.committed_character_revision AND r.quest_key=NEW.quest_key
            AND r.pinned_content_revision=NEW.pinned_content_revision AND r.definition_hash=NEW.definition_hash)
        AND NOT EXISTS(SELECT 1 FROM game_quest_reward_participants p WHERE p.character_id=NEW.character_id
            AND p.committed_character_revision=NEW.committed_character_revision AND p.quest_key=NEW.quest_key
            AND p.pinned_content_revision=NEW.pinned_content_revision AND p.definition_hash=NEW.definition_hash
            AND p.created_xact_id=pg_current_xact_id()))
        OR (TG_OP='INSERT' AND NEW.started_character_revision<>NEW.committed_character_revision)
        OR (NEW.completed_character_revision IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.committed_character_revision=NEW.completed_character_revision
                AND r.quest_key=NEW.quest_key AND r.completes)) THEN
        RAISE EXCEPTION 'quest state is not written by a receipt of its quest' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $migration$
DECLARE f TEXT;
BEGIN
    FOREACH f IN ARRAY ARRAY['game_quest_reward_participant_complete','game_quest_reward_participants_complete',
        'game_character_quest_track_guard','game_character_quest_state_guard'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',f);
    END LOOP;
END $migration$;
REVOKE ALL ON game_quest_reward_participants FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_reward_participants TO oteryn_game_runtime;
GRANT SELECT ON game_quest_reward_participants TO oteryn_game_control;

-- Bounded source operations use the existing Quest clock, arithmetic and owner.
-- Exact effect proofs extend the participant arm; legacy storage proofs remain valid.
CREATE FUNCTION game_quest_reward_effects_valid(v_plan JSONB,v_keys TEXT[],v_before BIGINT[],v_after BIGINT[])
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
DECLARE v_effect JSONB; v_index INTEGER; v_expected NUMERIC; v_condition BOOLEAN;
    v_now NUMERIC:=floor(extract(epoch FROM transaction_timestamp()));
BEGIN
    IF jsonb_typeof(v_plan->'storage') IS DISTINCT FROM 'array'
        OR jsonb_typeof(coalesce(v_plan->'state_ops','[]'::jsonb)) IS DISTINCT FROM 'array'
        OR cardinality(v_keys)<>jsonb_array_length(v_plan->'storage')+jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))
        OR cardinality(v_keys) NOT BETWEEN 1 AND 16
        OR cardinality(v_keys)<>(SELECT count(DISTINCT k) FROM unnest(v_keys) k)
        OR cardinality(v_keys)<>(SELECT count(DISTINCT declared_effect.value->>'track') FROM (
            SELECT value FROM jsonb_array_elements(v_plan->'storage')
            UNION ALL SELECT value FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb))) declared_effect)
        THEN RETURN false;
    END IF;
    FOR v_effect IN SELECT value FROM jsonb_array_elements(v_plan->'storage') LOOP
        v_index:=array_position(v_keys,v_effect->>'track');
        IF v_index IS NULL OR v_before[v_index] NOT BETWEEN (v_effect->>'minimum')::bigint AND (v_effect->>'maximum')::bigint
            THEN RETURN false; END IF;
        IF coalesce((v_effect->>'preserve')::boolean,false)
            OR (v_effect ? 'set_if' AND v_before[v_index] IS DISTINCT FROM (v_effect->>'set_if')::bigint)
        THEN v_expected:=v_before[v_index];
        ELSE v_expected:=(v_effect->>'set')::numeric; END IF;
        IF v_expected IS DISTINCT FROM v_after[v_index]::numeric THEN RETURN false; END IF;
    END LOOP;
    FOR v_effect IN SELECT value FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb)) LOOP
        v_index:=array_position(v_keys,v_effect->>'track');
        IF v_index IS NULL THEN RETURN false; END IF;
        CASE v_effect #>> '{from,kind}'
            WHEN 'any' THEN v_condition:=true;
            WHEN 'between' THEN v_condition:=v_before[v_index] BETWEEN (v_effect #>> '{from,minimum}')::bigint AND (v_effect #>> '{from,maximum}')::bigint;
            WHEN 'elapsed_at_least' THEN v_condition:= (v_effect #>> '{from,seconds}')::numeric BETWEEN 0 AND 31622400
                AND v_now-v_before[v_index]::numeric >= (v_effect #>> '{from,seconds}')::numeric;
            ELSE RETURN false;
        END CASE;
        IF NOT coalesce(v_condition,false) THEN RETURN false; END IF;
        CASE v_effect #>> '{effect,kind}'
            WHEN 'add' THEN v_expected:=v_before[v_index]::numeric+(v_effect #>> '{effect,value}')::numeric;
            WHEN 'set_now' THEN v_expected:=v_now;
            ELSE RETURN false;
        END CASE;
        IF v_expected IS DISTINCT FROM v_after[v_index]::numeric THEN RETURN false; END IF;
    END LOOP;
    RETURN true;
END $$;

CREATE OR REPLACE FUNCTION game_quest_reward_participant_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_source game_quest_reward_sources%ROWTYPE; v_receipt game_character_quest_receipts%ROWTYPE; v_index INTEGER;
BEGIN
    SELECT * INTO v_source FROM game_quest_reward_sources WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR NEW.created_xact_id<>pg_current_xact_id() OR v_source.created_xact_id<>pg_current_xact_id()
        OR v_source.plan->'participants'->NEW.participant_index IS DISTINCT FROM NEW.plan
        OR NEW.plan->>'quest' IS DISTINCT FROM NEW.quest_key THEN
        RAISE EXCEPTION 'quest participant lacks exact same-transaction source' USING ERRCODE='23514';
    END IF;
    SELECT * INTO v_receipt FROM game_character_quest_receipts WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR v_receipt.cause_kind<>'command' OR v_receipt.quest_key=NEW.quest_key
        OR v_receipt.created_xact_id<>pg_current_xact_id()
        OR v_receipt.committed_character_revision<>NEW.committed_character_revision
        OR NEW.track_keys && v_receipt.track_keys
        OR NOT game_quest_reward_effects_valid(NEW.plan,NEW.track_keys,NEW.values_before,NEW.values_after) THEN
        RAISE EXCEPTION 'quest participant is not bound to exact primary effects' USING ERRCODE='23514';
    END IF;
    FOR v_index IN 1..cardinality(NEW.track_keys) LOOP
        IF NOT EXISTS(SELECT 1 FROM game_character_quest_tracks t WHERE t.character_id=NEW.character_id
            AND t.quest_key=NEW.quest_key AND t.track_key=NEW.track_keys[v_index] AND t.value=NEW.values_after[v_index]
            AND t.committed_character_revision=NEW.committed_character_revision) THEN
            RAISE EXCEPTION 'quest participant does not equal exact source effects' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM game_character_quest_states q WHERE q.character_id=NEW.character_id
        AND q.quest_key=NEW.quest_key AND q.pinned_content_revision=NEW.pinned_content_revision
        AND q.definition_hash=NEW.definition_hash AND q.committed_character_revision=NEW.committed_character_revision) THEN
        RAISE EXCEPTION 'quest participant lacks its original owner state' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE FUNCTION game_quest_reward_state_ops_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_receipt game_character_quest_receipts%ROWTYPE; v_count BIGINT;
BEGIN
    v_count:=jsonb_array_length(NEW.plan->'storage')+jsonb_array_length(coalesce(NEW.plan->'state_ops','[]'::jsonb))
        +coalesce((SELECT sum(jsonb_array_length(part.value->'storage')+jsonb_array_length(coalesce(part.value->'state_ops','[]'::jsonb)))
            FROM jsonb_array_elements(coalesce(NEW.plan->'participants','[]'::jsonb)) part),0);
    IF v_count>16 THEN RAISE EXCEPTION 'quest source operation bound exceeded' USING ERRCODE='23514'; END IF;
    IF jsonb_array_length(coalesce(NEW.plan->'state_ops','[]'::jsonb))>0 THEN
        SELECT * INTO v_receipt FROM game_character_quest_receipts WHERE character_id=NEW.character_id
            AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
        IF NOT FOUND OR v_receipt.created_xact_id<>pg_current_xact_id()
            OR NEW.created_xact_id<>pg_current_xact_id()
            OR NOT game_quest_reward_effects_valid(NEW.plan,v_receipt.track_keys,v_receipt.values_before,v_receipt.values_after) THEN
            RAISE EXCEPTION 'quest source operations lack exact same-transaction effects' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_state_ops_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_state_ops_complete();
DO $migration$
DECLARE v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY['game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])',
        'game_quest_reward_participant_complete()','game_quest_reward_state_ops_complete()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',v_function,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',v_function);
    END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION game_quest_reward_effects_valid(JSONB,TEXT[],BIGINT[],BIGINT[]) TO oteryn_game_runtime;
