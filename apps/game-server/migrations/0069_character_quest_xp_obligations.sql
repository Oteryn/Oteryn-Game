-- QUEST-XP-1 / migration 0069 (QUEST-GATE-0
-- `reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` §5.5 and
-- the §15 "XP chain" note; QUEST-STATE-0 §13.1): the quest XP obligation.
--
--   * `game_character_quest_xp_obligations`: one row per committed XP-bearing quest transition,
--     inserted only with the quest receipt that names it (same Character, receipt key and pin),
--     by the same physical transaction, for an occurrence no XP receipt names. It keeps the
--     amount (1 to QUESTGATE0-RL-05), a UUIDv7 reward occurrence and the quest's pinned content
--     revision as provenance only. It is never updated, and deleted only with the XP receipt of
--     its occurrence for the same Character and amount. At most QUESTGATE0-RL-10 (16) rows per
--     Character.
--   * An XP receipt of an occurrence that names an obligation commits only with that delete, so
--     a quest occurrence is never awarded without consuming its obligation.
--
-- The XP award keeps its own receipt and revision advance: the shared consistency guard (the
-- 0056 body, eight receipt kinds plus the quest receipt) is unchanged. Rollback after rows exist
-- may only stop new writes: the table and its guards stay (the 0020 pattern).

CREATE TABLE game_character_quest_xp_obligations (
    reward_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(reward_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    transition_key TEXT NOT NULL,
    amount BIGINT NOT NULL CHECK (amount BETWEEN 1 AND 100000000),
    pinned_content_revision TEXT NOT NULL
        CHECK (pinned_content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    created_at BIGINT NOT NULL CHECK (created_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    -- One obligation per quest receipt (proven by the deferred guard below; the receipt is
    -- immutable and never deleted).
    UNIQUE (character_id, cause_id, cause_ordinal, transition_key)
);

-- Inserted rows only; an update never; a delete only with the XP receipt naming the occurrence,
-- for the same Character and amount. That receipt is of the deleting transaction: an obligation
-- is born only for an occurrence without an XP receipt, and an XP receipt of an occurrence that
-- still names an obligation cannot commit.
CREATE FUNCTION game_character_quest_xp_obligation_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_character_xp_receipts r
         WHERE r.reward_occurrence_id = OLD.reward_occurrence_id
           AND r.character_id = OLD.character_id
           AND r.experience_awarded = OLD.amount) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'quest XP obligation transition is not allowed' USING ERRCODE = '23514';
END;
$$;

-- An obligation exists only as the companion of its quest receipt (same Character, key and pin,
-- same physical transaction), for an occurrence no XP receipt names yet, and a Character holds at
-- most QUESTGATE0-RL-10 of them.
CREATE FUNCTION game_character_quest_xp_obligation_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
         WHERE r.character_id = NEW.character_id
           AND r.cause_id = NEW.cause_id
           AND r.cause_ordinal = NEW.cause_ordinal
           AND r.transition_key = NEW.transition_key
           AND r.pinned_content_revision = NEW.pinned_content_revision
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest XP obligation must commit with its quest receipt'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_character_xp_receipts r
         WHERE r.reward_occurrence_id = NEW.reward_occurrence_id) THEN
        RAISE EXCEPTION 'quest XP obligation occurrence is already awarded'
            USING ERRCODE = '23514';
    END IF;
    IF (SELECT count(*) FROM game_character_quest_xp_obligations o
         WHERE o.character_id = NEW.character_id) > 16 THEN
        RAISE EXCEPTION 'quest XP obligations exceed QUESTGATE0-RL-10' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- An XP receipt whose occurrence names a quest XP obligation commits only with its delete.
CREATE FUNCTION game_character_xp_receipt_consumes_quest_obligation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM game_character_quest_xp_obligations o
         WHERE o.reward_occurrence_id = NEW.reward_occurrence_id) THEN
        RAISE EXCEPTION 'quest XP award must consume its obligation' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE INDEX game_character_quest_xp_obligations_by_character
    ON game_character_quest_xp_obligations (character_id);

CREATE TRIGGER game_character_quest_xp_obligations_stamp_xact BEFORE INSERT
    ON game_character_quest_xp_obligations FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_character_quest_xp_obligation_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_quest_xp_obligations FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_xp_obligation_row_guard();
CREATE CONSTRAINT TRIGGER game_character_quest_xp_obligation_proven
    AFTER INSERT ON game_character_quest_xp_obligations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_xp_obligation_proven();
CREATE TRIGGER game_character_quest_xp_obligations_no_truncate BEFORE TRUNCATE
    ON game_character_quest_xp_obligations EXECUTE FUNCTION game_character_reject_truncate();
CREATE CONSTRAINT TRIGGER game_character_xp_receipt_consumes_quest_obligation
    AFTER INSERT ON game_character_xp_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_xp_receipt_consumes_quest_obligation();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_quest_xp_obligation_row_guard()',
        'game_character_quest_xp_obligation_proven()',
        'game_character_xp_receipt_consumes_quest_obligation()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_quest_xp_obligations FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_quest_xp_obligation_row_guard(),
    game_character_quest_xp_obligation_proven(),
    game_character_xp_receipt_consumes_quest_obligation()
    FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_character_quest_xp_obligations TO oteryn_game_runtime;
GRANT SELECT ON game_character_quest_xp_obligations TO oteryn_game_control;
