-- W11: Rotten Blood is a bounded Quest projection and a Character receipt owner.
-- Access uses the existing game_character_quest_tracks owner. No second wallet,
-- item consumer, account outfit table, encounter timer or encounter engine.
CREATE TABLE IF NOT EXISTS game_character_rotten_blood (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    regular_taints SMALLINT NOT NULL DEFAULT 0 CHECK (regular_taints BETWEEN 0 AND 4),
    child_flags SMALLINT NOT NULL DEFAULT 0 CHECK (child_flags BETWEEN 0 AND 15),
    final_taint BOOLEAN NOT NULL DEFAULT false,
    completed BOOLEAN NOT NULL DEFAULT false,
    last_cleanse_at BIGINT CHECK (last_cleanse_at >= 0)
);

CREATE TABLE IF NOT EXISTS game_character_rotten_blood_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    occurrence BYTEA NOT NULL CHECK (octet_length(occurrence) BETWEEN 1 AND 256),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding)=32),
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 5),
    source_death_key BYTEA REFERENCES game_party_kill_xp_receipts(death_key),
    source_terminal_attempt BYTEA CHECK (octet_length(source_terminal_attempt)=32),
    source_party_minimum SMALLINT CHECK (source_party_minimum BETWEEN 0 AND 4),
    source_dealt_damage BOOLEAN,
    before_state JSONB NOT NULL CHECK (jsonb_typeof(before_state)='object'),
    result JSONB NOT NULL CHECK (jsonb_typeof(result)='object'),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551615),
    committed_character_revision NUMERIC(20,0)
        CHECK (committed_character_revision=original_character_revision+1
               AND committed_character_revision<=18446744073709551615),
    level_before BIGINT NOT NULL,
    level_after BIGINT NOT NULL CHECK (level_after=level_before),
    experience_before BIGINT NOT NULL,
    experience_after BIGINT NOT NULL CHECK (experience_after=experience_before),
    profile_revision TEXT NOT NULL,
    ruleset_revision TEXT NOT NULL,
    content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    reward_revision TEXT NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,occurrence),
    UNIQUE (character_id,committed_character_revision),
    CHECK ((kind IN (1,2) AND source_death_key IS NOT NULL
            AND octet_length(source_death_key)=52 AND source_party_minimum IS NOT NULL
            AND source_dealt_damage IS NOT NULL AND source_terminal_attempt IS NULL)
        OR (kind IN (3,4) AND source_death_key IS NULL
            AND source_party_minimum IS NULL AND source_dealt_damage IS NULL AND source_terminal_attempt IS NULL)
        OR (kind=5 AND source_death_key IS NULL AND source_party_minimum IS NULL
            AND source_dealt_damage IS NULL AND source_terminal_attempt IS NOT NULL))
);

CREATE OR REPLACE VIEW game_character_hazard_revision_chain_rotten_blood WITH (security_invoker=true) AS
    SELECT * FROM game_character_hazard_revision_chain_soul_war
    UNION ALL
    SELECT character_id,original_character_revision,committed_character_revision,
        level_before,level_after,experience_before,experience_after,
        profile_revision,ruleset_revision,content_revision,simulation_revision,
        evidence_revision,declaration_revision,policy_revision,reward_revision
    FROM game_character_rotten_blood_receipts WHERE committed_character_revision IS NOT NULL;

DO $migration$
DECLARE body TEXT;
BEGIN
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure)
        INTO STRICT body;
    IF position('game_character_hazard_revision_chain_soul_war' IN body)>0 THEN
        EXECUTE replace(body,'game_character_hazard_revision_chain_soul_war','game_character_hazard_revision_chain_rotten_blood');
    ELSIF position('game_character_hazard_revision_chain_rotten_blood' IN body)=0
        AND NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass
            AND tgname='game_rotten_blood_character_chain') THEN
        RAISE EXCEPTION 'Rotten Blood requires the current Soul War Character receipt chain';
    END IF;
    -- A re-run keeps any later wrapper above this already-installed chain.
