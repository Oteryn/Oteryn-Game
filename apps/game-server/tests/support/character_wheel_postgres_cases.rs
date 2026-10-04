// WHEEL-W1 cases (migration 0070, WHEEL-0 §4, §5.1 and §13) on the CHARM-2 harness: Character 41
// is first moved by SQL to level 60 (r2, an XP award) and to vocation `sorcerer` (r3, a build
// receipt), so it has 10 available points. The writer cases run `commit_character_wheel`,
// `reset_character_wheel` and the admission step directly or through the revision slot; the
// guard cases issue the exact SQL a writer must (root and state successor, receipt, state row,
// slot rows) as one transaction, so every guard branch is reached independently of the writer.

use std::sync::Arc;

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::character_wheel::{
    CommittedWheelChange, WHEEL_SLOTS, WheelAllocation, WheelChangeFacts, WheelChangeRequest,
    WheelCommitOutcome, WheelIneligibility, WheelOccurrence, WheelReceiptKind, WheelRefusal,
    WheelResetOutcome, WheelRuleset, WheelSlots, WheelStages, admit_character_wheel,
};
use crate::foundation::ConnectionGeneration;

const WHEEL: &str = include_str!("../../../../rulesets/progression/wheel-of-destiny/wheel.json");
const R1: &str = "wheel-authoring-candidate-r1";
const CHAIN: &str = "Character Wheel allocation is inconsistent with its Wheel receipts";
const PROGRESSION: &str = "Character progression revision/receipt chain is inconsistent";
const AT_ROOT: &str = "Wheel receipt is not at the current Character root revision";
const KIND: &str = "game_character_wheel_receipt_kind_shape";
const COMPLETE: &str = "Wheel ruleset revision must have 36 slot capacities";

/// Level and experience after the setup award.
const LEVEL: i64 = 60;
const EXPERIENCE: i64 = 5000;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn ruleset(revision: &str) -> TestResult<Arc<WheelRuleset>> {
    let file: serde_json::Value = serde_json::from_str(WHEEL)?;
    Ok(Arc::new(
        WheelRuleset::from_catalogue(revision, &file["data"]).map_err(debug)?,
    ))
}

fn slots(points: &[(usize, u16)]) -> TestResult<WheelSlots> {
    let mut out = [0; WHEEL_SLOTS];
    for (slot, value) in points {
        out[slot - 1] = *value;
    }
    WheelSlots::new(out).map_err(|error| debug(error).into())
}

fn occurrence(seed: u8) -> TestResult<WheelOccurrence> {
    WheelOccurrence::from_bytes(id(seed)).map_err(|error| debug(error).into())
}

fn request(seed: u8, expected: u64, points: &[(usize, u16)]) -> TestResult<WheelChangeRequest> {
    Ok(WheelChangeRequest {
        occurrence: occurrence(seed)?,
        expected_wheel_revision: expected,
        slots: slots(points)?,
    })
}

const ELIGIBLE: WheelChangeFacts = WheelChangeFacts {
    promoted: true,
    at_temple: false,
};
const AT_TEMPLE: WheelChangeFacts = WheelChangeFacts {
    promoted: true,
    at_temple: true,
};

async fn commit(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: WheelChangeRequest,
    ruleset: &Arc<WheelRuleset>,
    facts: WheelChangeFacts,
) -> Result<WheelCommitOutcome, String> {
    harness
        .root
        .commit_character_wheel(
            authority,
            &harness.node,
            fence,
            request,
            Arc::clone(ruleset),
            facts,
        )
        .await
        .map_err(debug)
}

fn committed<E: std::fmt::Debug>(
    outcome: Result<WheelCommitOutcome, E>,
) -> TestResult<CommittedWheelChange> {
    match outcome {
        Ok(WheelCommitOutcome::Committed(receipt)) => Ok(receipt),
        other => Err(format!("expected a commit, got {other:?}").into()),
    }
}

/// Every relation 0070 binds and the revision chain, as one comparable value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(character_revision::text, ',') FROM game_character_roots), \
           (SELECT string_agg(character_revision::text, ',') \
              FROM game_character_progression_state), \
           (SELECT string_agg(concat_ws(':', committed_character_revision, kind, \
                     after_wheel_revision, wheel_ruleset_revision), ',' ORDER BY 1) \
              FROM game_character_wheel_receipts), \
           (SELECT string_agg(concat_ws(':', wheel_ruleset_revision, wheel_revision, \
                     allocated_total, committed_character_revision), ',') \
              FROM game_character_wheel_state), \
           (SELECT string_agg(concat_ws(':', slot, points), ',' ORDER BY slot) \
              FROM game_character_wheel_slots))",
    )
    .fetch_one(pool)
    .await?)
}

