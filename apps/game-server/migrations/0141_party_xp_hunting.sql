-- PARTY-XP-1: minimum typed PREY-0 reader state and atomic kill/XP evidence.
-- No list draws, wildcard economy, Task Board or player Prey commands are admitted here.
CREATE TABLE game_character_prey_slots (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 1 AND 3),
    state TEXT NOT NULL CHECK (state IN ('LOCKED', 'SELECTION', 'ACTIVE')),
    creature_key TEXT,
    bonus_type TEXT CHECK (bonus_type IN ('DAMAGE', 'REDUCTION', 'XP', 'LOOT')),
    grade SMALLINT CHECK (grade BETWEEN 1 AND 10),
    bonus_percent SMALLINT CHECK (bonus_percent BETWEEN 0 AND 100),
    hunting_s_left INTEGER NOT NULL CHECK (hunting_s_left BETWEEN 0 AND 7200),
    slot_epoch BIGINT NOT NULL CHECK (slot_epoch > 0),
    PRIMARY KEY(character_id, slot),
    CHECK (state <> 'ACTIVE' OR (creature_key IS NOT NULL AND bonus_type IS NOT NULL
        AND grade IS NOT NULL AND bonus_percent IS NOT NULL)),
    CHECK (state = 'ACTIVE' OR (creature_key IS NULL AND hunting_s_left = 0)),
    CHECK ((bonus_type IS NULL) = (grade IS NULL)),
    CHECK ((bonus_type IS NULL) = (bonus_percent IS NULL)),
    CHECK (creature_key IS NULL OR octet_length(creature_key) BETWEEN 1 AND 256)
);
CREATE UNIQUE INDEX game_character_prey_selected_creature
    ON game_character_prey_slots(character_id, creature_key) WHERE state = 'ACTIVE';
CREATE TABLE game_account_prey_unlocks (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    unlock_kind TEXT NOT NULL CHECK (unlock_kind = 'PREY_SLOT_PERMANENT'),
    target SMALLINT NOT NULL CHECK (target IN (2,3)),
    source TEXT NOT NULL CHECK (source = 'STORE_CLAIM'),
    claiming_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    PRIMARY KEY(account_id, world_id, unlock_kind, target)
);
-- OFFLINE-0 §3 keeps the value in the existing build owner. This module adds only
-- the stamina field read by XP; offline training/pool/settlement remain outside this lane.
ALTER TABLE game_character_build_state ADD COLUMN stamina_minutes INTEGER NOT NULL DEFAULT 2520
    CHECK (stamina_minutes BETWEEN 0 AND 2520);
