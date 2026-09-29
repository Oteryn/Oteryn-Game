// CHARM-2 Bestiary kill progress (migration 0018,
// `durability::bestiary_progress`). Every wrapper provides the same
// path-loaded crate root and the `bestiary_postgres_harness` module.

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, context, debug, fence, id, register,
    runtime,
};
use crate::domain::CharacterId;
use crate::domain::bestiary::BestiaryRace;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ConnectionGeneration, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::Executor;
use std::future::Future;
use std::task::Poll;

const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";

async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
where
    A: Future,
    B: Future,
{
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut first_output = None;
    let mut second_output = None;
    std::future::poll_fn(move |context| {
        if first_output.is_none()
            && let Poll::Ready(output) = first.as_mut().poll(context)
        {
            first_output = Some(output);
        }
        if second_output.is_none()
            && let Poll::Ready(output) = second.as_mut().poll(context)
        {
            second_output = Some(output);
        }
        match (first_output.take(), second_output.take()) {
            (Some(first), Some(second)) => Poll::Ready((first, second)),
            (first, second) => {
                first_output = first;
                second_output = second;
                Poll::Pending
            }
        }
    })
    .await
}

/// A small bound (3) so saturation is reachable in a test.
fn race(key: &str) -> TestResult<BestiaryRace> {
    BestiaryRace::new(key, "definition-r1", vec![1, 2, 3]).map_err(|error| debug(error).into())
}

fn kill(tag: u8, key: &str) -> TestResult<BestiaryKillRequest> {
    Ok(BestiaryKillRequest {
        occurrence: BestiaryKillOccurrence::from_bytes(id(tag)).map_err(debug)?,
        race: race(key)?,
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
    })
}

