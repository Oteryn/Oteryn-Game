-- CHAR-NAME-1 (D166; owner answers of 2026-09-30 recorded in
-- `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §6.1): the Character display name and its
-- reservation, so `ListCharactersForAccount` can source names.
--
-- Scope of this migration:
--   * `game_character_roots.name`: the current display name under naming policy revision 1
--     (2..29 ASCII letters in words joined by single spaces), and `name_key`, its generated
--     comparison key (ASCII lower case without spaces). The name is immutable here: there is
--     no rename yet, so the revision guard now also holds it equal;
--   * `game_character_name_reservations`: one row per comparison key in one global namespace
--     across every World. Inserting a root reserves its key in the same statement (definer
--     trigger), so a taken name fails the root insert with 23505 whatever the writer. Rows are
--     immutable and never released here: the release after a rename or a terminal deletion
--     (a 30-day hold, owner answer 4b) ships with those operations.
-- Existing Characters are preproduction only (stated assumption): `name` is NOT NULL without a
-- default, so this migration fails closed on a store that already holds a Character, instead of
-- inventing names.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any named
-- Character exists it drops the reservation table, its trigger and the four functions, restores
-- the 0009 guard body and drops both columns. After named Characters exist the names are
-- authority state: a rollback may only stop new writes.

CREATE FUNCTION game_character_is_name(value TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT octet_length(value) BETWEEN 2 AND 29 AND value ~ '^[A-Za-z]+( [A-Za-z]+)*$'
$$;

CREATE FUNCTION game_character_name_key(value TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT lower(replace(value, ' ', ''))
$$;

ALTER TABLE game_character_roots
    ADD COLUMN name TEXT NOT NULL CHECK (game_character_is_name(name)),
    ADD COLUMN name_key TEXT GENERATED ALWAYS AS (game_character_name_key(name)) STORED;

CREATE TABLE game_character_name_reservations (
    name_key TEXT PRIMARY KEY CHECK (name_key ~ '^[a-z]{2,29}$'),
    naming_policy_revision SMALLINT NOT NULL CHECK (naming_policy_revision = 1),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    reserved_at BIGINT NOT NULL CHECK (reserved_at >= 0)
);
CREATE INDEX game_character_name_reservations_character
    ON game_character_name_reservations (character_id);

-- Every root reserves its current name key. SECURITY DEFINER: no writer role holds a direct
-- INSERT on the reservations, so a reservation exists only for a committed root's name.
CREATE FUNCTION game_character_reserve_root_name() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN
    INSERT INTO game_character_name_reservations(name_key, naming_policy_revision, character_id,
        reserved_at)
    VALUES (NEW.name_key, 1, NEW.character_id,
        floor(extract(epoch FROM statement_timestamp()) * 1000)::BIGINT);
    RETURN NULL;
END; $$;
CREATE TRIGGER game_character_reserve_root_name AFTER INSERT ON game_character_roots
    FOR EACH ROW EXECUTE FUNCTION game_character_reserve_root_name();

CREATE TRIGGER game_character_name_reservation_immutable BEFORE UPDATE OR DELETE
    ON game_character_name_reservations FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_name_reservations_no_truncate BEFORE TRUNCATE
    ON game_character_name_reservations EXECUTE FUNCTION game_character_reject_truncate();

-- 0009 guard plus the name: a revision successor never changes it.
CREATE OR REPLACE FUNCTION game_character_root_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.account_id = OLD.account_id
       AND NEW.world_id = OLD.world_id
       AND NEW.lifecycle = OLD.lifecycle
       AND NEW.profile_revision = OLD.profile_revision
       AND NEW.ruleset_revision = OLD.ruleset_revision
       AND NEW.content_revision = OLD.content_revision
       AND NEW.starter_template_revision = OLD.starter_template_revision
       AND NEW.name = OLD.name
       AND NEW.character_revision = OLD.character_revision + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character root permits only one exact revision successor'
        USING ERRCODE = '23514';
END;
$$;

-- CREATE OR REPLACE resets a function's configuration, so the replaced guard
-- gets its fixed search_path again with the new functions.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_is_name(text)',
        'game_character_name_key(text)',
        'game_character_reserve_root_name()',
        'game_character_root_revision_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_name_reservations FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_is_name(text),
    game_character_name_key(text),
    game_character_reserve_root_name()
FROM PUBLIC;
-- Runtime: the bootstrap writer inserts the named root; the definer trigger writes the
-- reservation. The two name functions back a CHECK and the generated key, which PostgreSQL
-- evaluates as the writing role.
GRANT EXECUTE ON FUNCTION game_character_is_name(text), game_character_name_key(text)
    TO oteryn_game_runtime;
GRANT SELECT ON game_character_name_reservations TO oteryn_game_runtime, oteryn_game_control;
