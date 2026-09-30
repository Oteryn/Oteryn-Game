-- ACHIEVEMENT step 3 (`OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md` §3 and §5;
-- account-progress decision 2026-09-28 §4.4 and §4.6): durable achievement
-- grant requests and the account fact `AccountAchievement`, written by
-- `durability::account_achievement` inside a granting domain's own fenced
-- Character transaction.
--
-- Scope of this migration:
--   * `game_account_achievement_grant_requests`: one immutable row per
--     (source event, achievement key): the account, the earning Character,
--     the catalogue revision of the achievement and the server time. It is
--     the fact's durable provenance and is never deleted;
--   * `game_account_achievements`: the write-once fact, unique per
--     (account, achievement key). Its only payload is the request it was
--     derived from (composite foreign key), so the first committed insert
--     keeps its provenance and a later request for the same key does not
--     change it;
--   * a deferred guard: a request binds its Character to the Character's
--     account and is consumed into the fact in its own transaction, so a
--     request without a fact, or a fact without a request, never commits.
-- Neither table advances CharacterRevision: a grant request rides on the
-- granting event's transaction and that event's own fence (reward-claim
-- composition decision: the transaction serializes on the Character root row
-- lock). No existing table, function or trigger changes. Points are a read
-- over the facts and a world catalogue and are not stored.

CREATE TABLE game_account_achievement_grant_requests (
    source_kind TEXT NOT NULL CHECK (source_kind ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    source_event_id BYTEA NOT NULL CHECK (octet_length(source_event_id) BETWEEN 1 AND 64),
    achievement_key TEXT NOT NULL
        CHECK (octet_length(achievement_key) <= 160
               AND achievement_key ~ '^oteryn:achievement/[a-z0-9]+(_[a-z0-9]+)*$'),
    achievement_revision TEXT NOT NULL
        CHECK (achievement_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    account_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    earned_at BIGINT NOT NULL CHECK (earned_at >= 0),
    PRIMARY KEY (source_kind, source_event_id, achievement_key),
    UNIQUE (account_id, achievement_key, source_kind, source_event_id)
);

CREATE TABLE game_account_achievements (
    account_id UUID NOT NULL,
    achievement_key TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_event_id BYTEA NOT NULL,
    PRIMARY KEY (account_id, achievement_key),
    FOREIGN KEY (account_id, achievement_key, source_kind, source_event_id)
        REFERENCES game_account_achievement_grant_requests
            (account_id, achievement_key, source_kind, source_event_id)
);

-- Deferred, at commit, for every inserted request: its Character is a live
-- root of its account, and the account holds the fact for its key (derived
-- from this request or from an earlier one).
CREATE FUNCTION game_account_achievement_request_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
            SELECT 1 FROM game_character_roots r
             WHERE r.character_id = NEW.character_id
               AND r.account_id = NEW.account_id
               AND r.lifecycle = 1)
       OR NOT EXISTS (
            SELECT 1 FROM game_account_achievements f
             WHERE f.account_id = NEW.account_id
               AND f.achievement_key = NEW.achievement_key) THEN
        RAISE EXCEPTION 'achievement grant request is not bound to its account or not consumed'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_account_achievement_request_consumed
    AFTER INSERT ON game_account_achievement_grant_requests
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_account_achievement_request_guard();

CREATE TRIGGER game_account_achievement_request_immutable BEFORE UPDATE OR DELETE
    ON game_account_achievement_grant_requests FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_account_achievement_immutable BEFORE UPDATE OR DELETE
    ON game_account_achievements FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_account_achievement_requests_no_truncate BEFORE TRUNCATE
    ON game_account_achievement_grant_requests EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_account_achievements_no_truncate BEFORE TRUNCATE
    ON game_account_achievements EXECUTE FUNCTION game_character_reject_truncate();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_account_achievement_request_guard() '
                   'SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_account_achievement_grant_requests, game_account_achievements FROM PUBLIC;
REVOKE ALL ON FUNCTION game_account_achievement_request_guard() FROM PUBLIC;
-- Runtime: a granting transaction inserts a request and, when the account
-- lacks it, the fact; it never updates or deletes either.
GRANT SELECT, INSERT ON game_account_achievement_grant_requests, game_account_achievements
    TO oteryn_game_runtime;
GRANT SELECT ON game_account_achievement_grant_requests, game_account_achievements
    TO oteryn_game_control;
