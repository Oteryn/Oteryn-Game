//! Private schema qualification: historical reads, immutable appearance receipts and projection
//! consistency. No gameplay fence, grant producer or production migration is installed. Fixture
//! INSERTs are storage constraint tests, not gameplay writes or relog qualification.
pub use oteryn_game_server::domain;
#[path = "../src/durability/character_appearance_tests.rs"]
mod character_appearance;

use character_appearance::{
    AccountAppearanceUnlock, read_account_appearance_unlocks, read_selected_appearance,
    verify_character_appearance_integrity,
};
use domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, validate_selection,
};
use domain::{AccountId, CharacterId};
use sqlx::{Connection, Executor};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

// Final-schema proposal, isolated to this database. No production names are claimed installed.
// The CP slot must add root/account constraints, the shared progression guard union and recovery
// verifier, permissions and fenced producers together. This fixture grants no write authority.
const PRIVATE_SCHEMA: &str = r#"
CREATE FUNCTION app_colours_valid(value INTEGER[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT array_ndims(value)=1 AND array_lower(value,1)=1 AND cardinality(value)=4
   AND array_position(value,NULL) IS NULL
   AND 0 <= ALL(value) AND 65535 >= ALL(value)
$$;
CREATE TABLE game_character_appearance_receipts (
 appearance_occurrence_id UUID PRIMARY KEY
   CHECK(get_byte(uuid_send(appearance_occurrence_id),6)>>4=7
     AND get_byte(uuid_send(appearance_occurrence_id),8)&192=128),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding)=33 AND get_byte(command_binding,0)=1),
 character_id UUID NOT NULL,
 original_character_revision NUMERIC(20,0) NOT NULL
   CHECK(original_character_revision BETWEEN 1 AND 18446744073709551614),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision=original_character_revision+1),
 level_before BIGINT NOT NULL CHECK(level_before BETWEEN 1 AND 4294967295),
 level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL CHECK(experience_before>=0),
 experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 outfit_before TEXT,
 colours_before INTEGER[],
 addons_before SMALLINT,
 mount_before TEXT,
 content_before TEXT,
 selection_revision_before NUMERIC(20,0),
 outfit_after TEXT NOT NULL,
 colours_after INTEGER[] NOT NULL CHECK(app_colours_valid(colours_after)),
 addons_after SMALLINT NOT NULL CHECK(addons_after BETWEEN 0 AND 255),
 mount_after TEXT,
 profile_revision TEXT NOT NULL CHECK(octet_length(profile_revision) BETWEEN 1 AND 128),
 ruleset_revision TEXT NOT NULL CHECK(octet_length(ruleset_revision) BETWEEN 1 AND 128),
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 simulation_revision TEXT NOT NULL CHECK(octet_length(simulation_revision) BETWEEN 1 AND 128),
 evidence_revision TEXT NOT NULL CHECK(octet_length(evidence_revision) BETWEEN 1 AND 128),
 declaration_revision TEXT NOT NULL CHECK(octet_length(declaration_revision) BETWEEN 1 AND 128),
 policy_revision TEXT NOT NULL CHECK(octet_length(policy_revision) BETWEEN 1 AND 128),
 reward_revision TEXT NOT NULL CHECK(octet_length(reward_revision) BETWEEN 1 AND 128),
 committed_at BIGINT NOT NULL CHECK(committed_at>=0),
 CHECK((outfit_before IS NULL AND colours_before IS NULL AND addons_before IS NULL
         AND mount_before IS NULL AND content_before IS NULL AND selection_revision_before IS NULL)
    OR (outfit_before IS NOT NULL AND colours_before IS NOT NULL AND addons_before IS NOT NULL
         AND content_before IS NOT NULL AND selection_revision_before IS NOT NULL
         AND app_colours_valid(colours_before) AND addons_before BETWEEN 0 AND 255
         AND octet_length(content_before) BETWEEN 1 AND 128
         AND selection_revision_before BETWEEN 2 AND original_character_revision)),
 CHECK((outfit_before,colours_before,addons_before,mount_before)
     IS DISTINCT FROM (outfit_after,colours_after,addons_after,mount_after)),
 UNIQUE(character_id,original_character_revision),
 UNIQUE(character_id,committed_character_revision),
 UNIQUE(appearance_occurrence_id,character_id,committed_character_revision)
);
CREATE TABLE game_character_appearance (
 character_id UUID PRIMARY KEY,
 outfit_key TEXT NOT NULL,
 colours INTEGER[] NOT NULL CHECK(app_colours_valid(colours)),
 addons SMALLINT NOT NULL CHECK(addons BETWEEN 0 AND 255),
 mount_key TEXT,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision BETWEEN 2 AND 18446744073709551615),
 last_appearance_occurrence_id UUID NOT NULL,
 FOREIGN KEY(last_appearance_occurrence_id,character_id,committed_character_revision)
   REFERENCES game_character_appearance_receipts
    (appearance_occurrence_id,character_id,committed_character_revision)
);
CREATE TABLE game_account_appearance_unlock_receipts (
 grant_occurrence_id UUID PRIMARY KEY,
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 granting_character_id UUID NOT NULL,
 source_kind TEXT NOT NULL CHECK(octet_length(source_kind)>0),
 source_event_id BYTEA NOT NULL CHECK(octet_length(source_event_id)>0),
 UNIQUE(source_kind,source_event_id,unlock_key),
 UNIQUE(grant_occurrence_id,account_id,unlock_key)
);
CREATE TABLE game_account_appearance_unlocks (
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 grant_occurrence_id UUID NOT NULL,
 PRIMARY KEY(account_id,unlock_key),
 FOREIGN KEY(grant_occurrence_id,account_id,unlock_key)
 REFERENCES game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key)
);
CREATE FUNCTION app_fixture_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable appearance receipt or fact' USING ERRCODE='23514'; END;
$$;
CREATE TRIGGER app_selection_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_character_appearance_receipts FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_selection_receipt_no_truncate BEFORE TRUNCATE
 ON game_character_appearance_receipts EXECUTE FUNCTION app_fixture_immutable();
CREATE FUNCTION app_selection_row_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.character_id=OLD.character_id THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'appearance projection cannot be deleted or reassigned' USING ERRCODE='23514';
END;
$$;
CREATE TRIGGER app_selection_row_guard BEFORE UPDATE OR DELETE
 ON game_character_appearance FOR EACH ROW EXECUTE FUNCTION app_selection_row_guard();
CREATE TRIGGER app_selection_no_truncate BEFORE TRUNCATE
 ON game_character_appearance EXECUTE FUNCTION app_fixture_immutable();
