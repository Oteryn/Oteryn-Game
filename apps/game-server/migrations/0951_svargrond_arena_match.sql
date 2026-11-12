-- SV15: arena progress is metadata; entry alone advances the Character chain for its fee.
CREATE TABLE game_arena_match_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    payload BYTEA NOT NULL CHECK(octet_length(payload)<=32),
    before_state JSONB NOT NULL CHECK(jsonb_typeof(before_state)='object'),
    after_state JSONB NOT NULL CHECK(jsonb_typeof(after_state)='object'),
    reply BYTEA NOT NULL CHECK(octet_length(reply) BETWEEN 1 AND 128),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id),
    UNIQUE(character_id,game_session_id,command_id)
);
CREATE INDEX game_arena_match_receipts_character ON game_arena_match_receipts(character_id);
CREATE TABLE game_character_arena_matches (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    body JSONB NOT NULL CHECK(jsonb_typeof(body)='object'
        AND (body->>'completed')::int BETWEEN 0 AND 3
        AND (body->>'rank')::int BETWEEN 0 AND 3
        AND (body->>'pit')::int BETWEEN 0 AND 10
        AND (body->>'phase')::int BETWEEN 0 AND 3),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    FOREIGN KEY(character_id,game_session_id,command_id)
        REFERENCES game_arena_match_receipts(character_id,game_session_id,command_id)
);
CREATE INDEX game_character_arena_match_receipt ON game_character_arena_matches(game_session_id,command_id);
CREATE TABLE game_arena_victories (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    run_id UUID NOT NULL,
    rank SMALLINT NOT NULL CHECK(rank BETWEEN 1 AND 3),
    pit SMALLINT NOT NULL CHECK(pit BETWEEN 1 AND 10),
    death_key BYTEA NOT NULL UNIQUE CHECK(octet_length(death_key)=52),
    PRIMARY KEY(character_id,run_id,pit)
);
CREATE TABLE game_arena_entry_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    fee_occurrence UUID NOT NULL UNIQUE,
    rank SMALLINT NOT NULL CHECK(rank BETWEEN 1 AND 3),
    original_character_revision NUMERIC(20,0) NOT NULL,
    committed_character_revision NUMERIC(20,0) NOT NULL CHECK(committed_character_revision=original_character_revision+1),
    level_before BIGINT NOT NULL,
    level_after BIGINT NOT NULL CHECK(level_after=level_before),
    experience_before BIGINT NOT NULL,
    experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
    profile_revision TEXT NOT NULL,
    ruleset_revision TEXT NOT NULL,
    content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    reward_revision TEXT NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,committed_character_revision),
    FOREIGN KEY(character_id,game_session_id,command_id)
        REFERENCES game_arena_match_receipts(character_id,game_session_id,command_id)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_arena_entry_command ON game_arena_entry_receipts(game_session_id,command_id);
-- Preserve the current chain and its name anchor for historical migration replay.
-- Only arena receipts are added; the live guard definition retains every later branch.
CREATE OR REPLACE VIEW game_character_hazard_revision_chain_rotten_blood_arena WITH (security_invoker=true) AS
    SELECT * FROM game_character_hazard_revision_chain_rotten_blood
    UNION ALL SELECT character_id,original_character_revision,committed_character_revision,
        level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
        content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
    FROM game_arena_entry_receipts;
DO $migration$
DECLARE body TEXT;
BEGIN
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('game_character_hazard_revision_chain_rotten_blood' IN body)=0 THEN
        RAISE EXCEPTION 'arena requires the current Character receipt-chain anchor';
    END IF;
    EXECUTE replace(body,'game_character_hazard_revision_chain_rotten_blood','game_character_hazard_revision_chain_rotten_blood_arena');
    SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
        WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
    ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
    EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('
        ||substr(body,7)||') OR cause_kind=17)';
    SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
        WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
    ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
    EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
        ||substr(body,7)||') OR (cause_kind=17 AND charm_key IS NULL AND '
        ||'purchase_service_key=''oteryn:service.svargrond-arena.entry'' AND fee_gold_units IN(1000,5000,10000))) NOT VALID';
    SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'arena requires the fee source-proof anchor'; END IF;
    EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
        '(NEW.cause_kind=17 AND EXISTS(SELECT 1 FROM game_arena_entry_receipts a WHERE '
        ||'a.character_id=NEW.character_id AND a.fee_occurrence=NEW.cause_occurrence_id '
        ||'AND a.committed_character_revision=NEW.committed_character_revision '
        ||'AND NEW.fee_gold_units=(ARRAY[1000,5000,10000])[a.rank] '
        ||'AND a.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
END $migration$;
CREATE FUNCTION game_arena_entry_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.cause_kind=17
        AND f.cause_occurrence_id=NEW.fee_occurrence AND f.character_id=NEW.character_id
        AND f.committed_character_revision=NEW.committed_character_revision
        AND f.fee_gold_units=(ARRAY[1000,5000,10000])[NEW.rank]
        AND f.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'arena entry lacks its atomic fee' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION game_arena_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_arena_match_receipts r WHERE r.character_id=NEW.character_id
        AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
        AND r.after_state=NEW.body AND r.created_xact_id=pg_current_xact_id()
        AND (TG_OP='INSERT' OR r.before_state=OLD.body)) THEN
        RAISE EXCEPTION 'arena state lacks its same-transaction predecessor/receipt' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_arena_entry_fee_complete AFTER INSERT ON game_arena_entry_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_arena_entry_complete();
CREATE CONSTRAINT TRIGGER game_arena_character_chain AFTER INSERT ON game_arena_entry_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_arena_state_proven BEFORE INSERT OR UPDATE ON game_character_arena_matches
    FOR EACH ROW EXECUTE FUNCTION game_arena_state_proven();
CREATE TRIGGER game_arena_receipt_immutable BEFORE UPDATE OR DELETE ON game_arena_match_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_arena_entry_immutable BEFORE UPDATE OR DELETE ON game_arena_entry_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_arena_receipt_stamp BEFORE INSERT ON game_arena_match_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_arena_entry_stamp BEFORE INSERT ON game_arena_entry_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_arena_state_no_truncate BEFORE TRUNCATE ON game_character_arena_matches EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_arena_receipt_no_truncate BEFORE TRUNCATE ON game_arena_match_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_arena_entry_no_truncate BEFORE TRUNCATE ON game_arena_entry_receipts EXECUTE FUNCTION game_character_reject_truncate();
DO $migration$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY['game_arena_entry_complete()','game_arena_state_proven()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',signature);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
    END LOOP;
END $migration$;
REVOKE ALL ON game_character_arena_matches,game_arena_match_receipts,game_arena_entry_receipts,game_character_hazard_revision_chain_rotten_blood_arena FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_arena_matches TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_arena_match_receipts,game_arena_entry_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_hazard_revision_chain_rotten_blood_arena TO oteryn_game_runtime;
GRANT SELECT ON game_character_arena_matches,game_arena_match_receipts,game_arena_entry_receipts TO oteryn_game_control;

REVOKE ALL ON game_arena_victories FROM PUBLIC;
GRANT SELECT,INSERT ON game_arena_victories TO oteryn_game_runtime;
GRANT SELECT ON game_arena_victories TO oteryn_game_control;
CREATE TRIGGER game_arena_victory_immutable BEFORE UPDATE OR DELETE ON game_arena_victories FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_arena_victory_no_truncate BEFORE TRUNCATE ON game_arena_victories EXECUTE FUNCTION game_character_reject_truncate();
