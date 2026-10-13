-- PVP-1 (PARTY-PVP-0 §6.2, §8.1, §8.3; candidate PVP-0 §8 slice PVP-1): the four PvP tables.
--
-- Scope of this migration only:
--   * `game_character_pvp_state`: one row per Character and World: the effective skull, the kill,
--     logout and PZ block deadlines, the white skull, the Adventurer forfeiture and the
--     all-source damage-taken bucket total. Deadlines are written ahead and only rise;
--   * `game_character_pvp_ledger`: the victim's damage ledger snapshot (at most 16 full and 64
--     compact rows per victim);
--   * `game_character_unjustified_points` and `game_character_revenge_marks`: death-transaction
--     consequences, written by the death transaction only (PVP-DEATH-1 and later slices);
--   * guards: World, deadline order, ledger caps, no TRUNCATE; grants for the runtime and control
--     roles.
-- No CharacterRevision advance (PARTY-PVP-0 §6.2 composition rule 1 amendment). Nothing here
-- turns PvP on: the go-live gate (PVP-0 §8) is unchanged and no row grants PvP by its absence.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the four
-- tables, their triggers and functions. Nothing else reads them.

CREATE TABLE game_character_pvp_state (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots (character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    -- 0 none, 1 red, 2 black. The effective skull is read lazily against `skull_until` (§6.2).
    skull SMALLINT NOT NULL DEFAULT 0 CHECK (skull IN (0, 1, 2)),
    skull_until TIMESTAMPTZ,
    kill_block_until TIMESTAMPTZ,
    logout_block_until TIMESTAMPTZ,
    pz_block_until TIMESTAMPTZ,
    white_skull_until TIMESTAMPTZ,
    adventurer_forfeited BOOLEAN NOT NULL DEFAULT FALSE,
    -- Per 10 s bucket: all-source damage taken and PvP damage taken (§6.2, §8.3).
    ledger_total_buckets JSONB NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(ledger_total_buckets) = 'array'
               AND jsonb_array_length(ledger_total_buckets) <= 31),
    revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK (revision BETWEEN 1 AND 18446744073709551615),
    CHECK ((skull = 0) = (skull_until IS NULL))
);

CREATE TABLE game_character_pvp_ledger (
    victim_character_id UUID NOT NULL REFERENCES game_character_pvp_state (character_id),
    contributor_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    -- At most 31 entries of {bucket start, damage, assist}; 10 s buckets (§8.3).
    buckets JSONB NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(buckets) = 'array' AND jsonb_array_length(buckets) <= 31),
    last_at TIMESTAMPTZ NOT NULL,
    compact BOOLEAN NOT NULL DEFAULT FALSE,
    -- A compact row keeps only identity, one damage-or-assist flag and `last_at` (§8.3):
    -- 1 damage, 2 assist.
    compact_kind SMALLINT CHECK (compact_kind IN (1, 2)),
    PRIMARY KEY (victim_character_id, contributor_character_id),
    CHECK (victim_character_id <> contributor_character_id),
    CHECK ((compact AND compact_kind IS NOT NULL AND buckets = '[]'::jsonb)
           OR (NOT compact AND compact_kind IS NULL))
);

CREATE TABLE game_character_unjustified_points (
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    death_occurrence_id UUID NOT NULL REFERENCES game_character_death_receipts (death_occurrence_id),
    victim_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    points_milli INTEGER NOT NULL CHECK (points_milli BETWEEN 1 AND 1000),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (character_id, death_occurrence_id),
    CHECK (character_id <> victim_character_id)
);
CREATE INDEX game_character_unjustified_points_age
    ON game_character_unjustified_points (committed_at, character_id);