fn award(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
    let context = context();
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

fn character() -> TestResult<CharacterId> {
    CharacterId::from_bytes(id(CHARACTER)).map_err(|error| debug(error).into())
}

fn committed(outcome: BestiaryKillOutcome) -> TestResult<(u32, u32, u64)> {
    match outcome {
        BestiaryKillOutcome::Committed(kill) => Ok((
            kill.kill_count_before,
            kill.kill_count_after,
            kill.committed_character_revision.get(),
        )),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

#[test]
fn kills_count_per_race_replay_saturate_and_share_the_revision_chain_with_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "count", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let root = &harness.root;
        let node = &harness.node;

        let first = root
            .commit_bestiary_kill(&authority, node, fence(1)?, kill(60, RAT)?)
            .await
            .map_err(debug)?;
        let BestiaryKillOutcome::Committed(first) = first else {
            return Err("first kill was not committed".into());
        };
        assert_eq!((first.kill_count_before, first.kill_count_after), (0, 1));
        assert_eq!(first.original_character_revision.get(), 1);
        assert_eq!(first.committed_character_revision.get(), 2);
        assert_eq!(first.final_kill_threshold, 3);
        assert_eq!(harness.root_revision().await?, "2");
        let state_revision: String = sqlx::query_scalar(
            "SELECT character_revision::text FROM game_character_progression_state",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(state_revision, "2");

        // Exact replay, even under a different expected revision, resolves
        // to the retained receipt; changed semantics conflict.
        for revision in [1, 2, 9] {
            assert_eq!(
                root.commit_bestiary_kill(&authority, node, fence(revision)?, kill(60, RAT)?)
                    .await
                    .map_err(debug)?,
                BestiaryKillOutcome::AlreadyCommitted(first.clone())
            );
        }
        let other_race = kill(60, CAVE_RAT)?;
        let mut other_thresholds = kill(60, RAT)?;
        other_thresholds.race =
            BestiaryRace::new(RAT, "definition-r1", vec![1, 2, 4]).map_err(debug)?;
        let mut other_revision = kill(60, RAT)?;
        other_revision.race =
            BestiaryRace::new(RAT, "definition-r2", vec![1, 2, 3]).map_err(debug)?;
        for changed in [other_race, other_thresholds, other_revision] {
            assert!(matches!(
                root.commit_bestiary_kill(&authority, node, fence(2)?, changed)
                    .await,
                Err(BestiaryProgressError::ConflictingOccurrence)
            ));
        }

        assert_eq!(
            committed(
                root.commit_bestiary_kill(&authority, node, fence(2)?, kill(61, RAT)?)
                    .await
                    .map_err(debug)?
            )?,
            (1, 2, 3)
        );
        assert_eq!(
            committed(
                root.commit_bestiary_kill(&authority, node, fence(3)?, kill(62, RAT)?)
                    .await
                    .map_err(debug)?
            )?,
            (2, 3, 4)
        );
        // At the final threshold the kill is saturated: no receipt, no row
        // change and no revision.
        assert_eq!(
            root.commit_bestiary_kill(&authority, node, fence(4)?, kill(63, RAT)?)
                .await
                .map_err(debug)?,
            BestiaryKillOutcome::Saturated {
                race_key: RAT.into(),
                kill_count: 3
            }
        );
        assert_eq!(harness.root_revision().await?, "4");
        assert_eq!(
            harness
                .count("game_character_bestiary_kill_receipts")
                .await?,
            3
        );

        // Races count independently.
        assert_eq!(
            committed(
                root.commit_bestiary_kill(&authority, node, fence(4)?, kill(64, CAVE_RAT)?)
                    .await
                    .map_err(debug)?
            )?,
            (0, 1, 5)
        );
        // An XP award and a kill share one revision chain.
        let xp = root
            .commit_character_experience(&authority, node, fence(5)?, award(65)?)
            .await
            .map_err(debug)?;
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err("XP award was not committed".into());
        };
        assert_eq!(xp.committed_character_revision.get(), 6);
        assert_eq!(
            committed(
                root.commit_bestiary_kill(&authority, node, fence(6)?, kill(66, CAVE_RAT)?)
                    .await
                    .map_err(debug)?
            )?,
            (1, 2, 7)
        );
        let (level, experience): (i64, i64) = sqlx::query_as(
            "SELECT level_after, experience_after FROM game_character_bestiary_kill_receipts \
              WHERE committed_character_revision = 7",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!((level, experience), (50, 1005));

        let reconciled = root
            .reconcile_bestiary_kill(
                &authority,
                BestiaryKillOccurrence::from_bytes(id(60)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(reconciled, Some(first.clone()));
        assert_eq!(
            root.reconcile_bestiary_kill(
                &authority,
                BestiaryKillOccurrence::from_bytes(id(63)).map_err(debug)?,
            )
            .await
            .map_err(debug)?,
            None
        );
        let rat = root
            .read_bestiary_progress(&authority, character()?, RAT)
            .await
            .map_err(debug)?
            .ok_or("missing rat progress")?;
        assert_eq!(
            (rat.kill_count, rat.committed_character_revision.get()),
            (3, 4)
        );
        let rat_race = race(RAT)?;
        assert_eq!(rat_race.stages_unlocked(rat.kill_count), 3);
        assert!(rat_race.is_complete(rat.kill_count));
        let cave_rat = root
            .read_bestiary_progress(&authority, character()?, CAVE_RAT)
            .await
            .map_err(debug)?
            .ok_or("missing cave rat progress")?;
        assert_eq!(cave_rat.kill_count, 2);
        assert_eq!(
            root.read_bestiary_progress(&authority, character()?, "oteryn:creature.wolf")
                .await
                .map_err(debug)?,
            None
        );

        // A replay after the session ended still resolves (no authority is
        // reacquired); a new kill is refused.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
             WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(SESSION).as_slice())
        .execute(&harness.pool)
        .await?;
        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, fence(1)?, kill(60, RAT)?)
                .await,
            Ok(BestiaryKillOutcome::AlreadyCommitted(_))
        ));
        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, fence(7)?, kill(67, CAVE_RAT)?)
                .await,
            Err(BestiaryProgressError::AuthorityRejected)
        ));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn restart_reopens_character_authority_and_reads_back_bestiary_progress() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "restart", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        harness
            .root
            .commit_bestiary_kill(&authority, &harness.node, fence(1)?, kill(60, RAT)?)
            .await
            .map_err(debug)?;
        drop(authority);
        drop(seal);

        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = restarted
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("restart integrity: {error:?}"))?;
        let progress = restarted
            .read_bestiary_progress(&authority, character()?, RAT)
            .await
            .map_err(debug)?
            .ok_or("missing progress after restart")?;
        assert_eq!(progress.kill_count, 1);
        drop(authority);
        drop(seal);
        drop(restarted);
        harness.cleanup().await
    })
}

