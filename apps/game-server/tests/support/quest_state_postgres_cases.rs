// QUEST-STATE-1 cases (migration 0056, QUEST-STATE-0 §3-§6, §9 and §13) on the CHARM-2 harness:
// Character 41 at revision one (level 50, 1000 experience) live on session 50. The writer cases
// run `commit_character_quest_transition` directly or through the revision slot; the guard cases
// issue the exact SQL a writer must (root and state successor, receipt, track rows, quest state)
// as one transaction, so every guard branch is reached independently of the writer. The
// reward-claim obligation cases run on the CHEST-1 harness (`reward_claim_mint_postgres_cases`).

use std::sync::Arc;

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::quest_state::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTrackChange, QuestTransition,
};
use crate::durability::quest_state::{
    CommittedQuestTransition, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};

const QUEST: &str = "oteryn:quest/fixture.rats";
const OTHER: &str = "oteryn:quest/fixture.other";
const STAGE: &str = "oteryn:quest-progress/fixture.rats.stage";
const TIMER: &str = "oteryn:quest-progress/fixture.rats.timer";
const FLAG: &str = "oteryn:quest-progress/fixture.other.flag";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn transition_key(name: &str) -> String {
    format!("oteryn:quest-transition/fixture.{name}")
}

fn effect(track: &str, from: QuestComparison, effect: QuestEffectKind) -> QuestEffect {
    QuestEffect {
        track: track.into(),
        from,
        effect,
    }
}

/// The fixture quests. `step` is the amount `step` adds: another amount is another definition
/// hash for the rats quest (§6), the other quest's hash is unchanged.
fn catalogue_with(step: i64, content: &str) -> TestResult<Arc<QuestStateCatalogue>> {
    use QuestComparison as From;
    use QuestEffectKind as Effect;
    let rats = |name: &str, effects: Vec<QuestEffect>, completes: bool| QuestTransition {
        key: transition_key(name),
        quest: QUEST.into(),
        effects,
        completes,
        experience: None,
    };
    let track = |key: &str, quest: &str, initial: i64, min: i64, max: i64| QuestTrack {
        key: key.into(),
        quest: quest.into(),
        initial,
        min,
        max,
    };
    Ok(Arc::new(
        QuestStateCatalogue::new(
            content,
            vec![
                track(STAGE, QUEST, -1, -1, 10),
                track(TIMER, QUEST, 0, 0, i64::MAX),
                track(FLAG, OTHER, 0, 0, 1),
            ],
            vec![
                rats(
                    "start",
                    vec![
                        effect(STAGE, From::Eq(-1), Effect::Set(1)),
                        effect(TIMER, From::Any, Effect::SetNow),
                    ],
                    false,
                ),
                rats(
                    "step",
                    vec![effect(STAGE, From::Between(1, 9), Effect::Add(step))],
                    false,
                ),
                rats(
                    "finish",
                    vec![effect(STAGE, From::Ge(3), Effect::Set(10))],
                    true,
                ),
                rats(
                    "wait",
                    vec![effect(TIMER, From::ElapsedAtLeast(3600), Effect::SetNow)],
                    false,
                ),
                rats(
                    "computed",
                    vec![effect(STAGE, From::Any, Effect::Computed)],
                    false,
                ),
                rats(
                    "overflow",
                    vec![effect(STAGE, From::Any, Effect::Add(100))],
                    false,
                ),
                rats(
                    "again",
                    vec![effect(STAGE, From::Any, Effect::Set(10))],
                    false,
                ),
                QuestTransition {
                    key: transition_key("other"),
                    quest: OTHER.into(),
                    effects: vec![effect(FLAG, From::Eq(0), Effect::Set(1))],
                    completes: false,
                    experience: None,
                },
            ],
        )
        .map_err(debug)?,
    ))
}

fn catalogue() -> TestResult<Arc<QuestStateCatalogue>> {
    catalogue_with(1, "content-1")
}

fn command(command_id: u64) -> TestResult<CommandRef> {
    command_in(SESSION, command_id)
}

fn command_in(session: u8, command_id: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(session)).map_err(debug)?,
        CommandId::new(command_id).map_err(debug)?,
    ))
}

fn request(name: &str, cause: QuestCause) -> QuestTransitionRequest {
    QuestTransitionRequest {
        transition_key: transition_key(name),
        cause,
    }
}

async fn commit(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: QuestTransitionRequest,
    catalogue: &Arc<QuestStateCatalogue>,
) -> Result<QuestTransitionOutcome, String> {
    harness
        .root
        .commit_character_quest_transition(
            authority,
            &harness.node,
            fence,
            request,
            Arc::clone(catalogue),
        )
        .await
        .map_err(debug)
}

fn committed<E: std::fmt::Debug>(
    outcome: Result<QuestTransitionOutcome, E>,
) -> TestResult<CommittedQuestTransition> {
    match outcome {
        Ok(QuestTransitionOutcome::Committed(receipt)) => Ok(receipt),
        other => Err(format!("expected a commit, got {other:?}").into()),
    }
}

/// Every relation 0056 binds and the revision chain, as one comparable value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(character_revision::text, ',') FROM game_character_roots), \
           (SELECT string_agg(character_revision::text, ',') \
              FROM game_character_progression_state), \
           (SELECT string_agg(concat_ws(':', committed_character_revision, transition_key, \
                     cause_kind, array_to_string(values_after, '/')), ',' ORDER BY 1) \
              FROM game_character_quest_receipts), \
           (SELECT string_agg(concat_ws(':', track_key, value, committed_character_revision), \
                     ',' ORDER BY 1) FROM game_character_quest_tracks), \
           (SELECT string_agg(concat_ws(':', quest_key, started_character_revision, \
                     coalesce(completed_character_revision::text, '-'), \
                     committed_character_revision), ',' ORDER BY 1) \
              FROM game_character_quest_states))",
    )
    .fetch_one(pool)
    .await?)
}

