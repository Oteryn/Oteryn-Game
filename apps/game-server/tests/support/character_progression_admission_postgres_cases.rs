// PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.5, §2.2) admission progression step
// cases. A child of the shared P03 cases, so it reuses their harness (fenced session at root
// revision 1, interpretation profile-1/ruleset-1/content-1, scope assignment, node
// incarnation) and runs in both PostgreSQL wrappers.

use super::{Harness, TestResult, configured_admin, context, fence, id};
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

#[allow(dead_code)]
#[path = "../../src/gameplay_transport/character_progression_binding.rs"]
mod character_progression_binding;

use character_progression_binding::seam::{InitializerFault, scoped};
use character_progression_binding::{
    ProgressionBinding, ProgressionDurable, ProgressionRounds, ProgressionUnbound,
    bind_character_progression,
};

const ROUNDS: ProgressionRounds = ProgressionRounds {
    attempts: 3,
    backoff: std::time::Duration::from_millis(5),
};

/// A level-one policy (the D88 start) with the Character root's three revisions.
fn policy(profile: &str, ruleset: &str, content: &str) -> FiniteProgressionPolicy<String, 2> {
    let mut context = context();
    context.profile = profile.into();
    context.ruleset = ruleset.into();
    context.content = content.into();
    FiniteProgressionPolicy {
        context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        death_policy_revision: "death-1".into(),
        declared_difference_revision: "declaration-1".into(),
        thresholds: [
            LevelThreshold {
                level: 1,
                minimum_experience: ExactI64::new(0),
            },
            LevelThreshold {
                level: 2,
                minimum_experience: ExactI64::new(100),
            },
        ],
        terminal_exclusive_experience: ExactI64::new(200),
        death_loss_numerator: 1,
        death_loss_denominator: 1,
        death_loss_rounding: RoundingMode::Floor,
    }
}

fn pinned(
    profile: &str,
    ruleset: &str,
    content: &str,
) -> Option<FiniteProgressionPolicy<String, 2>> {
    Some(policy(profile, ruleset, content))
}

async fn at(revision: u64) -> Result<Option<CurrentCharacterGameplayFence>, ()> {
    fence(revision).map(Some).map_err(|_| ())
}

async fn bind<P>(
    harness: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    policy_for: P,
    revision: u64,
    faults: Vec<Option<InitializerFault>>,
) -> (ProgressionBinding<2>, u32)
where
    P: Fn(&str, &str, &str) -> Option<FiniteProgressionPolicy<String, 2>>,
{
    scoped(
        faults,
        bind_character_progression(
            ProgressionDurable {
                root: &harness.root,
                character: authority,
                holder: &harness.node,
            },
            policy_for,
            || at(revision),
            ROUNDS,
        ),
    )
    .await
}

/// The row's character revision, level, experience and eight stored revisions.
async fn row(pool: &sqlx::PgPool) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', character_revision, level, total_experience, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision) \
           FROM game_character_progression_state",
    )
    .fetch_optional(pool)
    .await?)
}

const INITIALIZED: &str = "1|1|0|profile-1|ruleset-1|content-1|simulation-1|evidence-1|\
                           declaration-1|policy-1|reward-1";

fn death(policy: &FiniteProgressionPolicy<String, 2>) -> TestResult<CharacterDeathRequest<2>> {
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(95))
            .map_err(|error| format!("{error:?}"))?,
        context: policy.context.clone(),
        policy_revision: policy.policy_revision.clone(),
        reward_revision: policy.reward_revision.clone(),
        policy: policy.clone(),
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            world_id: crate::foundation::WorldId::decode(&id(42))
                .map_err(|error| format!("{error:?}"))?,
            channel_id: crate::foundation::ChannelId::decode(&id(43))
                .map_err(|error| format!("{error:?}"))?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
    })
}

fn run<F>(body: F) -> TestResult
where
    F: FnOnce(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult>>>,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(body(admin))
}

