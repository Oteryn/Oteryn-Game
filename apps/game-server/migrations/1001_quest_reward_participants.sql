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
    CHECK(cardinality(track_keys) BETWEEN 1 AND 8
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
    IF n>3 OR jsonb_array_length(NEW.plan->'storage') + coalesce((SELECT sum(jsonb_array_length(p->'storage')) FROM jsonb_array_elements(coalesce(NEW.plan->'participants','[]'::jsonb)) p),0)>8
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