CREATE FUNCTION app_selection_consistency() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_character UUID;
BEGIN
 v_character:=NEW.character_id;
 IF EXISTS (
   WITH ordered AS (
     SELECT a.*, lag(outfit_after) OVER w AS previous_outfit,
       lag(colours_after) OVER w AS previous_colours,
       lag(addons_after) OVER w AS previous_addons,
       lag(mount_after) OVER w AS previous_mount,
       lag(content_revision) OVER w AS previous_content,
       lag(committed_character_revision) OVER w AS previous_revision,
       row_number() OVER (ORDER BY committed_character_revision DESC) AS latest
     FROM game_character_appearance_receipts a WHERE character_id=v_character
     WINDOW w AS (ORDER BY committed_character_revision)
   )
   SELECT 1 FROM ordered a WHERE
     (outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before)
       IS DISTINCT FROM
     (previous_outfit,previous_colours,previous_addons,previous_mount,previous_content,previous_revision)
     OR (latest=1 AND NOT EXISTS (
       SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
         AND (p.outfit_key,p.colours,p.addons,p.mount_key,p.content_revision,
              p.committed_character_revision,p.last_appearance_occurrence_id)
           IS NOT DISTINCT FROM
             (a.outfit_after,a.colours_after,a.addons_after,a.mount_after,a.content_revision,
              a.committed_character_revision,a.appearance_occurrence_id)))
   UNION ALL
   SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
     AND NOT EXISTS(SELECT 1 FROM ordered)
 ) THEN
   RAISE EXCEPTION 'appearance projection differs from receipt chain' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER app_selection_consistent
 AFTER INSERT OR UPDATE ON game_character_appearance DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION app_selection_consistency();
CREATE CONSTRAINT TRIGGER app_selection_receipt_consistent
 AFTER INSERT ON game_character_appearance_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION app_selection_consistency();
CREATE TRIGGER app_unlock_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlocks FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_unlock_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlocks EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlock_receipts FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_receipt_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlock_receipts EXECUTE FUNCTION app_fixture_immutable();
"#;

// Complete unnumbered migration CONTENT, authored by APP-1. The CP slot assigns its number
// and installs it after merging current main; it is not installed by this private read fixture.
// The native qualification overlay extracts these exact literal bytes (verified SHA) into its
// temporary support SQL include, then executes them after the real SQLx migrator and before
// native recovery/admission. This is the only authored migration source, not a numbered file.
pub const APP1_MIGRATION_CONTENT: &str = r#"-- APP-1 unnumbered proposal. Native roots, shared progression union, least privilege.

CREATE FUNCTION game_appearance_colours_valid(value INTEGER[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT array_ndims(value)=1 AND array_lower(value,1)=1 AND cardinality(value)=4
   AND array_position(value,NULL) IS NULL
   AND 0 <= ALL(value) AND 65535 >= ALL(value)
$$;
CREATE TABLE game_character_appearance_receipts (
 appearance_occurrence_id UUID PRIMARY KEY
   CHECK(get_byte(uuid_send(appearance_occurrence_id),6)>>4=7
     AND get_byte(uuid_send(appearance_occurrence_id),8)&192=128),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding)=33 AND get_byte(command_binding,0)=1),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 original_character_revision NUMERIC(20,0) NOT NULL
   CHECK(original_character_revision BETWEEN 1 AND 18446744073709551614),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision=original_character_revision+1),
 level_before BIGINT NOT NULL CHECK(level_before BETWEEN 1 AND 4294967295),
 level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL CHECK(experience_before>=0),
 experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 outfit_before TEXT,
 colours_before INTEGER[],
 addons_before SMALLINT,
 mount_before TEXT,
 content_before TEXT,
 selection_revision_before NUMERIC(20,0),
 outfit_after TEXT NOT NULL,
 colours_after INTEGER[] NOT NULL CHECK(game_appearance_colours_valid(colours_after)),
 addons_after SMALLINT NOT NULL CHECK(addons_after BETWEEN 0 AND 255),
 mount_after TEXT,
 profile_revision TEXT NOT NULL CHECK(octet_length(profile_revision) BETWEEN 1 AND 128),
 ruleset_revision TEXT NOT NULL CHECK(octet_length(ruleset_revision) BETWEEN 1 AND 128),
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 simulation_revision TEXT NOT NULL CHECK(octet_length(simulation_revision) BETWEEN 1 AND 128),
 evidence_revision TEXT NOT NULL CHECK(octet_length(evidence_revision) BETWEEN 1 AND 128),
 declaration_revision TEXT NOT NULL CHECK(octet_length(declaration_revision) BETWEEN 1 AND 128),
 policy_revision TEXT NOT NULL CHECK(octet_length(policy_revision) BETWEEN 1 AND 128),
 reward_revision TEXT NOT NULL CHECK(octet_length(reward_revision) BETWEEN 1 AND 128),
 committed_at BIGINT NOT NULL CHECK(committed_at>=0),
 CHECK((outfit_before IS NULL AND colours_before IS NULL AND addons_before IS NULL
         AND mount_before IS NULL AND content_before IS NULL AND selection_revision_before IS NULL)
    OR (outfit_before IS NOT NULL AND colours_before IS NOT NULL AND addons_before IS NOT NULL
         AND content_before IS NOT NULL AND selection_revision_before IS NOT NULL
         AND game_appearance_colours_valid(colours_before) AND addons_before BETWEEN 0 AND 255
         AND octet_length(content_before) BETWEEN 1 AND 128
         AND selection_revision_before BETWEEN 2 AND original_character_revision)),
 CHECK((outfit_before,colours_before,addons_before,mount_before)
     IS DISTINCT FROM (outfit_after,colours_after,addons_after,mount_after)),
 UNIQUE(character_id,original_character_revision),
 UNIQUE(character_id,committed_character_revision),
 UNIQUE(appearance_occurrence_id,character_id,committed_character_revision)
);
CREATE TABLE game_character_appearance (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 outfit_key TEXT NOT NULL,
 colours INTEGER[] NOT NULL CHECK(game_appearance_colours_valid(colours)),
 addons SMALLINT NOT NULL CHECK(addons BETWEEN 0 AND 255),
 mount_key TEXT,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision BETWEEN 2 AND 18446744073709551615),
 last_appearance_occurrence_id UUID NOT NULL,
 FOREIGN KEY(last_appearance_occurrence_id,character_id,committed_character_revision)
   REFERENCES game_character_appearance_receipts
    (appearance_occurrence_id,character_id,committed_character_revision)
);
CREATE TABLE game_account_appearance_unlock_receipts (
 grant_occurrence_id UUID PRIMARY KEY,
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 granting_character_id UUID NOT NULL,
 source_kind TEXT NOT NULL CHECK(octet_length(source_kind)>0),
 source_event_id BYTEA NOT NULL CHECK(octet_length(source_event_id)>0),
 UNIQUE(source_kind,source_event_id,unlock_key),
 UNIQUE(grant_occurrence_id,account_id,unlock_key)
);
CREATE TABLE game_account_appearance_unlocks (
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 grant_occurrence_id UUID NOT NULL,
 PRIMARY KEY(account_id,unlock_key),
 FOREIGN KEY(grant_occurrence_id,account_id,unlock_key)
 REFERENCES game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key)
);
CREATE FUNCTION game_appearance_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable appearance receipt or fact' USING ERRCODE='23514'; END;
$$;
CREATE TRIGGER game_appearance_selection_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_character_appearance_receipts FOR EACH ROW EXECUTE FUNCTION game_appearance_immutable();
CREATE TRIGGER game_appearance_selection_receipt_no_truncate BEFORE TRUNCATE
 ON game_character_appearance_receipts EXECUTE FUNCTION game_appearance_immutable();