END $migration$;

-- Every semantic predecessor and successor is bounded and complete; JSON null
-- is intentionally distinct from a missing field or an unknown schema extension.
CREATE OR REPLACE FUNCTION game_rotten_blood_state_json_valid(value JSONB) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT jsonb_typeof(value)='object'
        AND value ?& ARRAY['access_stage','regular_taints','child_flags','final_taint','completed','last_cleanse_at']
        AND value - ARRAY['access_stage','regular_taints','child_flags','final_taint','completed','last_cleanse_at']='{}'::jsonb
        AND jsonb_typeof(value->'access_stage')='number'
        AND (value->>'access_stage')::numeric BETWEEN 0 AND 5
        AND (value->>'access_stage')::numeric=trunc((value->>'access_stage')::numeric)
        AND jsonb_typeof(value->'regular_taints')='number'
        AND (value->>'regular_taints')::numeric BETWEEN 0 AND 4
        AND (value->>'regular_taints')::numeric=trunc((value->>'regular_taints')::numeric)
        AND jsonb_typeof(value->'child_flags')='number'
        AND (value->>'child_flags')::numeric BETWEEN 0 AND 15
        AND (value->>'child_flags')::numeric=trunc((value->>'child_flags')::numeric)
        AND jsonb_typeof(value->'final_taint')='boolean'
        AND jsonb_typeof(value->'completed')='boolean'
        AND (value->'last_cleanse_at'='null'::jsonb
            OR (jsonb_typeof(value->'last_cleanse_at')='number'
                AND (value->>'last_cleanse_at')::numeric BETWEEN 0 AND 9223372036854775807
                AND (value->>'last_cleanse_at')::numeric=trunc((value->>'last_cleanse_at')::numeric)))
$$;