async fn track_value(pool: &sqlx::PgPool, track: &str) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks \
          WHERE character_id = encode($1,'hex')::uuid AND track_key = $2",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(track)
    .fetch_optional(pool)
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

/// The root and typed-state successor of one semantic Character transaction.
fn advance(original: u64, experience: i64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// An XP award transaction from (50, `before`) to (50, `after`) experience.
fn award(occurrence: u8, original: u64, before: i64, after: i64) -> String {
    format!(
        "{advance}INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           {original}, {committed}, 50, 50, {before}, {after}, {awarded}, 'profile-1', \
           'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', \
           'reward-1', 1);",
        advance = advance(original, after),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 8]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        awarded = after - before,
    )
}

/// A stance toggle transaction (0017) at level 50 and `experience`.
fn toggle(occurrence: u8, original: u64, experience: i64, after: &str) -> String {
    format!(
        "{advance}INSERT INTO game_character_stance_receipts(\
           stance_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, \
           level_after, experience_before, experience_after, stance_before, stance_after, \
           profile_revision, ruleset_revision, content_revision, simulation_revision, \
           evidence_revision, declaration_revision, policy_revision, reward_revision, \
           committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           {original}, {committed}, 50, 50, {experience}, {experience}, NULL, '{after}', \
           'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
           'declaration-1', 'policy-1', 'reward-1', 3); \
         INSERT INTO game_character_stance VALUES ({character}, '{after}', {committed}, \
           {occurrence});",
        advance = advance(original, experience),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 8]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
    )
}