CREATE FUNCTION game_appearance_selection_row_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.character_id=OLD.character_id THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'appearance projection cannot be deleted or reassigned' USING ERRCODE='23514';
END;
$$;
CREATE TRIGGER game_appearance_selection_row_guard BEFORE UPDATE OR DELETE
 ON game_character_appearance FOR EACH ROW EXECUTE FUNCTION game_appearance_selection_row_guard();
CREATE TRIGGER game_appearance_selection_no_truncate BEFORE TRUNCATE
 ON game_character_appearance EXECUTE FUNCTION game_appearance_immutable();
CREATE FUNCTION game_appearance_selection_consistency() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_character UUID;
BEGIN
 v_character:=NEW.character_id;
 IF EXISTS (
   WITH ordered AS (
     SELECT a.*, lag(outfit_after) OVER w AS previous_outfit,
       lag(colours_after) OVER w AS previous_colours,
       lag(addons_after) OVER w AS previous_addons,
       lag(mount_after) OVER w AS previous_mount,
       lag(content_revision) OVER w AS previous_content,
       lag(committed_character_revision) OVER w AS previous_revision,
       row_number() OVER (ORDER BY committed_character_revision DESC) AS latest
     FROM game_character_appearance_receipts a WHERE character_id=v_character
     WINDOW w AS (ORDER BY committed_character_revision)
   )
   SELECT 1 FROM ordered a WHERE
     (outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before)
       IS DISTINCT FROM
     (previous_outfit,previous_colours,previous_addons,previous_mount,previous_content,previous_revision)
     OR (latest=1 AND NOT EXISTS (
       SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
         AND (p.outfit_key,p.colours,p.addons,p.mount_key,p.content_revision,
              p.committed_character_revision,p.last_appearance_occurrence_id)
           IS NOT DISTINCT FROM
             (a.outfit_after,a.colours_after,a.addons_after,a.mount_after,a.content_revision,
              a.committed_character_revision,a.appearance_occurrence_id)))
   UNION ALL
   SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
     AND NOT EXISTS(SELECT 1 FROM ordered)
 ) THEN
   RAISE EXCEPTION 'appearance projection differs from receipt chain' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_appearance_selection_consistent
 AFTER INSERT OR UPDATE ON game_character_appearance DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_appearance_selection_consistency();
CREATE CONSTRAINT TRIGGER game_appearance_selection_receipt_consistent
 AFTER INSERT ON game_character_appearance_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_appearance_selection_consistency();
CREATE TRIGGER game_appearance_unlock_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlocks FOR EACH ROW EXECUTE FUNCTION game_appearance_immutable();
CREATE TRIGGER game_appearance_unlock_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlocks EXECUTE FUNCTION game_appearance_immutable();
CREATE TRIGGER game_appearance_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlock_receipts FOR EACH ROW EXECUTE FUNCTION game_appearance_immutable();
CREATE TRIGGER game_appearance_receipt_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlock_receipts EXECUTE FUNCTION game_appearance_immutable();

-- D495 and the 0073 guard union: 0070 (WHEEL-W1) re-issued the shared Character progression
-- guard from the 0056 body, so this branch's 0068 familiar (0033) arms were dropped. This
-- re-issues 0070's body unchanged with every 0068 familiar clause restored; 0070 is untouched.
-- Existing triggers bind the function by name.

CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
    -- The stance, progress and build row triggers also fire on DELETE (rejected
    -- before they run).
    IF TG_OP = 'DELETE' THEN
        v_character := OLD.character_id;
    ELSE
        v_character := NEW.character_id;
    END IF;
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = v_character;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character progression lost its root' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_state FROM game_character_progression_state
     WHERE character_id = v_character;

    IF v_root_revision = 1 THEN
        IF EXISTS (SELECT 1 FROM game_character_xp_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_death_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_appearance_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_appearance WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_kill_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_progress WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_charm_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_monk_state_receipts WHERE character_id = v_character)
           -- CHAR-BUILD-1
           OR EXISTS (SELECT 1 FROM game_character_build_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character)
           -- PROF-1: revision one has no proficiency relation.
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipt_lines WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_familiar_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_familiar_state WHERE character_id = v_character)
           -- QUEST-STATE-1: revision one has no quest receipt, track or state (an obligation
           -- is outside the revision chain).
           OR EXISTS (SELECT 1 FROM game_character_quest_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_tracks WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_states WHERE character_id = v_character)
           -- WHEEL-W1: revision one has no Wheel receipt, state or slot.
           OR EXISTS (SELECT 1 FROM game_character_wheel_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_state WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_slots WHERE character_id = v_character)
           OR (v_state.character_id IS NOT NULL AND v_state.character_revision <> 1)
           -- H-1: only a monk state receipt sets Harmony or a forced Serene time.
           OR (v_state.character_id IS NOT NULL
               AND (v_state.harmony <> 0 OR v_state.serene_forced_remaining_micros <> 0)) THEN
            RAISE EXCEPTION 'initial Character progression is inconsistent' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;

    IF v_state.character_id IS NULL
       OR v_state.character_revision <> v_root_revision
       OR EXISTS (
            WITH chain AS (
                SELECT x.original_character_revision, x.committed_character_revision,
                       x.level_before, x.level_after, x.experience_before, x.experience_after,
                       x.profile_revision, x.ruleset_revision, x.content_revision,
                       x.simulation_revision, x.evidence_revision, x.declaration_revision,
                       x.policy_revision, x.reward_revision
                  FROM game_character_xp_receipts x WHERE x.character_id = v_character
                UNION ALL
                SELECT d.original_character_revision, d.committed_character_revision,
                       d.level_before, d.level_after, d.experience_before, d.experience_after,
                       d.profile_revision, d.ruleset_revision, d.content_revision,
                       d.simulation_revision, d.evidence_revision, d.declaration_revision,
                       d.policy_revision, d.reward_revision
                  FROM game_character_death_receipts d WHERE d.character_id = v_character
                UNION ALL
                SELECT s.original_character_revision, s.committed_character_revision,
                       s.level_before, s.level_after, s.experience_before, s.experience_after,
                       s.profile_revision, s.ruleset_revision, s.content_revision,
                       s.simulation_revision, s.evidence_revision, s.declaration_revision,
                       s.policy_revision, s.reward_revision
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
                UNION ALL
                SELECT a.original_character_revision, a.committed_character_revision,
                       a.level_before, a.level_after, a.experience_before, a.experience_after,
                       a.profile_revision, a.ruleset_revision, a.content_revision,
                       a.simulation_revision, a.evidence_revision, a.declaration_revision,
                       a.policy_revision, a.reward_revision
                  FROM game_character_appearance_receipts a WHERE a.character_id = v_character
                UNION ALL
                -- CHARM-2
                SELECT b.original_character_revision, b.committed_character_revision,
                       b.level_before, b.level_after, b.experience_before, b.experience_after,
                       b.profile_revision, b.ruleset_revision, b.content_revision,
                       b.simulation_revision, b.evidence_revision, b.declaration_revision,
                       b.policy_revision, b.reward_revision
                  FROM game_character_bestiary_kill_receipts b WHERE b.character_id = v_character
                UNION ALL
                -- CHARM-3
                SELECT c.original_character_revision, c.committed_character_revision,
                       c.level_before, c.level_after, c.experience_before, c.experience_after,
                       c.profile_revision, c.ruleset_revision, c.content_revision,
                       c.simulation_revision, c.evidence_revision, c.declaration_revision,
                       c.policy_revision, c.reward_revision
                  FROM game_character_charm_receipts c WHERE c.character_id = v_character
                UNION ALL
                -- H-1
                SELECT m.original_character_revision, m.committed_character_revision,
                       m.level_before, m.level_after, m.experience_before, m.experience_after,
                       m.profile_revision, m.ruleset_revision, m.content_revision,
                       m.simulation_revision, m.evidence_revision, m.declaration_revision,
                       m.policy_revision, m.reward_revision
                  FROM game_character_monk_state_receipts m WHERE m.character_id = v_character
                UNION ALL
                -- CHAR-BUILD-1
                SELECT u.original_character_revision, u.committed_character_revision,
                       u.level_before, u.level_after, u.experience_before, u.experience_after,
                       u.profile_revision, u.ruleset_revision, u.content_revision,
                       u.simulation_revision, u.evidence_revision, u.declaration_revision,
                       u.policy_revision, u.reward_revision
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                -- PROF-1: eighth shared CharacterRevision receipt kind.
                SELECT f.original_character_revision, f.committed_character_revision,
                       f.level_before, f.level_after, f.experience_before, f.experience_after,
                       f.profile_revision, f.ruleset_revision, f.content_revision,
                       f.simulation_revision, f.evidence_revision, f.declaration_revision,
                       f.policy_revision, f.reward_revision
                  FROM game_character_proficiency_receipts f WHERE f.character_id = v_character
                UNION ALL
                -- FAMILIAR-1: changed snapshot, XP/level unchanged.
                SELECT f.original_character_revision, f.committed_character_revision,
                       f.level_before, f.level_after, f.experience_before, f.experience_after,
                       f.profile_revision, f.ruleset_revision, f.content_revision,
                       f.simulation_revision, f.evidence_revision, f.declaration_revision,
                       f.policy_revision, f.reward_revision
                  FROM game_character_familiar_receipts f WHERE f.character_id = v_character
                UNION ALL
                -- QUEST-STATE-1: ninth shared CharacterRevision receipt kind.
                SELECT q.original_character_revision, q.committed_character_revision,
                       q.level_before, q.level_after, q.experience_before, q.experience_after,
                       q.profile_revision, q.ruleset_revision, q.content_revision,
                       q.simulation_revision, q.evidence_revision, q.declaration_revision,
                       q.policy_revision, q.reward_revision
                  FROM game_character_quest_receipts q WHERE q.character_id = v_character
                UNION ALL
                -- WHEEL-W1: tenth shared CharacterRevision receipt kind.
                SELECT w.original_character_revision, w.committed_character_revision,
                       w.level_before, w.level_after, w.experience_before, w.experience_after,
                       w.profile_revision, w.ruleset_revision, w.content_revision,
                       w.simulation_revision, w.evidence_revision, w.declaration_revision,
                       w.policy_revision, w.reward_revision
                  FROM game_character_wheel_receipts w WHERE w.character_id = v_character
            )
            SELECT 1 FROM chain
            HAVING count(*)::numeric <> v_root_revision - 1
                OR count(DISTINCT committed_character_revision)::numeric <> v_root_revision - 1
            UNION ALL
            SELECT 1 WHERE NOT EXISTS (
                SELECT 1 FROM chain c
                 WHERE c.committed_character_revision = v_root_revision
                   AND c.level_after = v_state.level
                   AND c.experience_after = v_state.total_experience
                   AND c.profile_revision = v_state.profile_revision
                   AND c.ruleset_revision = v_state.ruleset_revision
                   AND c.content_revision = v_state.content_revision
                   AND c.simulation_revision = v_state.simulation_revision
                   AND c.evidence_revision = v_state.evidence_revision
                   AND c.declaration_revision = v_state.declaration_revision
                   AND c.policy_revision = v_state.policy_revision
                   AND c.reward_revision = v_state.reward_revision)
            UNION ALL
            SELECT 1 FROM chain x
             LEFT JOIN chain p ON p.committed_character_revision = x.original_character_revision
             WHERE x.committed_character_revision > v_root_revision
                OR (x.original_character_revision <> 1 AND p.committed_character_revision IS NULL)
                OR (p.committed_character_revision IS NOT NULL AND
                    (p.experience_after <> x.experience_before
                     OR p.level_after <> x.level_before))) THEN
        RAISE EXCEPTION 'Character progression revision/receipt chain is inconsistent'
            USING ERRCODE = '23514';
    END IF;

    IF EXISTS (
            WITH stance_chain AS (
                SELECT s.stance_occurrence_id AS occurrence_id, s.committed_character_revision,
                       s.stance_before, s.stance_after
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
                UNION ALL
                -- CHAR-BUILD-1 (A13 §4.2 "Stance fields"): a vocation change that prunes the
                -- stance is a stance transition.
                SELECT u.build_occurrence_id, u.committed_character_revision,
                       u.stance_before, u.stance_after
                  FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.stance_before IS DISTINCT FROM u.stance_after
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER w AS position,
                       lag(c.stance_after) OVER w AS previous_after,
                       count(*) OVER () AS transitions
                  FROM stance_chain c
                WINDOW w AS (ORDER BY c.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.stance_before IS DISTINCT FROM o.previous_after
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_stance r
                     WHERE r.character_id = v_character
                       AND r.stance_key IS NOT DISTINCT FROM o.stance_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_stance_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_stance r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM stance_chain)) THEN
        RAISE EXCEPTION 'Character stance slot is inconsistent with its stance receipts'
            USING ERRCODE = '23514';
    END IF;

    -- FAMILIAR-1 projection must remain coherent even when another Character writer fires
    -- this shared guard; snapshots do not replace the standard stance or build chains.
    IF EXISTS (
        WITH ordered AS (
            SELECT f.*, lag(f.state_after,1,
                '{"selected_look":0,"granted_looks":[],"saved_expiry_unix":0,"last_logout_unix":0,"lifecycle_epoch":0,"familiar_definition":null,"familiar_revision":null,"profile_revision":"none"}'::jsonb)
                OVER (ORDER BY f.committed_character_revision) AS previous,
                row_number() OVER (ORDER BY f.committed_character_revision DESC) AS latest
            FROM game_character_familiar_receipts f WHERE f.character_id=v_character
        ) SELECT 1 FROM ordered f
          WHERE f.state_before <> f.previous
             OR (f.latest=1 AND NOT EXISTS (SELECT 1 FROM game_character_familiar_state s
                 WHERE s.character_id=v_character AND s.state=f.state_after
                   AND s.committed_character_revision=f.committed_character_revision
                   AND s.last_familiar_occurrence_id=f.familiar_occurrence_id))
        UNION ALL SELECT 1 FROM game_character_familiar_state s
         WHERE s.character_id=v_character AND NOT EXISTS(SELECT 1 FROM ordered)
    ) THEN
        RAISE EXCEPTION 'Character familiar snapshot differs from its receipt chain' USING ERRCODE='23514';
    END IF;

    -- CHARM-2: the per-race kill chain and the progress rows.
    IF EXISTS (
            WITH ordered AS (
                SELECT b.bestiary_occurrence_id, b.race_key, b.committed_character_revision,
                       b.kill_count_before, b.kill_count_after,
                       coalesce(lag(b.kill_count_after) OVER w, 0) AS previous_after,
                       row_number() OVER w AS position,
                       count(*) OVER (PARTITION BY b.race_key) AS kills
                  FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                WINDOW w AS (PARTITION BY b.race_key ORDER BY b.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.kill_count_before <> o.previous_after
                OR (o.position = o.kills AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_progress r
                     WHERE r.character_id = v_character
                       AND r.race_key = o.race_key
                       AND r.kill_count = o.kill_count_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_bestiary_occurrence_id = o.bestiary_occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_bestiary_progress r
             WHERE r.character_id = v_character
               AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_kill_receipts b
                     WHERE b.character_id = v_character AND b.race_key = r.race_key)) THEN
        RAISE EXCEPTION 'Character Bestiary progress is inconsistent with its kill receipts'
            USING ERRCODE = '23514';
    END IF;

    -- CHAR-BUILD-1 (A13 §4.2 "Build chain" and "Row guard", SKILLS-0 §3.2): build receipts and
    -- death receipts with build fields, ordered by revision. Each `before` equals the previous
    -- `after`, or the seed for the first; the build row equals the latest one (values, revision,
    -- occurrence) and is absent when there is none. So a row-only write, a receipt without its
    -- row update, and any other kind that changes the row all fail.
    IF EXISTS (
            WITH build_chain AS (
                SELECT u.build_occurrence_id AS occurrence_id, u.committed_character_revision,
                       u.vocation_before, u.magic_level_before, u.mana_spent_before,
                       u.fist_level_before, u.fist_tries_before, u.club_level_before,
                       u.club_tries_before, u.sword_level_before, u.sword_tries_before,
                       u.axe_level_before, u.axe_tries_before, u.distance_level_before,
                       u.distance_tries_before, u.shielding_level_before,
                       u.shielding_tries_before, u.fishing_level_before, u.fishing_tries_before,
                       u.vocation_after, u.magic_level_after, u.mana_spent_after,
                       u.fist_level_after, u.fist_tries_after, u.club_level_after,
                       u.club_tries_after, u.sword_level_after, u.sword_tries_after,
                       u.axe_level_after, u.axe_tries_after, u.distance_level_after,
                       u.distance_tries_after, u.shielding_level_after, u.shielding_tries_after,
                       u.fishing_level_after, u.fishing_tries_after
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                SELECT d.death_occurrence_id, d.committed_character_revision,
                       d.vocation, d.magic_level_before, d.mana_spent_before,
                       d.fist_level_before, d.fist_tries_before, d.club_level_before,
                       d.club_tries_before, d.sword_level_before, d.sword_tries_before,
                       d.axe_level_before, d.axe_tries_before, d.distance_level_before,
                       d.distance_tries_before, d.shielding_level_before,
                       d.shielding_tries_before, d.fishing_level_before, d.fishing_tries_before,
                       d.vocation, d.magic_level_after, d.mana_spent_after,
                       d.fist_level_after, d.fist_tries_after, d.club_level_after,
                       d.club_tries_after, d.sword_level_after, d.sword_tries_after,
                       d.axe_level_after, d.axe_tries_after, d.distance_level_after,
                       d.distance_tries_after, d.shielding_level_after, d.shielding_tries_after,
                       d.fishing_level_after, d.fishing_tries_after
                  FROM game_character_death_receipts d
                 WHERE d.character_id = v_character AND d.vocation IS NOT NULL
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER (ORDER BY c.committed_character_revision) AS position,
                       count(*) OVER () AS transitions
                  FROM build_chain c
            )
            SELECT 1 FROM ordered o
              LEFT JOIN ordered p ON p.position = o.position - 1
             WHERE (p.position IS NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM ('none', 0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0,
                                          10, 0))
                OR (p.position IS NOT NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM (p.vocation_after, p.magic_level_after,
                         p.mana_spent_after, p.fist_level_after, p.fist_tries_after,
                         p.club_level_after, p.club_tries_after, p.sword_level_after,
                         p.sword_tries_after, p.axe_level_after, p.axe_tries_after,
                         p.distance_level_after, p.distance_tries_after,
                         p.shielding_level_after, p.shielding_tries_after,
                         p.fishing_level_after, p.fishing_tries_after))
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_build_state r
                     WHERE r.character_id = v_character
                       AND (r.vocation, r.magic_level, r.mana_spent, r.fist_level, r.fist_tries,
                            r.club_level, r.club_tries, r.sword_level, r.sword_tries,
                            r.axe_level, r.axe_tries, r.distance_level, r.distance_tries,
                            r.shielding_level, r.shielding_tries, r.fishing_level,
                            r.fishing_tries)
                         = (o.vocation_after, o.magic_level_after, o.mana_spent_after,
                            o.fist_level_after, o.fist_tries_after, o.club_level_after,
                            o.club_tries_after, o.sword_level_after, o.sword_tries_after,
                            o.axe_level_after, o.axe_tries_after, o.distance_level_after,
                            o.distance_tries_after, o.shielding_level_after,
                            o.shielding_tries_after, o.fishing_level_after,
                            o.fishing_tries_after)
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_build_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_build_state r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM build_chain)) THEN
        RAISE EXCEPTION 'Character build state is inconsistent with its build receipts'
            USING ERRCODE = '23514';
    END IF;

    -- WHEEL-W1 (WHEEL-0 §4 "Guards"): the Wheel full-chain, ruleset-chain, capacity and
    -- row-tip arm.
    PERFORM game_character_wheel_consistency(v_character);

    IF TG_TABLE_NAME = 'game_character_progression_state' AND TG_OP = 'UPDATE' THEN
        IF NOT EXISTS (
                SELECT 1 FROM game_character_xp_receipts x
                 WHERE x.character_id = v_character
                   AND x.original_character_revision = OLD.character_revision
                   AND x.committed_character_revision = NEW.character_revision
                   AND x.level_before = OLD.level AND x.level_after = NEW.level
                   AND x.experience_before = OLD.total_experience
                   AND x.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision
                   AND d.level_before = OLD.level AND d.level_after = NEW.level
                   AND d.experience_before = OLD.total_experience
                   AND d.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_stance_receipts s
                 WHERE s.character_id = v_character
                   AND s.original_character_revision = OLD.character_revision
                   AND s.committed_character_revision = NEW.character_revision
                   AND s.level_before = OLD.level AND s.level_after = NEW.level
                   AND s.experience_before = OLD.total_experience
                   AND s.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_appearance_receipts a
                 WHERE a.character_id = v_character
                   AND a.original_character_revision = OLD.character_revision
                   AND a.committed_character_revision = NEW.character_revision
                   AND a.level_before = OLD.level AND a.level_after = NEW.level
                   AND a.experience_before = OLD.total_experience
                   AND a.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-2
                SELECT 1 FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                   AND b.original_character_revision = OLD.character_revision
                   AND b.committed_character_revision = NEW.character_revision
                   AND b.level_before = OLD.level AND b.level_after = NEW.level
                   AND b.experience_before = OLD.total_experience
                   AND b.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-3
                SELECT 1 FROM game_character_charm_receipts c
                 WHERE c.character_id = v_character
                   AND c.original_character_revision = OLD.character_revision
                   AND c.committed_character_revision = NEW.character_revision
                   AND c.level_before = OLD.level AND c.level_after = NEW.level
                   AND c.experience_before = OLD.total_experience
                   AND c.experience_after = NEW.total_experience
                UNION ALL
                -- H-1
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision
                   AND m.level_before = OLD.level AND m.level_after = NEW.level
                   AND m.experience_before = OLD.total_experience
                   AND m.experience_after = NEW.total_experience
                   AND m.harmony_before = OLD.harmony AND m.harmony_after = NEW.harmony
                   AND m.serene_forced_remaining_micros_before = OLD.serene_forced_remaining_micros
                   AND m.serene_forced_remaining_micros_after = NEW.serene_forced_remaining_micros
                UNION ALL
                -- PROF-1: a proficiency successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_proficiency_receipts f
                 WHERE f.character_id = v_character
                   AND f.original_character_revision = OLD.character_revision
                   AND f.committed_character_revision = NEW.character_revision
                   AND f.level_before = OLD.level AND f.level_after = NEW.level
                   AND f.experience_before = OLD.total_experience
                   AND f.experience_after = NEW.total_experience
                UNION ALL
                -- CHAR-BUILD-1 (#1271 F3)
                SELECT 1 FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.original_character_revision = OLD.character_revision
                   AND u.committed_character_revision = NEW.character_revision
                   AND u.level_before = OLD.level AND u.level_after = NEW.level
                   AND u.experience_before = OLD.total_experience
                   AND u.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_familiar_receipts f
                 WHERE f.character_id = v_character
                   AND f.original_character_revision = OLD.character_revision
                   AND f.committed_character_revision = NEW.character_revision
                   AND f.level_before = OLD.level AND f.level_after = NEW.level
                   AND f.experience_before = OLD.total_experience
                   AND f.experience_after = NEW.total_experience
                UNION ALL
                -- QUEST-STATE-1: a quest successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_quest_receipts q
                 WHERE q.character_id = v_character
                   AND q.original_character_revision = OLD.character_revision
                   AND q.committed_character_revision = NEW.character_revision
                   AND q.level_before = OLD.level AND q.level_after = NEW.level
                   AND q.experience_before = OLD.total_experience
                   AND q.experience_after = NEW.total_experience
                UNION ALL
                -- WHEEL-W1: a Wheel successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_wheel_receipts w
                 WHERE w.character_id = v_character
                   AND w.original_character_revision = OLD.character_revision
                   AND w.committed_character_revision = NEW.character_revision
                   AND w.level_before = OLD.level AND w.level_after = NEW.level
                   AND w.experience_before = OLD.total_experience
                   AND w.experience_after = NEW.total_experience)
                THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
        -- H-1: a death transition empties Harmony and the forced Serene time; any other
        -- transition but a monk state receipt keeps both unchanged.
        IF EXISTS (
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision) THEN
            IF NEW.harmony <> 0 OR NEW.serene_forced_remaining_micros <> 0 THEN
                RAISE EXCEPTION 'a Character death must empty Harmony and the forced Serene time'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF (NEW.harmony <> OLD.harmony
               OR NEW.serene_forced_remaining_micros <> OLD.serene_forced_remaining_micros)
              AND NOT EXISTS (
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision) THEN
            RAISE EXCEPTION 'Harmony and the forced Serene time change only with a monk state receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- CREATE OR REPLACE clears function-level settings; restore the 0070 search_path pin.
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_progression_consistency_guard() SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON FUNCTION game_character_progression_consistency_guard() FROM PUBLIC;


CREATE CONSTRAINT TRIGGER game_appearance_receipt_progression_consistent
 AFTER INSERT ON game_character_appearance_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_appearance_progression_consistent
 AFTER INSERT OR UPDATE ON game_character_appearance DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
DO $$ DECLARE v_function TEXT; BEGIN
 FOREACH v_function IN ARRAY ARRAY[
  'game_appearance_colours_valid(integer[])', 'game_appearance_immutable()',
  'game_appearance_selection_row_guard()', 'game_appearance_selection_consistency()',
  'game_character_progression_consistency_guard()'
 ] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',v_function,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',v_function);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_appearance_colours_valid(integer[]) TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON game_character_appearance,game_character_appearance_receipts,
 game_account_appearance_unlocks,game_account_appearance_unlock_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_appearance_receipts,
 game_account_appearance_unlocks,game_account_appearance_unlock_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_appearance TO oteryn_game_runtime;
GRANT SELECT ON game_character_appearance,game_character_appearance_receipts,
 game_account_appearance_unlocks,game_account_appearance_unlock_receipts TO oteryn_game_control;
"#;

// Test-only catalogue and explicit unlock-key mapping. APP-2 owns the actual mapping and
// compatibility resolver; revision equality below is a fixture policy, not a product rule.
struct Content;
impl AppearanceContent<String> for Content {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        matches!(key.as_str(), "starter" | "earned" | "store").then_some(2)
    }
    fn outfit_is_starter(&self, key: &String, _: Sex) -> bool {
        key == "starter"
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, key: &String) -> bool {
        key == "horse"
    }
}
struct FixtureEarned<'a>(&'a [AccountAppearanceUnlock]);
impl FixtureEarned<'_> {
    fn fact(&self, key: &str) -> UnlockFact {
        match self.0.iter().find(|fact| fact.unlock_key == key) {
            None => UnlockFact::Absent,
            Some(fact) if fact.content_revision == "content-1" => UnlockFact::Compatible,
            Some(_) => UnlockFact::Incompatible,
        }
    }
}
impl AccountUnlocks<String> for FixtureEarned<'_> {
    fn outfit(&self, key: &String) -> UnlockFact {
        self.fact(key)
    }
    fn addon(&self, key: &String, addon: u8) -> UnlockFact {
        if key == "earned" && addon == 1 {
            self.fact("fixture-addon-one")
        } else {
            UnlockFact::Absent
        }
    }
    fn mount(&self, key: &String) -> UnlockFact {
        self.fact(key)
    }
}

