-- PROG-ADV-1: durable Gem Atelier / Fragment Workshop character state.
CREATE TABLE game_character_atelier_state (
    character_id UUID PRIMARY KEY,
    atelier_revision BIGINT NOT NULL CHECK (atelier_revision >= 0),
    gold BIGINT NOT NULL CHECK (gold >= 0),
    fragments_basic BIGINT NOT NULL CHECK (fragments_basic >= 0),
    fragments_supreme BIGINT NOT NULL CHECK (fragments_supreme >= 0),
    state_json JSONB NOT NULL,
    CONSTRAINT game_character_atelier_state_character_fk
        FOREIGN KEY (character_id) REFERENCES game_character_roots(character_id) ON DELETE CASCADE
);

CREATE TABLE game_character_atelier_receipts (
    character_id UUID NOT NULL,
    occurrence UUID NOT NULL,
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256) = 32),
    before_revision BIGINT NOT NULL CHECK (before_revision >= 0),
    after_revision BIGINT NOT NULL CHECK (after_revision = before_revision + 1),
    outcome_json JSONB NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('GEM_INIT', 'SET_LOCK')),
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision >= 1),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1
               AND committed_character_revision <= 18446744073709551615),
    level_before BIGINT NOT NULL,
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL,
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    profile_revision TEXT NOT NULL, ruleset_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL, evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL, policy_revision TEXT NOT NULL, reward_revision TEXT NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (character_id, after_revision),
    PRIMARY KEY (character_id, occurrence),
    CONSTRAINT game_character_atelier_receipts_character_fk
        FOREIGN KEY (character_id) REFERENCES game_character_roots(character_id) ON DELETE CASCADE
);

GRANT SELECT, INSERT, UPDATE ON game_character_atelier_state TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_atelier_receipts TO oteryn_game_runtime;

-- One permanent initialization marker survives all later dismantles and eligibility lapses.
CREATE UNIQUE INDEX game_character_atelier_init_once
 ON game_character_atelier_receipts(character_id) WHERE kind='GEM_INIT';

CREATE VIEW game_character_atelier_revision_chain WITH (security_invoker=true) AS
 SELECT * FROM game_character_cyclopedia_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_atelier_receipts;
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_character_cyclopedia_revision_chain' IN body)=0 THEN
  RAISE EXCEPTION 'expected cyclopedia Character receipt chain';
 END IF;
 EXECUTE replace(body,'game_character_cyclopedia_revision_chain','game_character_atelier_revision_chain');
END $$;
GRANT SELECT ON game_character_atelier_revision_chain TO oteryn_game_runtime;

CREATE TRIGGER game_atelier_receipt_stamp BEFORE INSERT ON game_character_atelier_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_atelier_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_atelier_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_atelier_receipt_no_truncate BEFORE TRUNCATE ON game_character_atelier_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE CONSTRAINT TRIGGER game_atelier_character_chain AFTER INSERT ON game_character_atelier_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE FUNCTION game_atelier_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts r
     WHERE r.character_id=NEW.character_id AND r.after_revision=NEW.atelier_revision
       AND r.outcome_json=NEW.state_json AND r.created_xact_id=pg_current_xact_id())
    OR NEW.gold<>0 OR NEW.fragments_basic<>0 OR NEW.fragments_supreme<>0 THEN
  RAISE EXCEPTION 'Atelier state requires its same-transaction receipt; assets belong to custody'
   USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_atelier_state_consistent AFTER INSERT OR UPDATE ON game_character_atelier_state
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_state_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_atelier_state_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_atelier_state_guard() FROM PUBLIC;

-- This secret is generated by PostgreSQL's CSPRNG, never by a client or a public occurrence.
-- Only the trusted Game runtime may read it; no root bytes are part of a receipt or a reply.
CREATE TABLE game_atelier_decision_root (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton),
 secret BYTEA NOT NULL CHECK(octet_length(secret)=32)
);
INSERT INTO game_atelier_decision_root VALUES
 (true,sha256(uuid_send(gen_random_uuid()) || uuid_send(gen_random_uuid()) || uuid_send(gen_random_uuid())));
REVOKE ALL ON game_atelier_decision_root FROM PUBLIC;
GRANT SELECT ON game_atelier_decision_root TO oteryn_game_runtime;

CREATE FUNCTION game_atelier_receipt_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_atelier_state s
   WHERE s.character_id=NEW.character_id AND s.atelier_revision=NEW.after_revision
     AND s.state_json=NEW.outcome_json)
  OR (NEW.kind='GEM_INIT' AND (NEW.before_revision<>0 OR NEW.after_revision<>1
      OR (SELECT count(*) FROM jsonb_object_keys(NEW.outcome_json->'gems'))<>8))
  OR (NEW.kind<>'GEM_INIT' AND NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts p
      WHERE p.character_id=NEW.character_id AND p.after_revision=NEW.before_revision)) THEN
  RAISE EXCEPTION 'Atelier receipt requires its complete state successor and predecessor'
   USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_atelier_receipt_consistent AFTER INSERT ON game_character_atelier_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_receipt_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_atelier_receipt_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_atelier_receipt_guard() FROM PUBLIC;