/// A new Character is initialized with the binding's exact request from the root's
/// interpretation; its first death then commits under that binding, and a later admission finds
/// the matching row without the initializer.
#[test]
fn a_new_character_is_bound_initialized_and_its_death_is_durable() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = Harness::create(admin, "prog_admit_new", false).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let read = std::cell::RefCell::new(Vec::new());
            let (binding, calls) = bind(
                &harness,
                &authority,
                |profile: &str, ruleset: &str, content: &str| {
                    read.borrow_mut()
                        .push([profile, ruleset, content].map(str::to_owned));
                    pinned(profile, ruleset, content)
                },
                1,
                Vec::new(),
            )
            .await;
            assert_eq!(
                read.into_inner(),
                vec![["profile-1", "ruleset-1", "content-1"].map(str::to_owned)]
            );
            let bound = policy("profile-1", "ruleset-1", "content-1");
            assert_eq!(binding, ProgressionBinding::Bound(Box::new(bound.clone())));
            assert_eq!(calls, 1);
            assert_eq!(row(&harness.pool).await?.as_deref(), Some(INITIALIZED));

            let sequencer = CharacterRevisionSequencer::new();
            let settled = harness
                .root
                .settle_player_death(
                    &sequencer,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    death(&bound)?,
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(settled.committed_character_revision.get(), 2);
            // A replay under the same binding matches the stored receipt's policy digest.
            let replay = harness
                .root
                .settle_player_death(
                    &sequencer,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    death(&bound)?,
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(replay, settled);
            let receipt: String = sqlx::query_scalar(
                "SELECT concat_ws('|', count(*), min(profile_revision), min(ruleset_revision), \
                   min(content_revision), min(policy_revision), min(reward_revision)) \
                   FROM game_character_death_receipts",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(receipt, "1|profile-1|ruleset-1|content-1|policy-1|reward-1");

            // A later admission at the new root revision finds the matching row.
            let (again, calls) = bind(&harness, &authority, pinned, 2, Vec::new()).await;
            assert_eq!(again, ProgressionBinding::Bound(Box::new(bound)));
            assert_eq!(calls, 0);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

/// Rows that cannot bind, and a Character past revision 1 without one, never reach the
/// initializer and are never rewritten.
#[test]
fn unbindable_rows_and_missing_rows_past_revision_one_decide_without_a_write() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = Harness::create(admin, "prog_admit_unbound", false).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            // No backfill: a missing row past revision 1 stays uninitialized.
            let (binding, calls) = bind(&harness, &authority, pinned, 2, Vec::new()).await;
            assert_eq!(
                binding,
                ProgressionBinding::Unbound(ProgressionUnbound::Uninitialized)
            );
            assert_eq!(calls, 0);
            // Content forming no policy for the root's revisions.
            let (binding, calls) = bind(
                &harness,
                &authority,
                |_: &str, _: &str, _: &str| None,
                1,
                Vec::new(),
            )
            .await;
            assert_eq!(
                binding,
                ProgressionBinding::Unbound(ProgressionUnbound::ContextMismatch)
            );
            assert_eq!(calls, 0);
            assert_eq!(row(&harness.pool).await?, None);

            let (binding, _) = bind(&harness, &authority, pinned, 1, Vec::new()).await;
            assert!(matches!(binding, ProgressionBinding::Bound(_)));
            // The row is at revision 1; a fence past it is stale.
            let (binding, calls) = bind(&harness, &authority, pinned, 2, Vec::new()).await;
            assert_eq!(
                binding,
                ProgressionBinding::Unbound(ProgressionUnbound::StaleRevision)
            );
            assert_eq!(calls, 0);
            // A changed table, and a changed death policy alone, change the policy revision.
            let table = |profile: &str, ruleset: &str, content: &str| {
                let mut policy = policy(profile, ruleset, content);
                policy.thresholds[1].minimum_experience = ExactI64::new(150);
                policy.policy_revision = "policy-2".into();
                Some(policy)
            };
            let death_only = |profile: &str, ruleset: &str, content: &str| {
                let mut policy = policy(profile, ruleset, content);
                policy.death_policy_revision = "death-2".into();
                policy.policy_revision = "policy-3".into();
                Some(policy)
            };
            for (binding, calls) in [
                bind(&harness, &authority, table, 1, Vec::new()).await,
                bind(&harness, &authority, death_only, 1, Vec::new()).await,
            ] {
                assert_eq!(
                    binding,
                    ProgressionBinding::Unbound(ProgressionUnbound::ContextMismatch)
                );
                assert_eq!(calls, 0);
            }
            assert_eq!(row(&harness.pool).await?.as_deref(), Some(INITIALIZED));
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

/// Every round sends the identical request: a failed round is retried, a lost acknowledgement
/// is reconciled by the committed row, and rounds that all fail leave no row and a later
/// admission initializes it.
#[test]
fn faulted_initializer_rounds_retry_reconcile_and_fail_closed() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = Harness::create(admin, "prog_admit_fault", false).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let fail = Some(InitializerFault::FailBeforeCommit);
            let (binding, calls) =
                bind(&harness, &authority, pinned, 1, vec![fail, fail, fail]).await;
            assert_eq!(
                binding,
                ProgressionBinding::Unbound(ProgressionUnbound::InitializationUnavailable)
            );
            assert_eq!(calls, ROUNDS.attempts);
            assert_eq!(row(&harness.pool).await?, None);

            let (binding, calls) = bind(&harness, &authority, pinned, 1, vec![fail]).await;
            assert!(matches!(binding, ProgressionBinding::Bound(_)));
            assert_eq!(calls, 2);
            assert_eq!(row(&harness.pool).await?.as_deref(), Some(INITIALIZED));
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

#[test]
fn a_lost_initializer_acknowledgement_is_reconciled_by_the_next_round() -> TestResult {
    run(|admin| {
        Box::pin(async move {
            let harness = Harness::create(admin, "prog_admit_lost_ack", false).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let (binding, calls) = bind(
                &harness,
                &authority,
                pinned,
                1,
                vec![Some(InitializerFault::LoseAcknowledgement)],
            )
            .await;
            assert_eq!(
                binding,
                ProgressionBinding::Bound(Box::new(policy("profile-1", "ruleset-1", "content-1")))
            );
            assert_eq!(calls, 1);
            assert_eq!(row(&harness.pool).await?.as_deref(), Some(INITIALIZED));
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}