// A successor may skip global revisions consumed by other event families. It must still name
// the exact prior appearance selection and its provenance. The fixture tests this chain only;
// the CP slot additionally binds every global revision through the 0073 union.
const SUCCESSOR_INSERT: &str = "INSERT INTO game_character_appearance_receipts \
 SELECT encode($1,'hex')::uuid,command_binding,character_id,2,3,level_after,level_after,\
 experience_after,experience_after,outfit_after,colours_after,addons_after,mount_after,\
 content_revision,committed_character_revision,'starter',ARRAY[0,0,0,0],0,NULL,\
 profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,\
 declaration_revision,policy_revision,reward_revision,committed_at \
 FROM game_character_appearance_receipts WHERE appearance_occurrence_id=encode($2,'hex')::uuid";

async fn receipt_constraints(connection: &mut sqlx::PgConnection) -> TestResult {
    // Every failure uses an otherwise valid binding and rolls back the WHOLE transaction.
    // This is storage atomicity, not a claim that the missing native gameplay fence ran.
    for (old, replacement, label) in [
        (
            "character_id,2,3",
            "character_id,2,4",
            "revision must advance exactly once",
        ),
        (
            "level_after,level_after",
            "level_after,level_after+1",
            "level unchanged",
        ),
        (
            "experience_after,experience_after",
            "experience_after,experience_after+1",
            "experience unchanged",
        ),
        (
            "'starter',ARRAY[0,0,0,0],0,NULL",
            "outfit_after,colours_after,addons_after,mount_after",
            "no cosmetic no-op",
        ),
        (
            "content_revision,committed_character_revision,'starter'",
            "NULL,committed_character_revision,'starter'",
            "complete optional before",
        ),
        ("ARRAY[0,0,0,0]", "ARRAY[0,0,0,NULL]", "non-null colours"),
        (
            "ARRAY[0,0,0,0]",
            "ARRAY[0,0,0,65536]",
            "unsigned-16 colours",
        ),
        (
            "outfit_after,colours_after,addons_after,mount_after",
            "'substituted',colours_after,addons_after,mount_after",
            "exact prior selection",
        ),
        (
            "content_revision,committed_character_revision,'starter'",
            "'substituted',committed_character_revision,'starter'",
            "exact prior provenance",
        ),
    ] {
        assert!(SUCCESSOR_INSERT.contains(old));
        let mut tx = connection.begin().await?;
        let sql = SUCCESSOR_INSERT.replacen(old, replacement, 1);
        let result = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id(71).as_slice())
            .bind(id(70).as_slice())
            .execute(&mut *tx)
            .await;
        let error = match result {
            Err(error) => error,
            Ok(_) => {
                // A deferred check must see the proposed projection as well, so the negative
                // case cannot pass merely because the successor forgot its projection update.
                sqlx::query("UPDATE game_character_appearance SET outfit_key='starter',colours=ARRAY[0,0,0,0],addons=0,mount_key=NULL,committed_character_revision=3,last_appearance_occurrence_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).execute(&mut *tx).await?;
                match sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                    .execute(&mut *tx)
                    .await
                {
                    Err(error) => error,
                    Ok(_) => return Err(format!("accepted invalid receipt: {label}").into()),
                }
            }
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514"),
            "wrong refusal: {label}"
        );
        tx.rollback().await?;
        let revision: String = sqlx::query_scalar(
            "SELECT committed_character_revision::text FROM game_character_appearance",
        )
        .fetch_one(&mut *connection)
        .await?;
        assert_eq!(revision, "2", "rollback changed projection: {label}");
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
                .fetch_one(&mut *connection)
                .await?;
        assert_eq!(receipts, 1, "rollback retained a receipt: {label}");
    }
    for query in [
        "UPDATE game_character_appearance_receipts SET content_revision='changed'",
        "DELETE FROM game_character_appearance_receipts",
        "TRUNCATE game_character_appearance_receipts CASCADE",
        "DELETE FROM game_character_appearance",
        "TRUNCATE game_character_appearance",
        "UPDATE game_character_appearance SET outfit_key='receipt-free'",
    ] {
        let error = match sqlx::query(query).execute(&mut *connection).await {
            Err(error) => error,
            Ok(_) => return Err(format!("accepted immutable mutation: {query}").into()),
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
    }
    // A real successor of the private chain commits, keeps the historical receipt, and reloads.
    let mut tx = connection.begin().await?;
    sqlx::query(SUCCESSOR_INSERT)
        .bind(id(71).as_slice())
        .bind(id(70).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_appearance SET outfit_key='starter',colours=ARRAY[0,0,0,0],addons=0,mount_key=NULL,committed_character_revision=3,last_appearance_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
            .fetch_one(&mut *connection)
            .await?;
    assert_eq!(receipts, 2);
    Ok(())
}

async fn qualify(connection: &mut sqlx::PgConnection) -> TestResult {
    assert!(
        read_selected_appearance(connection, CharacterId::from_bytes(id(41))?)
            .await
            .is_err()
    );
    connection.execute(sqlx::raw_sql(PRIVATE_SCHEMA)).await?;
    let character = CharacterId::from_bytes(id(41))?;
    let other_character = CharacterId::from_bytes(id(42))?;
    let account = AccountId::from_bytes(id(40))?;
    let other_account = AccountId::from_bytes(id(39))?;
    assert!(
        read_selected_appearance(connection, character)
            .await?
            .is_none()
    );
    let mut selection_tx = connection.begin().await?;
    sqlx::query("INSERT INTO game_character_appearance_receipts(appearance_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,outfit_after,colours_after,addons_after,mount_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) VALUES(encode($2,'hex')::uuid,decode('01' || repeat('00',32),'hex'),encode($1,'hex')::uuid,1,2,1,1,0,0,'earned',ARRAY[0,1,2,132],1,'horse','profile-1','rules-1','old-world','sim-1','evidence-1','decl-1','policy-1','reward-1',1)")
        .bind(character.as_bytes().as_slice()).bind(id(70).as_slice()).execute(&mut *selection_tx).await?;
    sqlx::query("INSERT INTO game_character_appearance VALUES(encode($1,'hex')::uuid,'earned',ARRAY[0,1,2,132],1,'horse','old-world',2,encode($2,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(id(70).as_slice()).execute(&mut *selection_tx).await?;
    selection_tx.commit().await?;
    // These are fixture inserts, not an earned-grant or selected-appearance writer.
    for (seed, key, revision) in [
        (60, "earned", "content-1"),
        (61, "fixture-addon-one", "content-1"),
        (62, "horse", "incompatible"),
    ] {
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,'quest',$1)")
            .bind(id(seed).as_slice()).bind(account.as_bytes().as_slice()).bind(key).bind(revision).bind(character.as_bytes().as_slice())
            .execute(&mut *connection).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) ON CONFLICT(account_id,unlock_key) DO NOTHING")
            .bind(account.as_bytes().as_slice()).bind(key).bind(id(seed).as_slice()).execute(&mut *connection).await?;
    }
    let original = read_selected_appearance(connection, character)
        .await?
        .ok_or("missing selection")?;
    assert!(
        read_selected_appearance(connection, other_character)
            .await?
            .is_none()
    );
    assert!(
        read_account_appearance_unlocks(connection, other_account)
            .await?
            .is_empty()
    );
    let facts = read_account_appearance_unlocks(connection, account).await?;
    assert_eq!(facts.len(), 3);
    assert_eq!(facts[0].granting_character_id, character);
    assert_eq!(facts[0].source_kind, "quest");
    assert!(!facts[0].source_event_id.is_empty());
    assert_eq!(
        validate_selection(
            &original.selection,
            Sex::Male,
            &Content,
            &FixtureEarned(&facts)
        ),
        Err(AppearanceRejection::MountNotAuthorized)
    );
    let mut valid = original.selection.clone();
    valid.mount_key = None;
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
        Ok(())
    );
    let missing_addon: Vec<_> = facts
        .iter()
        .filter(|fact| fact.unlock_key != "fixture-addon-one")
        .cloned()
        .collect();
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&missing_addon)),
        Err(AppearanceRejection::AddonNotAuthorized { addon: 1 })
    );
    valid.outfit_key = "store".into();
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
        Err(AppearanceRejection::OutfitNotAuthorized)
    );

    // Another granting character/revision cannot replace the first fact/provenance.
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','replacement',encode($3,'hex')::uuid,'npc',$1)")
        .bind(id(63).as_slice()).bind(account.as_bytes().as_slice()).bind(other_character.as_bytes().as_slice()).execute(&mut *connection).await?;
    let replay = sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'earned',encode($2,'hex')::uuid) ON CONFLICT(account_id,unlock_key) DO NOTHING")
        .bind(account.as_bytes().as_slice()).bind(id(63).as_slice()).execute(&mut *connection).await?;
    assert_eq!(replay.rows_affected(), 0);
    assert_eq!(
        read_account_appearance_unlocks(connection, account).await?,
        facts
    );
    let conflicting_source = sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','substitution',encode($3,'hex')::uuid,'quest',$4)")
        .bind(id(64).as_slice()).bind(other_account.as_bytes().as_slice()).bind(other_character.as_bytes().as_slice()).bind(id(60).as_slice())
        .execute(&mut *connection).await;
    assert!(
        conflicting_source.is_err(),
        "source-event identity must not be rebound"
    );
    for query in [
        "UPDATE game_account_appearance_unlocks SET unlock_key='changed'",
        "DELETE FROM game_account_appearance_unlocks",
        "TRUNCATE game_account_appearance_unlocks",
        "UPDATE game_account_appearance_unlock_receipts SET content_revision='changed'",
        "DELETE FROM game_account_appearance_unlock_receipts",
        "TRUNCATE game_account_appearance_unlock_receipts CASCADE",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000001','bad',ARRAY[0,0,0],0,NULL,'ok',2,'00000000-0000-7000-8000-000000000070')",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000002','bad',ARRAY[0,0,0,NULL],0,NULL,'ok',2,'00000000-0000-7000-8000-000000000070')",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000003','bad',ARRAY[0,0,0,0],0,NULL,repeat('é',65),2,'00000000-0000-7000-8000-000000000070')",
    ] {
        assert!(
            sqlx::query(query).execute(&mut *connection).await.is_err(),
            "fixture schema accepted {query}"
        );
    }
    assert_eq!(
        read_selected_appearance(connection, character).await?,
        Some(original)
    );
    receipt_constraints(connection).await?;
    verify_character_appearance_integrity(connection).await?;
    // Recovery checks must reject corruption even if a privileged operator bypassed triggers.
    // Each single-field mutation is rolled back before moving to the next invariant.
    for query in [
        "UPDATE game_character_appearance SET outfit_key='corrupt'",
        "UPDATE game_character_appearance_receipts SET content_before='corrupt' WHERE committed_character_revision=3",
        "UPDATE game_character_appearance_receipts SET colours_before=ARRAY[1,1,1,1] WHERE committed_character_revision=3",
        "DELETE FROM game_character_appearance",
    ] {
        let mut tx = connection.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(query).execute(&mut *tx).await?;
        assert!(matches!(
            verify_character_appearance_integrity(&mut tx).await,
            Err(character_appearance::AppearanceReadError::InvalidStoredState)
        ));
        tx.rollback().await?;
        verify_character_appearance_integrity(connection).await?;
    }
    Ok(())
}