/// One quest receipt row as a writer inserts it, with every field a case may change.
#[derive(Clone)]
struct Receipt {
    original: u64,
    experience: i64,
    cause: (&'static str, u8, u64),
    transition: String,
    quest: String,
    binding: Vec<u8>,
    pin: (&'static str, [u8; 32]),
    completes: bool,
    effects: Vec<(String, i64, i64)>,
}

impl Receipt {
    fn new(original: u64, ordinal: u64, effects: &[(&str, i64, i64)]) -> Self {
        Self {
            original,
            experience: 1000,
            cause: ("command", SESSION, ordinal),
            transition: transition_key("sql"),
            quest: QUEST.into(),
            binding: vec![1; 33],
            pin: ("content-1", [7; 32]),
            completes: false,
            effects: effects
                .iter()
                .map(|(track, before, after)| ((*track).to_owned(), *before, *after))
                .collect(),
        }
    }

    fn committed(&self) -> u64 {
        self.original + 1
    }

    fn insert(&self) -> String {
        let keys = self
            .effects
            .iter()
            .map(|(track, _, _)| format!("'{track}'"))
            .collect::<Vec<_>>()
            .join(",");
        let values = |pick: fn(&(String, i64, i64)) -> i64| {
            self.effects
                .iter()
                .map(|effect| pick(effect).to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        format!(
            "INSERT INTO game_character_quest_receipts(character_id, cause_kind, cause_id, \
               cause_ordinal, transition_key, quest_key, request_binding, \
               pinned_content_revision, definition_hash, completes, track_keys, values_before, \
               values_after, original_character_revision, committed_character_revision, \
               level_before, level_after, experience_before, experience_after, \
               profile_revision, ruleset_revision, content_revision, simulation_revision, \
               evidence_revision, declaration_revision, policy_revision, reward_revision, \
               committed_at) \
             VALUES ({character}, '{kind}', {cause}, {ordinal}, '{transition}', '{quest}', \
               '\\x{binding}'::bytea, '{pin}', '\\x{hash}'::bytea, {completes}, \
               ARRAY[{keys}]::text[], ARRAY[{before}]::bigint[], ARRAY[{after}]::bigint[], \
               {original}, {committed}, 50, 50, {experience}, {experience}, 'profile-1', \
               'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', 'declaration-1', \
               'policy-1', 'reward-1', 5);",
            character = uuid(CHARACTER),
            kind = self.cause.0,
            cause = uuid(self.cause.1),
            ordinal = self.cause.2,
            transition = self.transition,
            quest = self.quest,
            binding = hex(&self.binding),
            pin = self.pin.0,
            hash = hex(&self.pin.1),
            completes = self.completes,
            before = values(|effect| effect.1),
            after = values(|effect| effect.2),
            original = self.original,
            committed = self.committed(),
            experience = self.experience,
        )
    }

    /// Every track row the receipt writes.
    fn tracks(&self) -> String {
        self.effects
            .iter()
            .map(|(track, _, after)| {
                format!(
                    "INSERT INTO game_character_quest_tracks VALUES ({character}, '{track}', \
                       '{quest}', {after}, {committed}) \
                     ON CONFLICT (character_id, track_key) DO UPDATE \
                       SET value = EXCLUDED.value, \
                           committed_character_revision = EXCLUDED.committed_character_revision;",
                    character = uuid(CHARACTER),
                    quest = self.quest,
                    committed = self.committed(),
                )
            })
            .collect()
    }

    /// The quest state row of a quest started by this receipt.
    fn state_insert(&self) -> String {
        format!(
            "INSERT INTO game_character_quest_states VALUES ({character}, '{quest}', '{pin}', \
               '\\x{hash}'::bytea, {committed}, {completed}, {committed});",
            character = uuid(CHARACTER),
            quest = self.quest,
            pin = self.pin.0,
            hash = hex(&self.pin.1),
            committed = self.committed(),
            completed = if self.completes {
                self.committed().to_string()
            } else {
                "NULL".into()
            },
        )
    }

    /// The quest state row of a quest this receipt continues.
    fn state_update(&self) -> String {
        format!(
            "UPDATE game_character_quest_states SET committed_character_revision = {committed}, \
               completed_character_revision = coalesce(completed_character_revision, {completed}) \
             WHERE character_id = {character} AND quest_key = '{quest}';",
            character = uuid(CHARACTER),
            quest = self.quest,
            committed = self.committed(),
            completed = if self.completes {
                self.committed().to_string()
            } else {
                "NULL".into()
            },
        )
    }

    /// The complete writer transaction of a quest's first receipt.
    fn start(&self) -> String {
        format!(
            "{}{}{}{}",
            advance(self.original, self.experience),
            self.insert(),
            self.tracks(),
            self.state_insert()
        )
    }

    /// The complete writer transaction of a later receipt of a started quest.
    fn continue_quest(&self) -> String {
        format!(
            "{}{}{}{}",
            advance(self.original, self.experience),
            self.insert(),
            self.tracks(),
            self.state_update()
        )
    }
}

const RECEIPT: &str = "quest receipt effect has no track row of its quest written by it";
const AT_ROOT: &str = "quest receipt is not at the current Character root revision";
const DISTINCT: &str = "quest receipt effects must name distinct track keys";
const STATE_OF_RECEIPT: &str = "quest receipt has no quest state of its pin written by it";
const TRACK: &str = "quest track row does not equal the effect of its receipt";
const TRACK_ROW: &str = "quest track cannot be deleted, rekeyed or rewritten at its revision";
const STATE: &str = "quest state is not written by a receipt of its quest";
const STATE_ROW: &str = "quest state cannot be deleted, rekeyed, repinned or uncompleted";
const OBLIGATION_RECEIPT: &str =
    "quest obligation receipt must consume the obligation of its claim";
const IMMUTABLE: &str = "Character first-slice authority history is immutable";
const TRUNCATE: &str = "Character authority relations cannot be truncated";
const CHAIN: &str = "Character progression revision/receipt chain is inconsistent";

#[test]
fn quest_transitions_commit_replay_and_refuse_writing_nothing() -> TestResult {
    run("quest_writer", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let catalogue = catalogue()?;
        let copy = harness
            .root
            .read_character_quest_state(&authority, character)
            .await
            .map_err(debug)?;
        assert!(copy.tracks().is_empty() && copy.states().is_empty());

        // r2: the quest's first transition writes its state, pinned to the current content
        // revision and definition hash, and SET_NOW reads the database transaction time.
        let start = request("start", QuestCause::Command(command(1)?));
        let first =
            committed(commit(harness, &authority, fence(1)?, start.clone(), &catalogue).await)?;
        assert_eq!(first.committed_character_revision.get(), 2);
        assert_eq!(first.quest_key, QUEST);
        assert_eq!(first.pinned_content_revision, "content-1");
        assert_eq!(
            Some(first.definition_hash),
            catalogue.definition_hash(QUEST)
        );
        assert_eq!(
            first.changes[0],
            QuestTrackChange {
                track: STAGE.into(),
                before: -1,
                after: 1
            }
        );
        let started_at = first.changes[1].after;
        let database_now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
                .fetch_one(pool)
                .await?;
        assert!((database_now - 5..=database_now).contains(&started_at));
        assert_eq!(track_value(pool, STAGE).await?, Some(1));

        // Exact replay returns the receipt, even at the stale revision; the same key under
        // another cause kind conflicts, before anything else is read.
        assert_eq!(
            commit(harness, &authority, fence(1)?, start.clone(), &catalogue).await,
            Ok(QuestTransitionOutcome::AlreadyCommitted(first.clone()))
        );
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(2)?,
                request("start", QuestCause::Use(command(1)?)),
                &catalogue
            )
            .await,
            Err(debug(CharacterProgressionError::ConflictingOccurrence))
        );
        assert_eq!(
            harness
                .root
                .reconcile_character_quest_transition(&authority, character, start.clone())
                .await
                .map_err(debug)?,
            Some(first.clone())
        );
        assert_eq!(
            harness
                .root
                .reconcile_character_quest_transition(
                    &authority,
                    character,
                    request("start", QuestCause::Command(command(99)?))
                )
                .await
                .map_err(debug)?,
            None
        );

        // Each refusal code writes nothing (§4 "Validation").
        let before = snapshot(pool).await?;
        let changed = catalogue_with(2, "content-1")?;
        for (name, catalogue, expected) in [
            ("start", &catalogue, QuestRefusal::StageMismatch),
            ("wait", &catalogue, QuestRefusal::StageMismatch),
            ("computed", &catalogue, QuestRefusal::NotSupported),
            ("overflow", &catalogue, QuestRefusal::OutOfRange),
            ("step", &changed, QuestRefusal::RevisionMismatch),
        ] {
            let outcome = commit(
                harness,
                &authority,
                fence(2)?,
                request(name, QuestCause::Command(command(10)?)),
                catalogue,
            )
            .await;
            assert_eq!(
                outcome,
                Ok(QuestTransitionOutcome::Refused(expected)),
                "{name}"
            );
            assert_eq!(snapshot(pool).await?, before, "{name} wrote nothing");
        }
        // Rejected before any write: an unknown transition, an over-long key, another content
        // revision, a stale revision and a respawn still pending.
        let long = QuestTransitionRequest {
            transition_key: format!("oteryn:{}", "k".repeat(122)),
            cause: QuestCause::Command(command(11)?),
        };
        for (outcome, expected, case) in [
            (
                commit(
                    harness,
                    &authority,
                    fence(2)?,
                    request("unknown", QuestCause::Command(command(11)?)),
                    &catalogue,
                )
                .await,
                CharacterProgressionError::InvalidInput,
                "unknown transition",
            ),
            (
                commit(harness, &authority, fence(2)?, long, &catalogue).await,
                CharacterProgressionError::InvalidInput,
                "RL-06",
            ),
            (
                commit(
                    harness,
                    &authority,
                    fence(2)?,
                    request("step", QuestCause::Command(command(11)?)),
                    &catalogue_with(1, "content-2")?,
                )
                .await,
                CharacterProgressionError::ProgressionContextMismatch,
                "another content revision",
            ),
            (
                commit(
                    harness,
                    &authority,
                    fence(1)?,
                    request("step", QuestCause::Command(command(11)?)),
                    &catalogue,
                )
                .await,
                CharacterProgressionError::CharacterRevisionMismatch,
                "stale revision",
            ),
        ] {
            assert_eq!(outcome, Err(debug(expected)), "{case}");
            assert_eq!(snapshot(pool).await?, before, "{case} wrote nothing");
        }

        // r3, r4: steps up to the bound of the track, then the completing transition.
        let step = committed(
            commit(
                harness,
                &authority,
                fence(2)?,
                request("step", QuestCause::Command(command(12)?)),
                &catalogue,
            )
            .await,
        )?;
        assert_eq!(step.changes[0].after, 2);
        let death = ExperienceRewardOccurrence::from_bytes(id(70)).map_err(debug)?;
        committed(
            commit(
                harness,
                &authority,
                fence(3)?,
                request("step", QuestCause::CreatureDeath(death)),
                &catalogue,
            )
            .await,
        )?;
        let finish = committed(
            commit(
                harness,
                &authority,
                fence(4)?,
                request("finish", QuestCause::Command(command(13)?)),
                &catalogue,
            )
            .await,
        )?;
        assert!(finish.completes);
        assert_eq!(track_value(pool, STAGE).await?, Some(10));
        assert_eq!(
            snapshot(pool).await?.split('|').nth(4),
            Some(format!("{QUEST}:2:5:5").as_str())
        );
        // The completed quest's stage no longer admits a step; it keeps its pin and no longer
        // blocks a changed definition (§6).
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(5)?,
                request("step", QuestCause::Command(command(14)?)),
                &catalogue,
            )
            .await,
            Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))
        );
        let again = committed(
            commit(
                harness,
                &authority,
                fence(5)?,
                request("again", QuestCause::Command(command(15)?)),
                &changed,
            )
            .await,
        )?;
        assert_eq!(
            Some(again.definition_hash),
            catalogue.definition_hash(QUEST)
        );
        assert_ne!(
            changed.definition_hash(QUEST),
            catalogue.definition_hash(QUEST)
        );

        // The admission load and the integrity check of the whole chain.
        let copy = harness
            .root
            .read_character_quest_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(copy.tracks().get(STAGE), Some(&10));
        assert_eq!(copy.tracks().get(TIMER), Some(&started_at));
        assert!(copy.states()[QUEST].completed);
        drop(authority);
        harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn quest_receipts_join_the_revision_chain_with_the_other_receipt_kinds() -> TestResult {
    run("quest_chain", async |harness| {
        let pool = &harness.pool;
        let catalogue = catalogue()?;
        // Revision one has no quest receipt, track or state.
        expect_rejected(
            pool,
            "track r1",
            &format!(
                "INSERT INTO game_character_quest_tracks VALUES ({}, '{STAGE}', '{QUEST}', 1, 2);",
                uuid(CHARACTER)
            ),
            TRACK,
        )
        .await?;
        expect_rejected(
            pool,
            "quest receipt at r1 without a successor",
            &Receipt::new(1, 1, &[(STAGE, -1, 1)]).insert(),
            AT_ROOT,
        )
        .await?;

        // r2 XP, r3 quest (writer), r4 stance, r5 quest (exact writer SQL), r6 quest (writer):
        // one gap-free chain across kinds, each `before` equal to its predecessor's `after`.
        expect_committed(pool, "XP r2", &award(60, 1, 1000, 1010)).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer
            .acquire(CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?)
            .await;
        let r3 = slot
            .commit_quest_transition(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                request("start", QuestCause::Command(command(1)?)),
                Arc::clone(&catalogue),
            )
            .await;
        assert_eq!(
            committed(r3)?.committed_character_revision.get(),
            3,
            "the slot loaded its cursor from the root"
        );
        drop(slot);
        expect_committed(pool, "stance r4", &toggle(61, 3, 1010, "guard")).await?;
        let mut sql = Receipt::new(4, 2, &[(STAGE, 1, 2)]);
        sql.experience = 1010;
        sql.pin = ("content-1", catalogue.definition_hash(QUEST).ok_or("hash")?);
        expect_committed(
            pool,
            "quest r5 as the writer writes it",
            &sql.continue_quest(),
        )
        .await?;
        let mut slot = sequencer
            .acquire(CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?)
            .await;
        let r6 = committed(
            slot.commit_quest_transition(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                request("step", QuestCause::Command(command(3)?)),
                Arc::clone(&catalogue),
            )
            .await,
        )?;
        assert_eq!(r6.committed_character_revision.get(), 6);
        assert_eq!(
            r6.changes[0].before, 2,
            "the writer continues from the SQL receipt"
        );
        drop(slot);
        drop(authority);
        harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // A quest receipt must keep level and experience and continue the chain.
        let mut forged = Receipt::new(6, 9, &[(STAGE, 3, 4)]);
        forged.experience = 1011;
        expect_rejected(pool, "experience jump", &forged.continue_quest(), CHAIN).await?;
        drop(seal);
        Ok(())
    })
}