CREATE TABLE game_party_kill_xp_receipts (
    death_key BYTEA PRIMARY KEY CHECK (octet_length(death_key) = 52),
    principal_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    creature_key TEXT NOT NULL CHECK (octet_length(creature_key) BETWEEN 1 AND 256),
    base_xp BIGINT NOT NULL CHECK (base_xp >= 0),
    death_at_ms BIGINT NOT NULL CHECK (death_at_ms >= 0),
    death_at_us NUMERIC(20,0) NOT NULL CHECK (death_at_us BETWEEN 0 AND 18446744073709551615 AND floor(death_at_us/1000)=death_at_ms),
    principal_count SMALLINT NOT NULL CHECK (principal_count BETWEEN 1 AND 50),
    shared BOOLEAN NOT NULL,
    party_id UUID,
    party_revision NUMERIC(20,0) CHECK (party_revision BETWEEN 1 AND 18446744073709551615),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CHECK ((party_id IS NULL) = (party_revision IS NULL)),
    CHECK (NOT shared OR party_id IS NOT NULL)
);
CREATE TABLE game_party_kill_xp_members (
    death_key BYTEA NOT NULL REFERENCES game_party_kill_xp_receipts(death_key),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    reward_occurrence_id UUID UNIQUE REFERENCES game_character_xp_receipts(reward_occurrence_id),
    shared BOOLEAN NOT NULL,
    party_id UUID,
    party_revision NUMERIC(20,0) CHECK (party_revision BETWEEN 1 AND 18446744073709551615),
    raw_share BIGINT NOT NULL CHECK (raw_share >= 0),
    prey_slot SMALLINT CHECK (prey_slot BETWEEN 1 AND 3),
    prey_epoch BIGINT CHECK (prey_epoch > 0),
    prey_grade SMALLINT CHECK (prey_grade BETWEEN 1 AND 10),
    prey_percent SMALLINT NOT NULL CHECK (prey_percent BETWEEN 0 AND 100),
    stamina_minutes INTEGER NOT NULL CHECK (stamina_minutes BETWEEN 0 AND 2520),
    xp_awarded BIGINT NOT NULL CHECK (xp_awarded >= 0),
    PRIMARY KEY(death_key, character_id),
    CHECK ((party_id IS NULL) = (party_revision IS NULL)),
    CHECK (NOT shared OR party_id IS NOT NULL),
    CHECK ((prey_slot IS NULL) = (prey_grade IS NULL)),
    CHECK ((prey_slot IS NULL) = (prey_epoch IS NULL)),
    CHECK ((xp_awarded = 0) = (reward_occurrence_id IS NULL))
);
CREATE TRIGGER game_party_kill_xp_receipts_immutable BEFORE UPDATE OR DELETE
    ON game_party_kill_xp_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_party_kill_xp_members_immutable BEFORE UPDATE OR DELETE
    ON game_party_kill_xp_members FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
REVOKE ALL ON game_character_prey_slots, game_account_prey_unlocks,
    game_party_kill_xp_receipts, game_party_kill_xp_members FROM PUBLIC;
-- Prey activation and Store unlock writers are allocated separately. This lane reads them only.
GRANT SELECT ON game_character_prey_slots, game_account_prey_unlocks TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_party_kill_xp_receipts, game_party_kill_xp_members TO oteryn_game_runtime;
GRANT SELECT ON game_character_prey_slots, game_account_prey_unlocks,
    game_party_kill_xp_receipts, game_party_kill_xp_members TO oteryn_game_control;

-- Existing build operations keep their original columns. This XP consumer cannot assign
-- stamina; only its accepted seed is available until STAMINA-1 admits its fenced writer.
REVOKE INSERT, UPDATE ON game_character_build_state FROM oteryn_game_runtime;
GRANT INSERT (character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,
    club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,
    distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries),
    UPDATE (character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,
    club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,
    distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries)
ON game_character_build_state TO oteryn_game_runtime;

CREATE FUNCTION game_party_kill_xp_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected SMALLINT; actual INTEGER;
BEGIN
    SELECT principal_count INTO expected FROM game_party_kill_xp_receipts WHERE death_key=NEW.death_key;
    SELECT count(*) INTO actual FROM (
        SELECT 1 FROM game_party_kill_xp_members WHERE death_key=NEW.death_key LIMIT 51
    ) bounded;
    IF expected IS NULL OR actual <> expected OR EXISTS (
        SELECT 1 FROM game_party_kill_xp_members m
        JOIN game_character_xp_receipts x USING(reward_occurrence_id)
        WHERE m.death_key=NEW.death_key
          AND (m.character_id<>x.character_id OR m.xp_awarded<>x.experience_awarded)
    ) THEN
        RAISE EXCEPTION 'party kill XP audit and native receipts disagree' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_party_kill_xp_complete
    AFTER INSERT ON game_party_kill_xp_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_party_kill_xp_complete();
CREATE CONSTRAINT TRIGGER game_party_kill_xp_member_complete
    AFTER INSERT ON game_party_kill_xp_members DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_party_kill_xp_complete();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_party_kill_xp_complete() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_party_kill_xp_complete() FROM PUBLIC;
