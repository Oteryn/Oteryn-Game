-- FORGE-1a: the forge dust asset (IMBUE-FORGE-0 §9; architect batch
-- `ARCH-BATCH-TIMED-FORGE-PROF-PACKETS-V1` §1.3, §2.1).
--
-- Dust is a Character balance, the second DUR-03 §18 non-item asset after bank gold, stored as
-- BANK-0 §3 stores bank gold:
--   * `game_character_forge_dust`: one row per Character with `balance` (0 to `dust_limit`),
--     `dust_limit` (100-225, `IMBFORGE0-RL-08`; initially 100) and `last_entry_id`. No row means
--     balance 0 and limit 100; a row with no `last_entry_id` is that zero state and has no entry;
--   * `game_character_forge_dust_entries`: one immutable row per balance change, chained by
--     `previous_entry_id` (the balance row's `last_entry_id` when it was written), with the
--     amount, the lost part of a gain above the limit, and the balance and limit before and
--     after. The kinds are `GAIN`, `SPEND`, `CONVERT` and `LIMIT_RAISE`; FORGE-1a admits only
--     `GAIN` (cause `creature_kill`, reserved for FORGE-CREATURE-1) and `SPEND` (cause
--     `proficiency`, reserved for PROF-SHAPE-1b). `CONVERT` and `LIMIT_RAISE` and the forge
--     causes are FORGE-1b's: nothing here raises `dust_limit`;
--   * guards: the balance row equals the after values of its latest entry, which has no
--     successor; each entry's before values equal its predecessor's after values (by
--     `previous_entry_id`, constant time); one chain per Character.
-- An entry is not a receipt and not a forge operation: it runs inside the caller's transaction
-- under the caller's cause occurrence, TransactionId, receipt and DUR-03 admission, and records
-- them (`cause`, `cause_occurrence_id`, `transaction_id`, `created_xact_id`). One entry per
-- (Character, cause occurrence). No existing table changes.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any entry
-- exists it drops the two tables and the guard functions. After entries exist they are retained
-- evidence of created and destroyed value: a rollback keeps them and may only stop new writes by
-- revoking the INSERT and UPDATE grants.

CREATE TABLE game_character_forge_dust_entries (
    entry_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(entry_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    previous_entry_id UUID REFERENCES game_character_forge_dust_entries(entry_id),
    kind TEXT NOT NULL CHECK (kind IN ('GAIN', 'SPEND', 'CONVERT', 'LIMIT_RAISE')),
    cause TEXT NOT NULL,
    cause_occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(cause_occurrence_id)),
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    -- The requested amount; a gain credits `amount - lost_amount`.
    amount BIGINT NOT NULL CHECK (amount BETWEEN 1 AND 4294967295),
    lost_amount BIGINT NOT NULL CHECK (lost_amount BETWEEN 0 AND amount),
    balance_before BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    -- IMBFORGE0-RL-08
    dust_limit_before SMALLINT NOT NULL CHECK (dust_limit_before BETWEEN 100 AND 225),
    dust_limit_after SMALLINT NOT NULL CHECK (dust_limit_after BETWEEN 100 AND 225),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CONSTRAINT game_character_forge_dust_entry_before_within_limit
        CHECK (balance_before BETWEEN 0 AND dust_limit_before),
    CONSTRAINT game_character_forge_dust_entry_after_within_limit
        CHECK (balance_after BETWEEN 0 AND dust_limit_after),
    -- FORGE-1b adds `CONVERT`, `LIMIT_RAISE` and the forge causes by replacing these two.
    CONSTRAINT game_character_forge_dust_entry_cause_admitted CHECK (
        (kind = 'GAIN' AND cause = 'creature_kill') OR (kind = 'SPEND' AND cause = 'proficiency')),
    -- A gain credits up to the limit and records the lost part (IMBUE-FORGE-0 §9); a spend never
    -- goes below zero; neither changes the limit.
    CONSTRAINT game_character_forge_dust_entry_arithmetic CHECK (CASE kind
        WHEN 'GAIN' THEN dust_limit_after = dust_limit_before
            AND balance_after = balance_before + amount - lost_amount
            AND (lost_amount = 0 OR balance_after = dust_limit_after)
        WHEN 'SPEND' THEN dust_limit_after = dust_limit_before
            AND lost_amount = 0
            AND balance_after = balance_before - amount
        ELSE FALSE
    END),
    -- One successor per entry and one first entry per Character: one linear chain.
    UNIQUE (previous_entry_id),
    UNIQUE (character_id, cause, cause_occurrence_id)
);

CREATE UNIQUE INDEX game_character_forge_dust_entries_first
    ON game_character_forge_dust_entries (character_id) WHERE previous_entry_id IS NULL;

CREATE TABLE game_character_forge_dust (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    balance BIGINT NOT NULL CHECK (balance >= 0),
    dust_limit SMALLINT NOT NULL DEFAULT 100,
    last_entry_id UUID UNIQUE REFERENCES game_character_forge_dust_entries(entry_id),
    -- IMBFORGE0-RL-08
    CONSTRAINT game_character_forge_dust_limit_range CHECK (dust_limit BETWEEN 100 AND 225),
    CONSTRAINT game_character_forge_dust_balance_within_limit CHECK (balance <= dust_limit),
    CONSTRAINT game_character_forge_dust_zero_row
        CHECK (last_entry_id IS NOT NULL OR (balance = 0 AND dust_limit = 100))
);

-- Deferred: the Character's chain and balance row agree at commit. Index probes only: the row,
-- the predecessor, the latest entry and its successor; never the whole chain.
CREATE FUNCTION game_character_forge_dust_chain_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character_id UUID;
    v_row game_character_forge_dust%ROWTYPE;
    v_previous game_character_forge_dust_entries%ROWTYPE;
    v_latest game_character_forge_dust_entries%ROWTYPE;
BEGIN
    IF TG_TABLE_NAME = 'game_character_forge_dust_entries' THEN
        v_character_id := NEW.character_id;
        IF NEW.previous_entry_id IS NULL THEN
            IF (NEW.balance_before, NEW.dust_limit_before) IS DISTINCT FROM (0::BIGINT, 100::SMALLINT) THEN
                RAISE EXCEPTION 'first forge dust entry must start from balance 0 and limit 100'
                    USING ERRCODE = '23514';
            END IF;
        ELSE
            SELECT * INTO v_previous FROM game_character_forge_dust_entries
             WHERE entry_id = NEW.previous_entry_id;
            IF NOT FOUND OR v_previous.character_id <> NEW.character_id
               OR (NEW.balance_before, NEW.dust_limit_before)
                  IS DISTINCT FROM (v_previous.balance_after, v_previous.dust_limit_after) THEN
                RAISE EXCEPTION 'forge dust entry does not follow its previous entry'
                    USING ERRCODE = '23514';
            END IF;
        END IF;
    ELSE
        v_character_id := COALESCE(NEW.character_id, OLD.character_id);
    END IF;

    SELECT * INTO v_row FROM game_character_forge_dust WHERE character_id = v_character_id;
    IF NOT FOUND THEN
        IF TG_TABLE_NAME = 'game_character_forge_dust_entries' THEN
            RAISE EXCEPTION 'forge dust entry has no balance row' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF v_row.last_entry_id IS NULL THEN
        IF EXISTS (SELECT 1 FROM game_character_forge_dust_entries
                    WHERE character_id = v_character_id) THEN
            RAISE EXCEPTION 'forge dust balance row does not name its latest entry'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    SELECT * INTO v_latest FROM game_character_forge_dust_entries
     WHERE entry_id = v_row.last_entry_id;
    IF NOT FOUND OR v_latest.character_id <> v_character_id
       OR (v_row.balance, v_row.dust_limit)
          IS DISTINCT FROM (v_latest.balance_after, v_latest.dust_limit_after)
       OR EXISTS (SELECT 1 FROM game_character_forge_dust_entries
                   WHERE previous_entry_id = v_row.last_entry_id) THEN
        RAISE EXCEPTION 'forge dust balance row does not equal its latest entry'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_forge_dust_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.last_entry_id IS NOT NULL
       AND NEW.last_entry_id IS DISTINCT FROM OLD.last_entry_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'forge dust balance changes only with a new entry and is never deleted'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_character_forge_dust_entry_immutable BEFORE UPDATE OR DELETE
    ON game_character_forge_dust_entries FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_forge_dust_entry_chain_consistent
    AFTER INSERT ON game_character_forge_dust_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_forge_dust_chain_guard();
CREATE TRIGGER game_character_forge_dust_entries_no_truncate BEFORE TRUNCATE
    ON game_character_forge_dust_entries EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_forge_dust_entries_stamp_xact BEFORE INSERT
    ON game_character_forge_dust_entries FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE TRIGGER game_character_forge_dust_row_guard BEFORE UPDATE OR DELETE
    ON game_character_forge_dust FOR EACH ROW
    EXECUTE FUNCTION game_character_forge_dust_row_guard();
CREATE CONSTRAINT TRIGGER game_character_forge_dust_row_chain_consistent
    AFTER INSERT OR UPDATE ON game_character_forge_dust
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_forge_dust_chain_guard();
CREATE TRIGGER game_character_forge_dust_no_truncate BEFORE TRUNCATE
    ON game_character_forge_dust EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_forge_dust_chain_guard()',
        'game_character_forge_dust_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_forge_dust_entries, game_character_forge_dust FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_forge_dust_chain_guard(),
    game_character_forge_dust_row_guard() FROM PUBLIC;
-- As BANK-0 §3: the runtime inserts entries and inserts or updates balance rows; it never
-- deletes either.
GRANT SELECT, INSERT ON game_character_forge_dust_entries TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_forge_dust TO oteryn_game_runtime;
GRANT SELECT ON game_character_forge_dust_entries, game_character_forge_dust TO oteryn_game_control;