#[test]
fn the_slot_retries_a_bypassed_quest_write_once_and_the_guard_catches_the_bypass() -> TestResult {
    run("quest_bypass", async |harness| {
        let pool = &harness.pool;
        let catalogue = catalogue()?;
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
            1
        );
        // A writer that bypasses the sequencer moves the root under the held slot.
        expect_committed(pool, "bypass XP r2", &award(60, 1, 1000, 1010)).await?;
        // The quest binding excludes the revision: the slot reloads the cursor and the one
        // retry commits at r3 (§5.2).
        let retried = committed(
            slot.commit_quest_transition(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                request("start", QuestCause::Command(command(1)?)),
                Arc::clone(&catalogue),
            )
            .await,
        )?;
        assert_eq!(retried.original_character_revision.get(), 2);
        assert_eq!(retried.committed_character_revision.get(), 3);
        drop(slot);

        // Directly at a stale revision, the fence refuses and nothing is written; a forged
        // receipt at a stale revision is rejected by the guard.
        let before = snapshot(pool).await?;
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(2)?,
                request("step", QuestCause::Command(command(2)?)),
                &catalogue,
            )
            .await,
            Err(debug(CharacterProgressionError::CharacterRevisionMismatch))
        );
        assert_eq!(snapshot(pool).await?, before);
        let mut stale = Receipt::new(1, 2, &[(STAGE, 1, 2)]);
        stale.experience = 1010;
        expect_rejected(
            pool,
            "receipt at a stale revision",
            &stale.insert(),
            AT_ROOT,
        )
        .await?;
        let script = format!("{}{}", stale.insert(), stale.tracks());
        expect_rejected(
            pool,
            "track moved back to a stale receipt",
            &script,
            TRACK_ROW,
        )
        .await?;
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn a_same_session_reconnect_commits_and_a_replaced_session_is_refused() -> TestResult {
    run("quest_reconnect", async |harness| {
        let pool = &harness.pool;
        let catalogue = catalogue()?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let pending = request("start", QuestCause::Command(command(1)?));
        // A reconnect in the same GameSession: the pending cause commits with the current
        // generation; the old generation's fence is refused.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation = 2")
            .execute(pool)
            .await?;
        let before = snapshot(pool).await?;
        assert_eq!(
            commit(harness, &authority, fence(1)?, pending.clone(), &catalogue).await,
            Err(debug(CharacterProgressionError::AuthorityRejected))
        );
        assert_eq!(snapshot(pool).await?, before);
        let current = CurrentCharacterGameplayFence {
            connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
            ..fence(1)?
        };
        committed(commit(harness, &authority, current, pending, &catalogue).await)?;
        // A command of a replaced GameSession is refused before any write.
        let before = snapshot(pool).await?;
        for cause in [
            QuestCause::Command(command_in(51, 1)?),
            QuestCause::Use(command_in(51, 2)?),
        ] {
            assert_eq!(
                commit(
                    harness,
                    &authority,
                    current,
                    request("step", cause),
                    &catalogue
                )
                .await,
                Err(debug(CharacterProgressionError::AuthorityRejected))
            );
        }
        // An ended session cannot commit, even its own command.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3")
            .execute(pool)
            .await?;
        assert_eq!(
            commit(
                harness,
                &authority,
                CurrentCharacterGameplayFence {
                    expected_character_revision: crate::domain::CharacterRevision::new(2)
                        .map_err(debug)?,
                    ..current
                },
                request("step", QuestCause::Command(command(2)?)),
                &catalogue,
            )
            .await,
            Err(debug(CharacterProgressionError::AuthorityRejected))
        );
        assert_eq!(snapshot(pool).await?, before);
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn quest_guards_reject_rows_without_their_receipt() -> TestResult {
    run("quest_guards", async |harness| {
        let pool = &harness.pool;
        // r2 starts the quest as the writer does.
        let start = Receipt::new(1, 1, &[(STAGE, -1, 1), (TIMER, 0, 100)]);
        expect_committed(pool, "start r2", &start.start()).await?;
        let next = Receipt::new(2, 2, &[(STAGE, 1, 2)]);

        // A receipt whose track row is not written, written with another value, or not
        // continuing from the stored value; a track row of another quest.
        expect_rejected(
            pool,
            "receipt without its track row",
            &format!(
                "{}{}{}",
                advance(2, 1000),
                next.insert(),
                next.state_update()
            ),
            RECEIPT,
        )
        .await?;
        let mut wrong_before = next.clone();
        wrong_before.effects[0].1 = 0;
        expect_rejected(
            pool,
            "before is not the stored value",
            &wrong_before.continue_quest(),
            TRACK,
        )
        .await?;
        let mut foreign = next.clone();
        foreign.quest = OTHER.into();
        expect_rejected(
            pool,
            "a track of another quest",
            &format!("{}{}", foreign.continue_quest(), foreign.state_insert()),
            RECEIPT,
        )
        .await?;
        let mut duplicate = next.clone();
        duplicate.effects.push((STAGE.into(), 1, 2));
        expect_rejected(
            pool,
            "the same track twice",
            &format!(
                "{}{}{}",
                advance(2, 1000),
                duplicate.insert(),
                next.tracks()
            ),
            DISTINCT,
        )
        .await?;
        // A quest state not written, of another pin, or completed by a receipt that does not
        // complete; a completing receipt whose state is not completed.
        expect_rejected(
            pool,
            "receipt without its state",
            &format!("{}{}{}", advance(2, 1000), next.insert(), next.tracks()),
            STATE_OF_RECEIPT,
        )
        .await?;
        let mut repinned = next.clone();
        repinned.pin = ("content-2", [7; 32]);
        expect_rejected(
            pool,
            "receipt of another pin",
            &repinned.continue_quest(),
            STATE_OF_RECEIPT,
        )
        .await?;
        let mut completes = next.clone();
        completes.completes = true;
        expect_rejected(
            pool,
            "completing receipt, state not completed",
            &format!(
                "{}{}{}{}",
                advance(2, 1000),
                completes.insert(),
                completes.tracks(),
                next.state_update()
            ),
            STATE_OF_RECEIPT,
        )
        .await?;
        expect_rejected(
            pool,
            "state completed without a completing receipt",
            &format!(
                "{}{}{}{}",
                advance(2, 1000),
                next.insert(),
                next.tracks(),
                completes.state_update()
            ),
            STATE,
        )
        .await?;
        // An obligation cause without its claim.
        let mut obligation = next.clone();
        obligation.cause = ("claim_obligation", 52, 1);
        expect_rejected(
            pool,
            "obligation receipt without a claim",
            &obligation.continue_quest(),
            OBLIGATION_RECEIPT,
        )
        .await?;

        // Row guards: tracks and states are never deleted, rekeyed, rewritten at their
        // revision, repinned or uncompleted; receipts are immutable; nothing is truncated.
        let character = uuid(CHARACTER);
        for (case, script, rule) in [
            (
                "delete a track",
                format!(
                    "DELETE FROM game_character_quest_tracks WHERE character_id = {character};"
                ),
                TRACK_ROW,
            ),
            (
                "rewrite a track at its revision",
                format!(
                    "UPDATE game_character_quest_tracks SET value = 5 WHERE track_key = '{STAGE}';"
                ),
                TRACK_ROW,
            ),
            (
                "move a track forward without a receipt",
                format!(
                    "UPDATE game_character_quest_tracks SET value = 5, \
                       committed_character_revision = 3 WHERE track_key = '{STAGE}';"
                ),
                TRACK,
            ),
            (
                "delete a state",
                "DELETE FROM game_character_quest_states;".to_owned(),
                STATE_ROW,
            ),
            (
                "repin a state",
                "UPDATE game_character_quest_states SET pinned_content_revision = 'content-2', \
                   committed_character_revision = 3;"
                    .to_owned(),
                STATE_ROW,
            ),
            (
                "update a receipt",
                "UPDATE game_character_quest_receipts SET completes = true;".to_owned(),
                IMMUTABLE,
            ),
            (
                "delete a receipt",
                "DELETE FROM game_character_quest_receipts;".to_owned(),
                IMMUTABLE,
            ),
        ] {
            expect_rejected(pool, case, &script, rule).await?;
        }
        for relation in [
            "game_character_quest_receipts",
            "game_character_quest_tracks",
            "game_character_quest_states",
            "game_character_quest_obligations",
        ] {
            expect_rejected(pool, relation, &format!("TRUNCATE {relation};"), TRUNCATE).await?;
        }

        // The exact writer shape continues the quest, and completes it.
        expect_committed(pool, "continue r3", &next.continue_quest()).await?;
        let mut finish = Receipt::new(3, 3, &[(STAGE, 2, 10)]);
        finish.completes = true;
        expect_committed(pool, "complete r4", &finish.continue_quest()).await?;
        expect_rejected(
            pool,
            "uncomplete a state",
            "UPDATE game_character_quest_states SET completed_character_revision = NULL, \
               committed_character_revision = 5;",
            STATE_ROW,
        )
        .await?;
        // A transition that only completes has no effect; one with none that does not
        // complete, nine effects, a binding over RL-04 or a key over RL-06 is refused.
        let mut empty = Receipt::new(4, 4, &[]);
        empty.completes = true;
        expect_committed(pool, "completes only r5", &empty.continue_quest()).await?;
        let shape = "game_character_quest_receipt_effects_shape";
        let mut nothing = Receipt::new(5, 5, &[]);
        nothing.completes = false;
        expect_rejected(pool, "no effect", &nothing.continue_quest(), shape).await?;
        let keys: Vec<String> = (0..9)
            .map(|index| format!("oteryn:quest-progress/fixture.rats.k{index}"))
            .collect();
        let nine: Vec<(&str, i64, i64)> = keys.iter().map(|key| (key.as_str(), 0, 1)).collect();
        expect_rejected(
            pool,
            "RL-02",
            &Receipt::new(5, 5, &nine).continue_quest(),
            shape,
        )
        .await?;
        let mut binding = Receipt::new(5, 5, &[(STAGE, 10, 10)]);
        binding.binding = vec![1; 1025];
        expect_rejected(
            pool,
            "RL-04",
            &binding.continue_quest(),
            "game_character_quest_receipts_request_binding_check",
        )
        .await?;
        let long = format!("oteryn:{}", "k".repeat(122));
        expect_rejected(
            pool,
            "RL-06",
            &format!(
                "INSERT INTO game_character_quest_tracks VALUES ({character}, '{long}', \
                   '{QUEST}', 0, 6);"
            ),
            "game_character_quest_tracks_track_key_check",
        )
        .await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

/// Seed `receipts` quests through the exact writer SQL, each started by one receipt that writes
/// `tracks` tracks: one transaction per receipt, from the current root revision.
async fn seed_quests(pool: &sqlx::PgPool, prefix: &str, receipts: u32, tracks: u32) -> TestResult {
    let mut original: u64 = sqlx::query_scalar::<_, String>(
        "SELECT character_revision::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(pool)
    .await?
    .parse()?;
    let mut connection = pool.acquire().await?;
    for n in 1..=receipts {
        let keys: Vec<String> = (1..=tracks)
            .map(|track| format!("oteryn:seed/{prefix}.k{n}.{track}"))
            .collect();
        let effects: Vec<(&str, i64, i64)> = keys.iter().map(|key| (key.as_str(), 0, 1)).collect();
        let mut receipt = Receipt::new(original, u64::from(n), &effects);
        receipt.cause = ("creature_death", 71, 0);
        receipt.transition = format!("oteryn:seed/{prefix}.t{n}");
        receipt.quest = format!("oteryn:seed/{prefix}.q{n}");
        let mut tx = sqlx::Connection::begin(&mut *connection).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(receipt.start()))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        original += 1;
    }
    Ok(())
}

async fn revision(harness: &Harness) -> TestResult<u64> {
    Ok(harness.root_revision().await?.parse()?)
}

#[test]
fn quest_states_are_bounded_at_rl_05() -> TestResult {
    run("quest_rl05", async |harness| {
        let pool = &harness.pool;
        let catalogue = catalogue()?;
        seed_quests(pool, "s", 1023, 1).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // The 1024th quest state commits; a 1025th is refused writing nothing.
        let at = revision(harness).await?;
        committed(
            commit(
                harness,
                &authority,
                fence(at)?,
                request("start", QuestCause::Command(command(1)?)),
                &catalogue,
            )
            .await,
        )?;
        let before = snapshot(pool).await?;
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(at + 1)?,
                request("other", QuestCause::Command(command(2)?)),
                &catalogue,
            )
            .await,
            Ok(QuestTransitionOutcome::Refused(
                QuestRefusal::CapacityExceeded
            ))
        );
        assert_eq!(snapshot(pool).await?, before);
        // An existing quest still moves at the bound.
        committed(
            commit(
                harness,
                &authority,
                fence(at + 1)?,
                request("step", QuestCause::Command(command(3)?)),
                &catalogue,
            )
            .await,
        )?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let copy = harness
            .root
            .read_character_quest_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(copy.states().len(), 1024, "the load holds RL-05 and RL-08");
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn quest_tracks_are_bounded_at_rl_01_and_the_load_fails_closed_over_it() -> TestResult {
    run("quest_rl01", async |harness| {
        let pool = &harness.pool;
        let catalogue = catalogue()?;
        // 511 x 8 + 6 = 4094 tracks; the fixture quest's start adds two: 4096.
        seed_quests(pool, "a", 511, 8).await?;
        seed_quests(pool, "b", 1, 6).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let at = revision(harness).await?;
        committed(
            commit(
                harness,
                &authority,
                fence(at)?,
                request("start", QuestCause::Command(command(1)?)),
                &catalogue,
            )
            .await,
        )?;
        assert_eq!(harness.count("game_character_quest_tracks").await?, 4096);
        let before = snapshot(pool).await?;
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(at + 1)?,
                request("other", QuestCause::Command(command(2)?)),
                &catalogue,
            )
            .await,
            Ok(QuestTransitionOutcome::Refused(
                QuestRefusal::CapacityExceeded
            ))
        );
        assert_eq!(snapshot(pool).await?, before);
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let copy = harness
            .root
            .read_character_quest_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(
            copy.tracks().len(),
            4096,
            "at RL-01 and RL-08 the load holds"
        );
        // One row over the bound (written outside the guards) fails the load closed.
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_character_quest_tracks VALUES ({}, 'oteryn:seed/over', \
               '{OTHER}', 1, {at})",
            uuid(CHARACTER)
        )))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        assert!(matches!(
            harness
                .root
                .read_character_quest_state(&authority, character)
                .await,
            Err(CharacterProgressionError::Unavailable(_))
        ));
        drop(authority);
        drop(seal);
        Ok(())
    })
}

// QUEST-XP-1 quest XP obligations (migration 0069) and their award, on the fixtures above.
#[path = "quest_xp_postgres_cases.rs"]
mod quest_xp_postgres_cases;

#[test]
fn source_herald_grouped_writer_death_replay_ineligible_and_stale_session_are_atomic() -> TestResult
{
    run("source_herald", async |harness| {
        let mission =
            "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
        let killed =
            "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/herald_killed";
        let source_key = "oteryn:quest-transition/crystalserver/targuna/herald-death/mission-1";
        let lowered = crate::durability::quest_state::quest::loader::load_embedded_quest_state(
            "source-herald-crystal00ce-r1",
        )
        .map_err(debug)?;
        let catalogue = Arc::new(lowered.catalogue().clone());
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = QuestTransitionRequest {
            transition_key: source_key.into(),
            cause: QuestCause::CreatureDeath(
                ExperienceRewardOccurrence::from_bytes(id(71)).map_err(debug)?,
            ),
        };
        let before = snapshot(&harness.pool).await?;
        assert_eq!(
            commit(harness, &authority, fence(1)?, request.clone(), &catalogue).await,
            Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))
        );
        assert_eq!(snapshot(&harness.pool).await?, before);
        let start=QuestTransitionRequest {transition_key:"oteryn:quest-transition/crystalserver/quest/u15_24/targuna/burning_heart/mission/npc_1".into(),cause:QuestCause::Command(command(51)?)};
        committed(commit(harness, &authority, fence(1)?, start, &catalogue).await)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer
            .acquire(CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?)
            .await;
        let first = committed(
            slot.commit_quest_transition(
                &harness.root,
                &authority,
                &harness.node,
                fence(2)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await,
        )?;
        assert_eq!(track_value(&harness.pool, mission).await?, Some(3));
        assert_eq!(track_value(&harness.pool, killed).await?, Some(1));
        assert_eq!(first.changes.len(), 2);
        let applied = snapshot(&harness.pool).await?;
        assert_eq!(
            slot.commit_quest_transition(
                &harness.root,
                &authority,
                &harness.node,
                fence(2)?,
                request.clone(),
                Arc::clone(&catalogue)
            )
            .await
            .map_err(debug),
            Ok(QuestTransitionOutcome::AlreadyCommitted(first))
        );
        assert_eq!(snapshot(&harness.pool).await?, applied);
        drop(slot);
        let other = QuestTransitionRequest {
            transition_key: source_key.into(),
            cause: QuestCause::CreatureDeath(
                ExperienceRewardOccurrence::from_bytes(id(72)).map_err(debug)?,
            ),
        };
        let mut stale = fence(3)?;
        stale.game_session_id = GameSessionId::decode(&id(99)).map_err(debug)?;
        assert!(
            commit(harness, &authority, stale, other.clone(), &catalogue)
                .await
                .is_err()
        );
        assert_eq!(snapshot(&harness.pool).await?, applied);
        assert_eq!(
            commit(harness, &authority, fence(3)?, other, &catalogue).await,
            Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))
        );
        assert_eq!(snapshot(&harness.pool).await?, applied);
        Ok(())
    })
}