CREATE OR REPLACE FUNCTION game_rotten_blood_receipt_admit() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    prior JSONB:=NEW.before_state;
    successor JSONB:=NEW.result->'state';
    creature TEXT;
    flag INTEGER;
    expected JSONB;
    now_seconds BIGINT:=floor(extract(epoch FROM now()))::bigint;
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id()
        OR NOT coalesce(game_rotten_blood_state_json_valid(prior),false)
        OR NOT coalesce(game_rotten_blood_state_json_valid(successor),false)
        OR NOT (NEW.result ?& ARRAY['state','refusal','grant_base_outfit'])
        OR NEW.result-ARRAY['state','refusal','grant_base_outfit']<>'{}'::jsonb
        OR jsonb_typeof(NEW.result->'grant_base_outfit')<>'boolean'
        OR successor->'access_stage'<>prior->'access_stage'
        OR (NEW.result->'refusal'<>'null'::jsonb
            AND (successor<>prior OR NEW.committed_character_revision IS NOT NULL
                OR NEW.result->'grant_base_outfit'<>'false'::jsonb))
        OR ((successor<>prior) IS DISTINCT FROM (NEW.committed_character_revision IS NOT NULL))
    THEN
        RAISE EXCEPTION 'invalid Rotten Blood receipt shape or successor' USING ERRCODE='23514';
    END IF;
    -- Access cannot be invented by a native kill or by cleansing.
    IF (prior->>'access_stage')::bigint<>coalesce((SELECT value FROM game_character_quest_tracks
        WHERE character_id=NEW.character_id
          AND track_key='oteryn:quest-progress/rotten-blood/access'),0) THEN
        RAISE EXCEPTION 'Rotten Blood receipt access lacks its Quest track' USING ERRCODE='23514';
    END IF;
    IF NEW.kind IN (1,2) THEN
        SELECT r.creature_key INTO creature
            FROM game_party_kill_xp_receipts r
            JOIN game_party_kill_xp_members m USING(death_key)
            WHERE r.death_key=NEW.source_death_key AND m.character_id=NEW.character_id
              AND r.created_xact_id=pg_current_xact_id();
        IF creature IS NULL
            OR (NEW.kind=1 AND creature NOT IN ('oteryn:creature.ichgahal','oteryn:creature.murcion','oteryn:creature.vemiath','oteryn:creature.chagorz'))
            OR (NEW.kind=2 AND creature<>'oteryn:creature.bakragore') THEN
            RAISE EXCEPTION 'Rotten Blood kill lacks its same-transaction native source' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.result->'refusal'<>'null'::jsonb THEN
        RETURN NEW;
    END IF;
    IF NEW.kind=1 THEN
        flag:=CASE creature WHEN 'oteryn:creature.ichgahal' THEN 1 WHEN 'oteryn:creature.murcion' THEN 2
              WHEN 'oteryn:creature.vemiath' THEN 4 WHEN 'oteryn:creature.chagorz' THEN 8 END;
        expected:=prior || jsonb_build_object(
            'regular_taints',(prior->>'regular_taints')::integer+1,
            'child_flags',(prior->>'child_flags')::integer | flag);
        IF (prior->>'access_stage')::integer<4 OR NOT NEW.source_dealt_damage
            OR NEW.source_party_minimum<>(prior->>'regular_taints')::integer
            OR ((prior->>'child_flags')::integer & flag)<>0
            OR successor<>expected OR NEW.result->'grant_base_outfit'<>'false'::jsonb THEN
            RAISE EXCEPTION 'Rotten Blood child taint has no valid distinct-boss gain' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.kind=2 THEN
        expected:=prior || jsonb_build_object('regular_taints',0,'child_flags',0,'final_taint',true,'completed',true);
        IF (prior->>'access_stage')::integer<4 OR NOT NEW.source_dealt_damage
            OR (prior->>'child_flags')::integer<>15 OR successor<>expected
            OR (NEW.result->>'grant_base_outfit')::boolean=(prior->>'completed')::boolean THEN
            RAISE EXCEPTION 'Rotten Blood final reward lacks its complete first-victory proof' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.kind=5 THEN
        -- 0711 adds the shared terminal-decision FK and source admission proof.
        flag:=(prior->>'child_flags')::integer & ~(successor->>'child_flags')::integer;
        expected:=prior || jsonb_build_object(
            'regular_taints',greatest(0,(prior->>'regular_taints')::integer-2),
            'child_flags',(prior->>'child_flags')::integer & ~flag);
        IF bit_count(flag::bit(4))<>2 OR successor<>expected
            OR NEW.result->'grant_base_outfit'<>'false'::jsonb THEN
            RAISE EXCEPTION 'Rotten Blood failed final lacks its two-child successor' USING ERRCODE='23514';
        END IF;
    ELSE
        expected:=prior || jsonb_build_object('regular_taints',0,'child_flags',0,
            'final_taint',NEW.kind=3 AND (prior->>'final_taint')::boolean,
            'last_cleanse_at',now_seconds);
        IF successor<>expected OR NEW.result->'grant_base_outfit'<>'false'::jsonb
            OR (prior->>'regular_taints')::integer=0 AND (prior->>'child_flags')::integer=0
                AND (NEW.kind=3 OR NOT (prior->>'final_taint')::boolean)
            OR (prior->'last_cleanse_at'<>'null'::jsonb
                AND now_seconds::numeric<(prior->>'last_cleanse_at')::numeric+244800) THEN
            RAISE EXCEPTION 'Rotten Blood cleanse lacks cooldown or taint successor' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_rotten_blood_state_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE predecessor JSONB;
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Rotten Blood state cannot be deleted' USING ERRCODE='23514';
    END IF;
    predecessor:=CASE WHEN TG_OP='INSERT' THEN
        jsonb_build_object('regular_taints',0,'child_flags',0,'final_taint',false,'completed',false,'last_cleanse_at',NULL)
        ELSE to_jsonb(OLD)-'character_id' END;
    IF NOT EXISTS(
        SELECT 1 FROM game_character_rotten_blood_receipts r
            JOIN game_character_roots c USING(character_id)
            WHERE r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()
              AND r.committed_character_revision=c.character_revision
              AND r.result->'refusal'='null'::jsonb
              AND (r.before_state-'access_stage')=predecessor
              AND ((r.result->'state')-'access_stage'::text)=to_jsonb(NEW)-'character_id'
    ) THEN
        RAISE EXCEPTION 'Rotten Blood state lacks its same-transaction Character receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION game_rotten_blood_receipt_complete() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.committed_character_revision IS NOT NULL AND NOT EXISTS(
        SELECT 1 FROM game_character_rotten_blood s
            WHERE s.character_id=NEW.character_id
              AND to_jsonb(s)-'character_id'=(NEW.result->'state')-'access_stage'::text
    ) THEN
        RAISE EXCEPTION 'Rotten Blood Character receipt lacks its state successor' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_receipt_stamp') THEN
        CREATE TRIGGER game_rotten_blood_receipt_stamp BEFORE INSERT ON game_character_rotten_blood_receipts
            FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_receipt_admit') THEN
        CREATE TRIGGER game_rotten_blood_receipt_admit BEFORE INSERT ON game_character_rotten_blood_receipts
            FOR EACH ROW EXECUTE FUNCTION game_rotten_blood_receipt_admit();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_receipt_immutable') THEN
        CREATE TRIGGER game_rotten_blood_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_rotten_blood_receipts
            FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_receipt_no_truncate') THEN
        CREATE TRIGGER game_rotten_blood_receipt_no_truncate BEFORE TRUNCATE ON game_character_rotten_blood_receipts
            EXECUTE FUNCTION game_character_reject_truncate();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_receipt_complete') THEN
        CREATE CONSTRAINT TRIGGER game_rotten_blood_receipt_complete AFTER INSERT ON game_character_rotten_blood_receipts
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_blood_receipt_complete();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_character_chain') THEN
        CREATE CONSTRAINT TRIGGER game_rotten_blood_character_chain AFTER INSERT ON game_character_rotten_blood_receipts
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood'::regclass AND tgname='game_rotten_blood_state_proven') THEN
        CREATE CONSTRAINT TRIGGER game_rotten_blood_state_proven AFTER INSERT OR UPDATE OR DELETE ON game_character_rotten_blood
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_blood_state_proven();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood'::regclass AND tgname='game_rotten_blood_state_no_truncate') THEN
        CREATE TRIGGER game_rotten_blood_state_no_truncate BEFORE TRUNCATE ON game_character_rotten_blood
            EXECUTE FUNCTION game_character_reject_truncate();
    END IF;