/// Runs `script` as one transaction and commits it, so immediate and deferred guard failures
/// both surface here.
async fn attempt(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn expect_committed(pool: &sqlx::PgPool, case: &str, script: &str) -> TestResult {
    attempt(pool, script)
        .await
        .map_err(|error| format!("{case}: expected commit, got {error}").into())
}

/// The rejection must be the named guard message (or CHECK constraint) and change nothing.
async fn expect_rejected(pool: &sqlx::PgPool, case: &str, script: &str, rule: &str) -> TestResult {
    let before = snapshot(pool).await?;
    let error = attempt(pool, script)
        .await
        .err()
        .ok_or_else(|| format!("{case}: expected rejection"))?;
    let database = error
        .as_database_error()
        .ok_or_else(|| format!("{case}: not a database error: {error}"))?;
    assert_eq!(database.code().as_deref(), Some("23514"), "{case}: {error}");
    let observed = database.constraint().unwrap_or(database.message());
    assert_eq!(observed, rule, "{case}");
    assert_eq!(
        snapshot(pool).await?,
        before,
        "{case}: rejected write changed state"
    );
    Ok(())
}

/// The root and typed-state successor of one semantic Character transaction.
fn advance(original: u64, level: i64, experience: i64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, level = {level}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// An XP award transaction at `original` from (`before`) to (`after`) level and experience.
fn award(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> String {
    format!(
        "{advance}INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           {original}, {committed}, {level_before}, {level_after}, {experience_before}, \
           {experience_after}, {awarded}, 'profile-1', 'ruleset-1', 'content-1', 'simulation-1', \
           'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 1);",
        advance = advance(original, after.0, after.1),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 8]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        level_before = before.0,
        level_after = after.0,
        experience_before = before.1,
        experience_after = after.1,
        awarded = after.1 - before.1,
    )
}

/// The vocation choice `none` -> `vocation` at r3 (level 60), with its build row.
fn choose_vocation(occurrence: u8, vocation: &str) -> String {
    let columns = "magic_level{s}, mana_spent{s}, fist_level{s}, fist_tries{s}, club_level{s}, \
                   club_tries{s}, sword_level{s}, sword_tries{s}, axe_level{s}, axe_tries{s}, \
                   distance_level{s}, distance_tries{s}, shielding_level{s}, shielding_tries{s}, \
                   fishing_level{s}, fishing_tries{s}";
    let seed = "0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0";
    format!(
        "{advance}INSERT INTO game_character_build_receipts(\
           build_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, cause, level_before, \
           level_after, experience_before, experience_after, vocation_before, {before}, \
           vocation_after, {after}, stance_before, stance_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, 2, 3, \
           'vocation_choice', {LEVEL}, {LEVEL}, {EXPERIENCE}, {EXPERIENCE}, 'none', {seed}, \
           '{vocation}', {seed}, NULL, NULL, 'profile-1', 'ruleset-1', 'content-1', \
           'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 2); \
         INSERT INTO game_character_build_state(character_id, vocation, {plain}, \
           committed_character_revision, last_build_occurrence_id) \
         VALUES ({character}, '{vocation}', {seed}, 3, {occurrence});",
        advance = advance(2, LEVEL, EXPERIENCE),
        before = columns.replace("{s}", "_before"),
        after = columns.replace("{s}", "_after"),
        plain = columns.replace("{s}", ""),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 33]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
    )
}

/// Character 41 at level 60, `sorcerer`, revision 3.
async fn eligible(pool: &sqlx::PgPool) -> TestResult {
    expect_committed(
        pool,
        "setup XP r2",
        &award(90, 1, (50, 1000), (LEVEL, EXPERIENCE)),
    )
    .await?;
    expect_committed(pool, "setup vocation r3", &choose_vocation(91, "sorcerer")).await
}

/// Register a later ruleset revision with the first revision's capacities.
fn register_revision(revision: &str, ordinal: i32, kind: &str) -> String {
    format!(
        "INSERT INTO game_wheel_ruleset_revisions VALUES ('{revision}', {ordinal}, '{kind}'); \
         INSERT INTO game_wheel_ruleset_slot_capacities \
         SELECT '{revision}', slot, capacity FROM game_wheel_ruleset_slot_capacities \
          WHERE wheel_ruleset_revision = '{R1}';"
    )
}

fn vector(points: &[(usize, i16)]) -> String {
    let mut out = [0_i16; WHEEL_SLOTS];
    for (slot, value) in points {
        out[slot - 1] = *value;
    }
    format!(
        "ARRAY[{}]::smallint[]",
        out.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// One Wheel receipt as a writer inserts it, with every field a case may change.
#[derive(Clone)]
struct Receipt {
    occurrence: String,
    kind: &'static str,
    original: u64,
    before_revision: u64,
    before: Vec<(usize, i16)>,
    after: Vec<(usize, i16)>,
    rulesets: (String, String),
}

impl Receipt {
    fn allocation(seed: u8, original: u64, before_revision: u64, after: &[(usize, i16)]) -> Self {
        Self {
            occurrence: uuid(seed),
            kind: "ALLOCATION",
            original,
            before_revision,
            before: Vec::new(),
            after: after.to_vec(),
            rulesets: (R1.into(), R1.into()),
        }
    }

    fn after_revision(&self) -> u64 {
        self.before_revision + 1
    }

    fn insert(&self) -> String {
        format!(
            "INSERT INTO game_character_wheel_receipts(character_id, wheel_occurrence_id, kind, \
               request_binding, before_wheel_revision, after_wheel_revision, slots_before, \
               slots_after, before_wheel_ruleset_revision, wheel_ruleset_revision, \
               original_character_revision, committed_character_revision, level_before, \
               level_after, experience_before, experience_after, profile_revision, \
               ruleset_revision, content_revision, simulation_revision, evidence_revision, \
               declaration_revision, policy_revision, reward_revision, committed_at) \
             VALUES ({character}, {occurrence}, '{kind}', '\\x{binding}'::bytea, \
               {before_revision}, {after_revision}, {before}, {after}, '{rs_before}', \
               '{rs_after}', {original}, {committed}, {LEVEL}, {LEVEL}, {EXPERIENCE}, \
               {EXPERIENCE}, 'profile-1', 'ruleset-1', 'content-1', 'simulation-1', \
               'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 7);",
            character = uuid(CHARACTER),
            occurrence = self.occurrence,
            kind = self.kind,
            binding = hex(&[7; 33]),
            before_revision = self.before_revision,
            after_revision = self.after_revision(),
            before = vector(&self.before),
            after = vector(&self.after),
            rs_before = self.rulesets.0,
            rs_after = self.rulesets.1,
            original = self.original,
            committed = self.original + 1,
        )
    }

    /// The state row and slot rows the receipt writes.
    fn rows(&self) -> String {
        let total: i32 = self
            .after
            .iter()
            .map(|(_, points)| i32::from(*points))
            .sum();
        let mut script = format!(
            "INSERT INTO game_character_wheel_state VALUES ({character}, '{ruleset}', \
               {revision}, {total}, {committed}, {occurrence}) \
             ON CONFLICT (character_id) DO UPDATE SET \
               wheel_ruleset_revision = EXCLUDED.wheel_ruleset_revision, \
               wheel_revision = EXCLUDED.wheel_revision, \
               allocated_total = EXCLUDED.allocated_total, \
               committed_character_revision = EXCLUDED.committed_character_revision, \
               last_wheel_occurrence_id = EXCLUDED.last_wheel_occurrence_id; \
             DELETE FROM game_character_wheel_slots WHERE character_id = {character};",
            character = uuid(CHARACTER),
            ruleset = self.rulesets.1,
            revision = self.after_revision(),
            committed = self.original + 1,
            occurrence = self.occurrence,
        );
        for (slot, points) in self.after.iter().filter(|(_, points)| *points > 0) {
            script.push_str(&format!(
                "INSERT INTO game_character_wheel_slots VALUES ({}, {slot}, {points});",
                uuid(CHARACTER)
            ));
        }
        script
    }

    /// The complete change: successor, receipt, state and slot rows.
    fn commit(&self) -> String {
        format!(
            "{}{}{}",
            advance(self.original, LEVEL, EXPERIENCE),
            self.insert(),
            self.rows()
        )
    }
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

#[test]
fn wheel_changes_commit_replay_and_refuse_writing_nothing() -> TestResult {
    run("wheel_writer", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let r1 = ruleset(R1)?;

        // No row: every slot 0 under the active revision, every stage 0.
        let none = harness
            .root
            .read_character_wheel(&authority, character, Arc::clone(&r1))
            .await
            .map_err(debug)?;
        assert_eq!(
            none,
            WheelAllocation {
                wheel_ruleset_revision: None,
                wheel_revision: 0,
                slots: WheelSlots::ZERO,
                current: true,
            }
        );
        assert_eq!(
            WheelStages::derive(&r1, &none, "sorcerer", 60, true),
            WheelStages::default()
        );

        // Level 50 without a vocation is not eligible; nothing is written.
        let before = snapshot(pool).await?;
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(1)?,
                request(1, 0, &[(15, 1)])?,
                &r1,
                ELIGIBLE
            )
            .await,
            Ok(WheelCommitOutcome::Refused(WheelRefusal::NotEligible(
                WheelIneligibility::NoVocation
            )))
        );
        assert_eq!(snapshot(pool).await?, before);
        eligible(pool).await?;

        // r4: the first change writes the state row pinned to the active revision.
        let first = committed(
            commit(
                harness,
                &authority,
                fence(3)?,
                request(1, 0, &[(15, 10)])?,
                &r1,
                ELIGIBLE,
            )
            .await,
        )?;
        assert_eq!(first.kind, WheelReceiptKind::Allocation);
        assert_eq!(first.committed_character_revision.get(), 4);
        assert_eq!(
            (first.before_wheel_revision, first.after_wheel_revision),
            (0, 1)
        );
        assert_eq!(first.wheel_ruleset_revision, R1);
        let stored = harness
            .root
            .read_character_wheel(&authority, character, Arc::clone(&r1))
            .await
            .map_err(debug)?;
        assert_eq!(stored.slots, slots(&[(15, 10)])?);
        assert_eq!(stored.wheel_revision, 1);
        assert!(stored.current);

        // Exact replay returns the receipt even at a stale fence; another binding of the
        // occurrence conflicts.
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(3)?,
                request(1, 0, &[(15, 10)])?,
                &r1,
                ELIGIBLE
            )
            .await,
            Ok(WheelCommitOutcome::AlreadyCommitted(first.clone()))
        );
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(4)?,
                request(1, 0, &[(15, 9)])?,
                &r1,
                ELIGIBLE
            )
            .await,
            Err(debug(CharacterProgressionError::ConflictingOccurrence))
        );

        // Each refusal writes nothing and leaves the occurrence unconsumed.
        let before = snapshot(pool).await?;
        for (case, request, facts, refusal) in [
            (
                "stale",
                request(2, 0, &[(15, 9)])?,
                AT_TEMPLE,
                WheelRefusal::StaleRevision,
            ),
            (
                "no change",
                request(2, 1, &[(15, 10)])?,
                ELIGIBLE,
                WheelRefusal::NoChange,
            ),
            (
                "not promoted",
                request(2, 1, &[(15, 9)])?,
                WheelChangeFacts {
                    promoted: false,
                    at_temple: true,
                },
                WheelRefusal::NotEligible(WheelIneligibility::NotPromoted),
            ),
            (
                "capacity",
                request(2, 1, &[(15, 51)])?,
                ELIGIBLE,
                WheelRefusal::OverCapacity,
            ),
            (
                "points",
                request(2, 1, &[(15, 10), (16, 1)])?,
                ELIGIBLE,
                WheelRefusal::OverPoints,
            ),
            (
                "adjacency",
                request(2, 1, &[(15, 10), (2, 1)])?,
                ELIGIBLE,
                WheelRefusal::NotAdjacent,
            ),
            (
                "temple",
                request(2, 1, &[(15, 9)])?,
                ELIGIBLE,
                WheelRefusal::RemovalNotAtTemple,
            ),
        ] {
            assert_eq!(
                commit(harness, &authority, fence(4)?, request, &r1, facts).await,
                Ok(WheelCommitOutcome::Refused(refusal)),
                "{case}"
            );
            assert_eq!(snapshot(pool).await?, before, "{case}");
        }

        // A stale session fence is refused before any Wheel read.
        let stale = CurrentCharacterGameplayFence {
            connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
            ..fence(4)?
        };
        assert_eq!(
            commit(
                harness,
                &authority,
                stale,
                request(2, 1, &[(15, 9)])?,
                &r1,
                AT_TEMPLE
            )
            .await,
            Err(debug(CharacterProgressionError::AuthorityRejected))
        );
        assert_eq!(snapshot(pool).await?, before);

        // r5: the same occurrence after its refusals commits a removal at a temple; the slot
        // row of a slot moved to 0 is deleted.
        let removal = committed(
            commit(
                harness,
                &authority,
                fence(4)?,
                request(2, 1, &[(16, 5)])?,
                &r1,
                AT_TEMPLE,
            )
            .await,
        )?;
        assert_eq!(removal.before, slots(&[(15, 10)])?);
        assert_eq!(removal.after_wheel_revision, 2);
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT string_agg(concat_ws(':', slot, points), ',' ORDER BY slot) \
                   FROM game_character_wheel_slots"
            )
            .fetch_one(pool)
            .await?,
            "16:5"
        );
        assert_eq!(
            harness
                .root
                .reconcile_character_wheel(&authority, character, occurrence(2)?)
                .await
                .map_err(debug)?,
            Some(removal)
        );
        assert_eq!(
            harness
                .root
                .reconcile_character_wheel(&authority, character, occurrence(3)?)
                .await
                .map_err(debug)?,
            None
        );
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn the_slot_retries_a_bypassed_wheel_change_once() -> TestResult {
    run("wheel_bypass", async |harness| {
        let pool = &harness.pool;
        eligible(pool).await?;
        let r1 = ruleset(R1)?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        assert_eq!(
            slot.cursor(&harness.root, &authority)
                .await
                .map_err(debug)?
                .get(),
            3
        );
        // A writer that bypasses the sequencer moves the root under the held slot.
        expect_committed(
            pool,
            "bypass XP r4",
            &award(60, 3, (LEVEL, EXPERIENCE), (LEVEL, EXPERIENCE + 10)),
        )
        .await?;
        // The Wheel binding excludes the CharacterRevision: one retry commits at r5.
        let retried = committed(
            slot.commit_wheel(
                &harness.root,
                &authority,
                &harness.node,
                fence(3)?,
                request(1, 0, &[(15, 10)])?,
                Arc::clone(&r1),
                ELIGIBLE,
            )
            .await,
        )?;
        assert_eq!(retried.original_character_revision.get(), 4);
        assert_eq!(retried.committed_character_revision.get(), 5);
        drop(slot);
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn the_admission_reset_clears_a_stale_allocation_once() -> TestResult {
    run("wheel_reset", async |harness| {
        let pool = &harness.pool;
        eligible(pool).await?;
        let r1 = ruleset(R1)?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        committed(
            commit(
                harness,
                &authority,
                fence(3)?,
                request(1, 0, &[(15, 10)])?,
                &r1,
                ELIGIBLE,
            )
            .await,
        )?;

        // A value-only revision keeps the allocation pinned to r1 and current.
        expect_committed(
            pool,
            "value-only r2",
            &register_revision("wheel-r2", 2, "VALUE_ONLY"),
        )
        .await?;
        let r2 = ruleset("wheel-r2")?;
        let kept = admit_character_wheel(
            &sequencer,
            &harness.root,
            &authority,
            &harness.node,
            fence(4)?,
            &r2,
        )
        .await
        .ok_or("admission load")?;
        assert_eq!(kept.wheel_ruleset_revision.as_deref(), Some(R1));
        assert_eq!(kept.slots, slots(&[(15, 10)])?);
        assert!(kept.current);
        assert_eq!(harness.root_revision().await?, "4");

        // A reset revision after it: before the reset the allocation fails closed (stages 0,
        // changes refused).
        expect_committed(pool, "reset r3", &register_revision("wheel-r3", 3, "RESET")).await?;
        let r3 = ruleset("wheel-r3")?;
        let pending = harness
            .root
            .read_character_wheel(&authority, character, Arc::clone(&r3))
            .await
            .map_err(debug)?;
        assert!(!pending.current);
        assert_eq!(
            WheelStages::derive(&r3, &pending, "sorcerer", 60, true),
            WheelStages::default()
        );
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(4)?,
                request(2, 1, &[(15, 9)])?,
                &r3,
                AT_TEMPLE
            )
            .await,
            Ok(WheelCommitOutcome::Refused(WheelRefusal::RulesetNotCurrent))
        );

        // The admission reset: one RULESET_RESET receipt at r5, every point returned.
        let reset = admit_character_wheel(
            &sequencer,
            &harness.root,
            &authority,
            &harness.node,
            fence(4)?,
            &r3,
        )
        .await
        .ok_or("admission load")?;
        assert_eq!(
            reset,
            WheelAllocation {
                wheel_ruleset_revision: Some("wheel-r3".into()),
                wheel_revision: 2,
                slots: WheelSlots::ZERO,
                current: true,
            }
        );
        assert_eq!(harness.root_revision().await?, "5");
        let receipt: (String, String, String) = sqlx::query_as(
            "SELECT kind, before_wheel_ruleset_revision, wheel_ruleset_revision \
               FROM game_character_wheel_receipts WHERE committed_character_revision = 5",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(
            receipt,
            ("RULESET_RESET".into(), R1.into(), "wheel-r3".into())
        );
        assert_eq!(harness.count("game_character_wheel_slots").await?, 0);

        // A replay finds the stored revision active and writes nothing.
        let before = snapshot(pool).await?;
        let mut slot = sequencer.acquire(character).await;
        assert_eq!(
            slot.reset_wheel(
                &harness.root,
                &authority,
                &harness.node,
                fence(5)?,
                Arc::clone(&r3)
            )
            .await
            .map_err(debug),
            Ok(WheelResetOutcome::Current)
        );
        drop(slot);
        assert_eq!(snapshot(pool).await?, before);

        // An older active revision than the stored one, or an unregistered one, is unresolved:
        // nothing is written and the allocation fails closed.
        for active in [&r1, &ruleset("wheel-r9")?] {
            assert_eq!(
                harness
                    .root
                    .reset_character_wheel(&authority, &harness.node, fence(5)?, Arc::clone(active))
                    .await
                    .map_err(debug),
                Ok(WheelResetOutcome::Unresolved)
            );
            assert!(
                !harness
                    .root
                    .read_character_wheel(&authority, character, Arc::clone(active))
                    .await
                    .map_err(debug)?
                    .current
            );
            assert_eq!(
                commit(
                    harness,
                    &authority,
                    fence(5)?,
                    request(3, 2, &[(15, 1)])?,
                    active,
                    ELIGIBLE
                )
                .await,
                Ok(WheelCommitOutcome::Refused(WheelRefusal::RulesetNotCurrent))
            );
        }
        assert_eq!(snapshot(pool).await?, before);

        // Under the new revision the Character allocates again from zero.
        committed(
            commit(
                harness,
                &authority,
                fence(5)?,
                request(3, 2, &[(15, 10)])?,
                &r3,
                ELIGIBLE,
            )
            .await,
        )?;
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn wheel_guards_reject_inconsistent_rows() -> TestResult {
    run("wheel_guards", async |harness| {
        let pool = &harness.pool;

        // Revision one has no Wheel row.
        let state_only = format!(
            "INSERT INTO game_character_wheel_state VALUES ({}, '{R1}', 1, 0, 2, {});",
            uuid(CHARACTER),
            uuid(70)
        );
        expect_rejected(pool, "state at revision one", &state_only, CHAIN).await?;
        eligible(pool).await?;

        // A ruleset revision needs its 36 capacities.
        expect_rejected(
            pool,
            "revision without capacities",
            "INSERT INTO game_wheel_ruleset_revisions VALUES ('wheel-r2', 2, 'RESET');",
            COMPLETE,
        )
        .await?;

        let first = Receipt::allocation(70, 3, 0, &[(15, 10)]);
        // Row-only writes, a receipt without its rows and a receipt off the root fail.
        let row_only = format!("{}{}", advance(3, LEVEL, EXPERIENCE), first.rows());
        expect_rejected(pool, "rows without receipt", &row_only, PROGRESSION).await?;
        let receipt_only = format!("{}{}", advance(3, LEVEL, EXPERIENCE), first.insert());
        expect_rejected(pool, "receipt without rows", &receipt_only, CHAIN).await?;
        expect_rejected(pool, "receipt off the root", &first.insert(), AT_ROOT).await?;
        let mut wrong_slots = first.commit();
        wrong_slots.push_str(&format!(
            "UPDATE game_character_wheel_slots SET points = 9 WHERE character_id = {};",
            uuid(CHARACTER)
        ));
        expect_rejected(pool, "slots differ from the receipt", &wrong_slots, CHAIN).await?;
        let mut wrong_total = first.commit();
        wrong_total.push_str(&format!(
            "UPDATE game_character_wheel_state SET allocated_total = 9, wheel_revision = 2, \
               committed_character_revision = 5 WHERE character_id = {};",
            uuid(CHARACTER)
        ));
        expect_rejected(pool, "state differs from the receipt", &wrong_total, CHAIN).await?;

        // The first receipt starts at Wheel revision 0 with an all-zero vector.
        let mut late = first.clone();
        late.before_revision = 1;
        expect_rejected(
            pool,
            "first receipt after revision 0",
            &late.commit(),
            CHAIN,
        )
        .await?;
        let mut seeded = first.clone();
        seeded.before = vec![(15, 1)];
        expect_rejected(
            pool,
            "first receipt from a non-zero vector",
            &seeded.commit(),
            CHAIN,
        )
        .await?;
        // Every slot within its capacity under the receipt's revision.
        let over = Receipt::allocation(70, 3, 0, &[(15, 51)]);
        expect_rejected(pool, "over capacity", &over.commit(), CHAIN).await?;
        // An allocation is a real change under one revision.
        let mut same = first.clone();
        same.before = vec![(15, 10)];
        same.before_revision = 0;
        expect_rejected(pool, "no-change receipt", &same.insert(), KIND).await?;

        expect_committed(pool, "first allocation r4", &first.commit()).await?;

        // A later receipt continues from its predecessor.
        let mut gap = Receipt::allocation(71, 4, 1, &[(15, 10), (9, 1)]);
        gap.before = vec![(15, 9)];
        expect_rejected(
            pool,
            "before vector is not the previous after",
            &gap.commit(),
            CHAIN,
        )
        .await?;
        let mut skipped = Receipt::allocation(71, 4, 2, &[(15, 10), (9, 1)]);
        skipped.before = vec![(15, 10)];
        expect_rejected(pool, "Wheel revision skipped", &skipped.commit(), CHAIN).await?;

        // A reset needs a RESET revision after the source up to the destination.
        expect_committed(
            pool,
            "value-only r2, reset r3",
            &format!(
                "{}{}",
                register_revision("wheel-r2", 2, "VALUE_ONLY"),
                register_revision("wheel-r3", 3, "RESET")
            ),
        )
        .await?;
        let reset = |destination: &str| {
            let mut receipt = Receipt::allocation(72, 4, 1, &[]);
            receipt.occurrence = "'01020304-0506-8708-8809-0a0b0c0d0e0f'::uuid".into();
            receipt.kind = "RULESET_RESET";
            receipt.before = vec![(15, 10)];
            receipt.rulesets = (R1.into(), destination.into());
            receipt
        };
        expect_rejected(
            pool,
            "reset to a value-only revision",
            &reset("wheel-r2").commit(),
            CHAIN,
        )
        .await?;
        let mut keeps = reset("wheel-r3");
        keeps.after = vec![(15, 10)];
        expect_rejected(pool, "reset that keeps points", &keeps.insert(), KIND).await?;
        expect_committed(pool, "reset to r3", &reset("wheel-r3").commit()).await?;
        // After it, an allocation must name the new revision as its source.
        let mut old = Receipt::allocation(73, 5, 2, &[(15, 1)]);
        old.rulesets = (R1.into(), R1.into());
        expect_rejected(
            pool,
            "allocation under the old revision",
            &old.commit(),
            CHAIN,
        )
        .await?;

        // Every guard runs as the writing role, which calls the Wheel arm.
        let runtime: bool = sqlx::query_scalar(
            "SELECT has_function_privilege('oteryn_game_runtime', \
               'game_character_wheel_consistency(uuid)', 'EXECUTE')",
        )
        .fetch_one(pool)
        .await?;
        assert!(runtime);

        // Receipts are immutable; the state row is never deleted.
        assert!(
            attempt(
                pool,
                "UPDATE game_character_wheel_receipts SET committed_at = 9;"
            )
            .await
            .is_err()
        );
        assert!(
            attempt(pool, "DELETE FROM game_character_wheel_state;")
                .await
                .is_err()
        );
        assert!(
            attempt(pool, "DELETE FROM game_character_wheel_receipts;")
                .await
                .is_err()
        );
        Ok(())
    })
}