/// QUEST-CAT-BOOT-1: the embedded quest catalogue the node loads at boot, at `content_revision`.
fn embedded_catalogue(content_revision: &str) -> TestResult<Arc<QuestStateCatalogue>> {
    let lowered =
        crate::durability::quest_state::quest::loader::load_embedded_quest_state(content_revision)
            .map_err(debug)?;
    Ok(Arc::new(lowered.catalogue().clone()))
}

/// The first embedded transition the catalogue applies from the initial values without an
/// experience reward or a clock comparison, and the first it refuses `NOT_SUPPORTED`.
fn embedded_transitions(catalogue: &QuestStateCatalogue) -> TestResult<(String, String)> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../content/quests/missions/quest-state.json"
    ))?;
    let (mut exact, mut computed) = (None, None);
    for quest in document["quests"].as_array().ok_or("quests")? {
        for transition in quest["transitions"].as_array().ok_or("transitions")? {
            let key = transition["key"].as_str().ok_or("key")?;
            let loaded = catalogue.transition(key).ok_or("transition")?;
            match catalogue.evaluate(loaded, &std::collections::BTreeMap::new(), 0) {
                Ok(_)
                    if loaded.experience.is_none()
                        && !loaded.effects.iter().any(|effect| {
                            matches!(effect.from, QuestComparison::ElapsedAtLeast(_))
                        }) =>
                {
                    exact.get_or_insert_with(|| key.to_owned());
                }
                Err(QuestRefusal::NotSupported) => {
                    computed.get_or_insert_with(|| key.to_owned());
                }
                _ => {}
            }
        }
    }
    Ok((
        exact.ok_or("no exact transition")?,
        computed.ok_or("no computed transition")?,
    ))
}