END $migration$;

DO $migration$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_rotten_blood_state_json_valid(JSONB)',
        'game_rotten_blood_receipt_admit()',
        'game_rotten_blood_state_proven()',
        'game_rotten_blood_receipt_complete()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',signature);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
    END LOOP;
END $migration$;
REVOKE ALL ON game_character_rotten_blood,game_character_rotten_blood_receipts,
    game_character_hazard_revision_chain_rotten_blood FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_character_rotten_blood TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_rotten_blood_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_hazard_revision_chain_rotten_blood TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_rotten_blood_state_json_valid(JSONB) TO oteryn_game_runtime;

-- The fixed 5m access price uses the established coin/bank fee owner. This
-- immutable join is its source proof, not another balance or currency ledger.
CREATE TABLE IF NOT EXISTS game_rotten_blood_access_fees (
    fee_occurrence UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(fee_occurrence)),
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
    transition_key TEXT NOT NULL CHECK(transition_key='oteryn:quest-transition/rotten-blood/purchase-access'),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(character_id,cause_id,cause_ordinal,transition_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);

-- Internal audit cause 14 is deliberately closed to this one service/price.
-- Preserve every historical cause constraint and every prior proof arm.
DO $migration$
DECLARE body TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
        WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
    IF body !~ 'cause_kind[[:space:]]*=[[:space:]]*14' THEN
        ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
        EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('
            ||substr(body,7)||') OR cause_kind=14)';
    END IF;
    SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
        WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
    IF position('oteryn:service.quest.rotten-blood.access' IN body)=0 THEN
        ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
        EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
            ||substr(body,7)||') OR (cause_kind=14 AND charm_key IS NULL '
            ||'AND purchase_service_key=''oteryn:service.quest.rotten-blood.access'' AND fee_gold_units=5000000)) NOT VALID';
    END IF;
    SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('game_rotten_blood_access_fees' IN body)=0 THEN
        IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN
            RAISE EXCEPTION 'Rotten Blood access requires the existing fee Character proof arm';
        END IF;
        EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
        '(NEW.cause_kind=14 AND EXISTS(SELECT 1 FROM game_rotten_blood_access_fees a '
        ||'JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) '
        ||'WHERE a.character_id=NEW.character_id AND a.fee_occurrence=NEW.cause_occurrence_id '
        ||'AND q.committed_character_revision=NEW.committed_character_revision '
        ||'AND q.created_xact_id=pg_current_xact_id() AND a.created_xact_id=pg_current_xact_id() '
        ||'AND NEW.fee_gold_units=5000000)) OR (NEW.cause_kind=5 AND EXISTS');
    END IF;