#[test]
fn stale_or_substituted_fences_context_and_missing_state_fail_closed_with_no_write() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin.clone(), "stale", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let root = &harness.root;
        let node = &harness.node;

        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, fence(2)?, kill(61, RAT)?)
                .await,
            Err(BestiaryProgressError::CharacterRevisionMismatch)
        ));
        let mut stale_connection = fence(1)?;
        stale_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut stale_lease = fence(1)?;
        stale_lease.character_lease_generation = 2;
        let mut stale_scope = fence(1)?;
        stale_scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        let mut other_session = fence(1)?;
        other_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        let mut other_character = fence(1)?;
        other_character.character_id = CharacterId::from_bytes(id(44)).map_err(debug)?;
        for (tag, substituted) in [
            (62, stale_connection),
            (63, stale_lease),
            (64, stale_scope),
            (65, other_session),
            (66, other_character),
        ] {
            assert!(
                matches!(
                    root.commit_bestiary_kill(&authority, node, substituted, kill(tag, RAT)?)
                        .await,
                    Err(BestiaryProgressError::AuthorityRejected)
                ),
                "substituted fence {tag} must be rejected"
            );
        }
        // A node incarnation that does not hold the scope assignment.
        let other_node = register(root, 2).await?;
        assert!(matches!(
            root.commit_bestiary_kill(&authority, &other_node, fence(1)?, kill(67, RAT)?)
                .await,
            Err(BestiaryProgressError::AuthorityRejected)
        ));
        let mut stale_context = kill(68, RAT)?;
        stale_context.context.content = "content-2".into();
        let mut stale_policy = kill(69, RAT)?;
        stale_policy.policy_revision = "policy-2".into();
        let mut stale_reward = kill(70, RAT)?;
        stale_reward.reward_revision = "reward-2".into();
        for request in [stale_context, stale_policy, stale_reward] {
            assert!(matches!(
                root.commit_bestiary_kill(&authority, node, fence(1)?, request)
                    .await,
                Err(BestiaryProgressError::ProgressionContextMismatch)
            ));
        }
        let mut zero_lease = fence(1)?;
        zero_lease.character_lease_generation = 0;
        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, zero_lease, kill(71, RAT)?)
                .await,
            Err(BestiaryProgressError::InvalidInput)
        ));
        assert_eq!(harness.root_revision().await?, "1");
        assert_eq!(
            harness
                .count("game_character_bestiary_kill_receipts")
                .await?,
            0
        );
        assert_eq!(harness.count("game_character_bestiary_progress").await?, 0);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;

        // A bootstrap-only Character has no typed progression to advance.
        let harness = Harness::create(admin, "missing", false).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .root
                .commit_bestiary_kill(&authority, &harness.node, fence(1)?, kill(60, RAT)?)
                .await,
            Err(BestiaryProgressError::MissingProgressionState)
        ));
        assert_eq!(harness.root_revision().await?, "1");
        assert_eq!(harness.count("game_character_bestiary_progress").await?, 0);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn concurrent_kills_serialize_on_the_character_revision() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "concurrent", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let root = &harness.root;
        let node = &harness.node;

        // Two distinct kills against one predecessor revision: the semantic
        // pass admits one and reports the other unavailable (as for XP); its
        // retry then fails on the advanced revision.
        let (left, right) = join_two(
            root.commit_bestiary_kill(&authority, node, fence(1)?, kill(60, RAT)?),
            root.commit_bestiary_kill(&authority, node, fence(1)?, kill(61, RAT)?),
        )
        .await;
        let losing = match (left, right) {
            (
                Ok(BestiaryKillOutcome::Committed(_)),
                Err(BestiaryProgressError::Unavailable(DurabilityError::RootUnavailable)),
            ) => 61,
            (
                Err(BestiaryProgressError::Unavailable(DurabilityError::RootUnavailable)),
                Ok(BestiaryKillOutcome::Committed(_)),
            ) => 60,
            outcomes => return Err(format!("unexpected concurrent outcomes: {outcomes:?}").into()),
        };
        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, fence(1)?, kill(losing, RAT)?)
                .await,
            Err(BestiaryProgressError::CharacterRevisionMismatch)
        ));
        // One occurrence submitted twice at once commits once; its retry
        // replays.
        let (left, right) = join_two(
            root.commit_bestiary_kill(&authority, node, fence(2)?, kill(62, RAT)?),
            root.commit_bestiary_kill(&authority, node, fence(2)?, kill(62, RAT)?),
        )
        .await;
        match (left, right) {
            (
                Ok(BestiaryKillOutcome::Committed(_)),
                Err(BestiaryProgressError::Unavailable(DurabilityError::RootUnavailable)),
            )
            | (
                Err(BestiaryProgressError::Unavailable(DurabilityError::RootUnavailable)),
                Ok(BestiaryKillOutcome::Committed(_)),
            ) => {}
            outcomes => return Err(format!("unexpected concurrent outcomes: {outcomes:?}").into()),
        }
        assert!(matches!(
            root.commit_bestiary_kill(&authority, node, fence(2)?, kill(62, RAT)?)
                .await,
            Ok(BestiaryKillOutcome::AlreadyCommitted(_))
        ));
        assert_eq!(harness.root_revision().await?, "3");
        let count: i64 = sqlx::query_scalar(
            "SELECT kill_count FROM game_character_bestiary_progress WHERE race_key = $1",
        )
        .bind(RAT)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(count, 2);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The exact SQL of one kill at `revision -> revision + 1` for Character 41,