#[test]
fn the_embedded_boot_catalogue_applies_and_refuses_on_the_writer() -> TestResult {
    run("quest_embedded", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let catalogue = embedded_catalogue("content-1")?;
        let (exact, computed) = embedded_transitions(&catalogue)?;
        let embedded = |key: &str, command_id: u64| -> TestResult<QuestTransitionRequest> {
            Ok(QuestTransitionRequest {
                transition_key: key.to_owned(),
                cause: QuestCause::Command(command(command_id)?),
            })
        };

        // Another content revision than the Character's is refused before any read.
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(1)?,
                embedded(&exact, 1)?,
                &embedded_catalogue("content-2")?,
            )
            .await,
            Err(debug(CharacterProgressionError::ProgressionContextMismatch))
        );
        // A `Computed` transition still writes nothing.
        assert_eq!(
            commit(
                harness,
                &authority,
                fence(1)?,
                embedded(&computed, 2)?,
                &catalogue
            )
            .await,
            Ok(QuestTransitionOutcome::Refused(QuestRefusal::NotSupported))
        );
        // An exact transition of the boot catalogue commits at the Character's revision.
        let applied = committed(
            commit(
                harness,
                &authority,
                fence(1)?,
                embedded(&exact, 3)?,
                &catalogue,
            )
            .await,
        )?;
        assert_eq!(applied.committed_character_revision.get(), 2);
        assert_eq!(applied.pinned_content_revision, "content-1");
        assert_eq!(
            Some(applied.definition_hash),
            catalogue.definition_hash(&applied.quest_key)
        );
        Ok(())
    })
}