END $migration$;

CREATE OR REPLACE FUNCTION game_rotten_blood_access_fee_complete() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.transition_key<>'oteryn:quest-transition/rotten-blood/purchase-access' THEN
        RETURN NULL;
    END IF;
    IF NEW.cause_kind<>'command' OR NEW.quest_key<>'oteryn:quest/rotten-blood'
        OR NEW.track_keys<>ARRAY['oteryn:quest-progress/rotten-blood/access']::text[]
        OR cardinality(NEW.values_before)<>1 OR NEW.values_before[1] NOT BETWEEN 0 AND 2
        OR NEW.values_after<>ARRAY[4]::bigint[]
        OR NOT EXISTS(SELECT 1 FROM game_rotten_blood_access_fees a
            JOIN game_item_fee_burns f ON f.cause_kind=14 AND f.cause_occurrence_id=a.fee_occurrence
            WHERE a.character_id=NEW.character_id AND a.cause_id=NEW.cause_id
              AND a.cause_ordinal=NEW.cause_ordinal AND a.transition_key=NEW.transition_key
              AND a.created_xact_id=pg_current_xact_id() AND f.created_xact_id=pg_current_xact_id()
              AND f.character_id=NEW.character_id AND f.fee_gold_units=5000000
              AND f.committed_character_revision=NEW.committed_character_revision) THEN
        RAISE EXCEPTION 'Rotten Blood paid access lacks its atomic exact fee' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_quest_receipts'::regclass AND tgname='game_rotten_blood_access_fee_complete') THEN
        CREATE CONSTRAINT TRIGGER game_rotten_blood_access_fee_complete AFTER INSERT ON game_character_quest_receipts
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_blood_access_fee_complete();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_rotten_blood_access_fees'::regclass AND tgname='game_rotten_blood_access_fee_stamp') THEN
        CREATE TRIGGER game_rotten_blood_access_fee_stamp BEFORE INSERT ON game_rotten_blood_access_fees
            FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_rotten_blood_access_fees'::regclass AND tgname='game_rotten_blood_access_fee_immutable') THEN
        CREATE TRIGGER game_rotten_blood_access_fee_immutable BEFORE UPDATE OR DELETE ON game_rotten_blood_access_fees
            FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
    END IF;
END $migration$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_rotten_blood_access_fees'::regclass AND tgname='game_rotten_blood_access_fee_no_truncate') THEN
        CREATE TRIGGER game_rotten_blood_access_fee_no_truncate BEFORE TRUNCATE ON game_rotten_blood_access_fees
            EXECUTE FUNCTION game_character_reject_truncate();
    END IF;
END $migration$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_rotten_blood_access_fee_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_rotten_blood_access_fee_complete() FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_rotten_blood_access_fee_complete() TO oteryn_game_runtime;
REVOKE ALL ON game_rotten_blood_access_fees FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT ON game_rotten_blood_access_fees TO oteryn_game_runtime;