/// with the given counts; the state row is at level 50 / 1000 experience.
fn kill_sql(
    occurrence: u8,
    revision: u64,
    before: i64,
    after: i64,
    final_threshold: i64,
    progress: &str,
) -> String {
    let character = format!("'{}'::uuid", hex(&id(CHARACTER)));
    let occurrence = format!("'{}'::uuid", hex(&id(occurrence)));
    let next = revision + 1;
    format!(
        "UPDATE game_character_roots SET character_revision = {next} \
          WHERE character_id = {character}; \
         UPDATE game_character_progression_state SET character_revision = {next} \
          WHERE character_id = {character}; \
         INSERT INTO game_character_bestiary_kill_receipts VALUES ({occurrence}, '\\x01'::bytea, \
           decode(repeat('00', 32), 'hex'), {character}, {revision}, {next}, 50, 50, 1000, 1000, \
           '{RAT}', 'definition-r1', {final_threshold}, {before}, {after}, 'profile-1', \
           'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', 'declaration-1', \
           'policy-1', 'reward-1', 1); \
         {progress}",
        progress = progress
            .replace("{character}", &character)
            .replace("{occurrence}", &occurrence)
            .replace("{next}", &next.to_string())
            .replace("{after}", &after.to_string()),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

const INSERT_PROGRESS: &str = "INSERT INTO game_character_bestiary_progress VALUES \
    ({character}, 'oteryn:creature.rat', {after}, {next}, {occurrence});";
const UPDATE_PROGRESS: &str = "UPDATE game_character_bestiary_progress \
    SET kill_count = {after}, committed_character_revision = {next}, \
        last_bestiary_occurrence_id = {occurrence} \
    WHERE character_id = {character} AND race_key = 'oteryn:creature.rat';";

async fn transaction(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    tx.execute(sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned())))
        .await?;
    tx.commit().await
}