CREATE TABLE game_character_revenge_marks (
    victim_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    killer_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    death_occurrence_id UUID NOT NULL REFERENCES game_character_death_receipts (death_occurrence_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    -- PARTYPVP0-RL-17: seven days.
    expires_at TIMESTAMPTZ NOT NULL,
    avenged_at TIMESTAMPTZ,
    PRIMARY KEY (victim_character_id, killer_character_id, death_occurrence_id),
    CHECK (victim_character_id <> killer_character_id),
    CHECK (expires_at > created_at AND expires_at <= created_at + INTERVAL '7 days')
);
CREATE INDEX game_character_revenge_marks_expiry
    ON game_character_revenge_marks (expires_at, victim_character_id);
CREATE INDEX game_character_pvp_ledger_age
    ON game_character_pvp_ledger (last_at, victim_character_id);

-- The state row's World is its Character's World (the root stands in for the World table), and a
-- deadline written ahead only rises (§8.1 "never lowered"): the logout, PZ and kill blocks and
-- the white skull may be raised or first set, never lowered or cleared. Each update advances the
-- revision by one.
CREATE FUNCTION game_character_pvp_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NOT EXISTS (SELECT 1 FROM game_character_roots r
                        WHERE r.character_id = NEW.character_id AND r.world_id = NEW.world_id) THEN
            RAISE EXCEPTION 'a PvP state names the World of its Character' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.character_id IS DISTINCT FROM OLD.character_id
       OR NEW.world_id IS DISTINCT FROM OLD.world_id THEN
        RAISE EXCEPTION 'a PvP state keeps its Character and World' USING ERRCODE = '23514';
    END IF;
    IF (OLD.kill_block_until IS NOT NULL
            AND NEW.kill_block_until IS DISTINCT FROM GREATEST(OLD.kill_block_until, NEW.kill_block_until))
       OR (OLD.logout_block_until IS NOT NULL
            AND NEW.logout_block_until IS DISTINCT FROM GREATEST(OLD.logout_block_until, NEW.logout_block_until))
       OR (OLD.pz_block_until IS NOT NULL
            AND NEW.pz_block_until IS DISTINCT FROM GREATEST(OLD.pz_block_until, NEW.pz_block_until))
       OR (OLD.white_skull_until IS NOT NULL
            AND NEW.white_skull_until IS DISTINCT FROM GREATEST(OLD.white_skull_until, NEW.white_skull_until)) THEN
        RAISE EXCEPTION 'a PvP deadline is never lowered or cleared' USING ERRCODE = '23514';
    END IF;
    IF NEW.revision <> OLD.revision + 1 THEN
        RAISE EXCEPTION 'a PvP state advances its revision by one' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- A ledger row's victim and contributor share one World, and the caps hold per victim
-- (`PARTYPVP0-RL-25`: 16 full, `PARTYPVP0-RL-32`: 64 compact). The victim's state row is locked
-- first, so concurrent writers of one victim count in order.
CREATE FUNCTION game_character_pvp_ledger_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_rows BIGINT;
BEGIN
    PERFORM 1 FROM game_character_pvp_state WHERE character_id = NEW.victim_character_id FOR UPDATE;
    IF NOT EXISTS (SELECT 1
                     FROM game_character_pvp_state v
                     JOIN game_character_roots c ON c.world_id = v.world_id
                    WHERE v.character_id = NEW.victim_character_id
                      AND c.character_id = NEW.contributor_character_id) THEN
        RAISE EXCEPTION 'a ledger row names one World' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.victim_character_id IS DISTINCT FROM OLD.victim_character_id
       OR NEW.contributor_character_id IS DISTINCT FROM OLD.contributor_character_id) THEN
        RAISE EXCEPTION 'a ledger row keeps its victim and contributor' USING ERRCODE = '23514';
    END IF;
    SELECT count(*) INTO v_rows FROM game_character_pvp_ledger l
     WHERE l.victim_character_id = NEW.victim_character_id
       AND l.compact = NEW.compact
       AND l.contributor_character_id <> NEW.contributor_character_id;
    IF v_rows >= (CASE WHEN NEW.compact THEN 64 ELSE 16 END) THEN
        RAISE EXCEPTION 'a victim ledger holds at most 16 full and 64 compact rows'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_character_pvp_state_guard BEFORE INSERT OR UPDATE
    ON game_character_pvp_state FOR EACH ROW
    EXECUTE FUNCTION game_character_pvp_state_guard();
CREATE TRIGGER game_character_pvp_ledger_guard BEFORE INSERT OR UPDATE
    ON game_character_pvp_ledger FOR EACH ROW
    EXECUTE FUNCTION game_character_pvp_ledger_guard();
CREATE TRIGGER game_character_pvp_state_no_truncate BEFORE TRUNCATE
    ON game_character_pvp_state EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_pvp_ledger_no_truncate BEFORE TRUNCATE
    ON game_character_pvp_ledger EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_unjustified_points_no_truncate BEFORE TRUNCATE
    ON game_character_unjustified_points EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_revenge_marks_no_truncate BEFORE TRUNCATE
    ON game_character_revenge_marks EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_pvp_state_guard()',
        'game_character_pvp_ledger_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_pvp_state, game_character_pvp_ledger,
    game_character_unjustified_points, game_character_revenge_marks FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_pvp_state_guard(), game_character_pvp_ledger_guard()
FROM PUBLIC;
-- The state row is never deleted. Ledger rows are deleted by the victim's death transaction and
-- the World job; points and marks only by the World job (§6.2).
GRANT SELECT, INSERT, UPDATE ON game_character_pvp_state TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_pvp_ledger TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_unjustified_points,
    game_character_revenge_marks TO oteryn_game_runtime;
GRANT SELECT ON game_character_pvp_state, game_character_pvp_ledger,
    game_character_unjustified_points, game_character_revenge_marks TO oteryn_game_control;