#[test]
fn private_appearance_schema_and_native_validation_postgres() -> TestResult {
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("app_private_{}_{suffix}", std::process::id());
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut admin)
            .await?;
        assert_eq!(version, "170006", "canonical PostgreSQL 17.6 required");
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut admin)
            .await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let result = async {
            let mut connection = sqlx::PgConnection::connect(&format!("{prefix}/{name}")).await?;
            qualify(&mut connection).await?;
            connection.close().await?;
            // Reload via a new connection; this is storage persistence, not an actual relog.
            let mut reloaded = sqlx::PgConnection::connect(&format!("{prefix}/{name}")).await?;
            let selected =
                read_selected_appearance(&mut reloaded, CharacterId::from_bytes(id(41))?)
                    .await?
                    .ok_or("lost selection")?;
            assert_eq!(selected.content_revision, "old-world");
            assert_eq!(selected.selection.outfit_key, "starter");
            assert_eq!(selected.selection.mount_key, None);
            assert_eq!(selected.committed_character_revision.get(), 3);
            assert_eq!(
                read_account_appearance_unlocks(&mut reloaded, AccountId::from_bytes(id(40))?)
                    .await?
                    .len(),
                3
            );
            reloaded.close().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {name} WITH (FORCE)"
        )))
        .execute(&mut admin)
        .await?;
        admin.close().await?;
        result
    })
}