#[test]
fn direct_sql_cannot_bypass_the_kill_chain_or_the_progress_rows() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "sql", true).await?;
        let pool = &harness.pool;
        let character = format!("'{}'::uuid", hex(&id(CHARACTER)));

        // The exact writer SQL for a first kill is admitted.
        transaction(pool, &kill_sql(60, 1, 0, 1, 3, INSERT_PROGRESS)).await?;

        const CHAIN: &str = "Bestiary progress is inconsistent with its kill receipts";
        const ROW: &str = "Bestiary progress is never deleted or reassigned";
        const CHECK: &str = "violates check constraint";
        const IMMUTABLE: &str = "authority history is immutable";
        const TRUNCATE: &str = "cannot be truncated";
        let rejected = [
            // A row with no receipt at all.
            (
                CHAIN,
                format!(
                    "INSERT INTO game_character_bestiary_progress VALUES \
                     ({character}, 'oteryn:creature.wolf', 1, 2, '{}'::uuid);",
                    hex(&id(61))
                ),
            ),
            // A row-only count change.
            (
                CHAIN,
                format!(
                    "UPDATE game_character_bestiary_progress SET kill_count = 3 \
                      WHERE character_id = {character};"
                ),
            ),
            // A count gap in the race's chain (0 -> 1, then 5 -> 6).
            (CHAIN, kill_sql(62, 2, 5, 6, 10, UPDATE_PROGRESS)),
            // A receipt whose row is not updated.
            (CHAIN, kill_sql(63, 2, 1, 2, 3, "")),
            // A receipt past the saturation bound.
            (CHECK, kill_sql(64, 2, 1, 2, 1, UPDATE_PROGRESS)),
            // A receipt that skips a count.
            (CHECK, kill_sql(65, 2, 1, 3, 3, UPDATE_PROGRESS)),
            // A kill that moves experience.
            (
                CHECK,
                kill_sql(66, 2, 1, 2, 3, UPDATE_PROGRESS)
                    .replace("50, 50, 1000, 1000", "50, 50, 1000, 1001"),
            ),
            // Deleting or moving a row, or rewriting a receipt.
            (
                ROW,
                format!(
                    "DELETE FROM game_character_bestiary_progress \
                      WHERE character_id = {character};"
                ),
            ),
            (
                ROW,
                format!(
                    "UPDATE game_character_bestiary_progress \
                        SET race_key = 'oteryn:creature.wolf' \
                      WHERE character_id = {character};"
                ),
            ),
            (
                IMMUTABLE,
                "UPDATE game_character_bestiary_kill_receipts SET kill_count_after = 1;".to_owned(),
            ),
            (
                IMMUTABLE,
                "DELETE FROM game_character_bestiary_kill_receipts;".to_owned(),
            ),
            (
                TRUNCATE,
                "TRUNCATE game_character_bestiary_progress;".to_owned(),
            ),
            (
                TRUNCATE,
                "TRUNCATE game_character_bestiary_kill_receipts;".to_owned(),
            ),
        ];
        for (expected, script) in &rejected {
            match transaction(pool, script).await {
                Err(error) => assert!(
                    error.to_string().contains(expected),
                    "{script} was rejected for another reason: {error}"
                ),
                Ok(()) => return Err(format!("direct SQL must be rejected: {script}").into()),
            }
        }
        assert_eq!(harness.root_revision().await?, "2");
        assert_eq!(
            harness
                .count("game_character_bestiary_kill_receipts")
                .await?,
            1
        );

        // The next exact kill is admitted after every rejection.
        transaction(pool, &kill_sql(67, 2, 1, 2, 3, UPDATE_PROGRESS)).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT kill_count FROM game_character_bestiary_progress")
                .fetch_one(pool)
                .await?;
        assert_eq!(count, 2);
        harness.cleanup().await
    })
}
