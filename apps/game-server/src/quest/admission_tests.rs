//! QUEST-RT-1: actual ComposedFreshAdmission on PostgreSQL 17.6.
//! Nested under the existing native monk-save fixture; no alternate quest admission owner.
#![allow(clippy::expect_used)]

use super::fixture::{Harness, debug, id, join_two, run as run_fixture};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::character_progression_content::{
    CharacterProgressionContent, tests::section_bytes,
};
use crate::durability::character_progression::{
    ProgressionInitializationOutcome, ProgressionInitializationRequest,
};
use crate::durability::quest_state::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, SessionProgressionBinding};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

const QUEST: &str = "oteryn:quest/fixture.admission";
const STAGE: &str = "oteryn:quest-progress/fixture.admission.stage";

/// The debug native settle chain exceeds the default test stack, as in the existing
/// kill-reward PostgreSQL scenario. This stack belongs only to this test thread; the
/// production runtime stack and process-wide RUST_MIN_STACK are unchanged.
const SCENARIO_STACK_BYTES: usize = 64 << 20;

fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult + Send + 'static,
{
    let scenario = std::thread::Builder::new()
        .name("quest-admission-postgres".into())
        .stack_size(SCENARIO_STACK_BYTES)
        .spawn(move || {
            run_fixture(async move |admin| Box::pin(body(admin)).await)
                .map_err(|error| error.to_string())
        })?;
    match scenario.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Pauses one native refresh after its independent current-fence read. A test changes just
/// the current generation or session presence before the normal production path resumes.
#[derive(Default)]
struct RefreshGate {
    entered: Notify,
    proceed: Notify,
}

thread_local! {
    static AFTER_FENCE: RefCell<Option<Arc<RefreshGate>>> = const { RefCell::new(None) };
}

pub(in crate::gameplay_transport) async fn after_fence() {
    let gate = AFTER_FENCE.with(|slot| slot.borrow_mut().take());
    if let Some(gate) = gate {
        gate.entered.notify_one();
        gate.proceed.notified().await;
    }
}

fn pause_after_fence() -> Arc<RefreshGate> {
    let gate = Arc::new(RefreshGate::default());
    AFTER_FENCE.with(|slot| {
        assert!(slot.borrow_mut().replace(Arc::clone(&gate)).is_none());
    });
    gate
}

fn session() -> GameSessionId {
    GameSessionId::decode(&id(50)).expect("fixture session")
}

fn catalogue() -> Arc<QuestStateCatalogue> {
    let transition = |name: &str, from, stage, experience| QuestTransition {
        key: format!("oteryn:quest-transition/fixture.admission.{name}"),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: STAGE.into(),
            from,
            effect: QuestEffectKind::Set(stage),
        }],
        completes: false,
        experience: Some(experience),
    };
    Arc::new(
        QuestStateCatalogue::new(
            // This provenance pin deliberately differs from the progression reward revision.
            "content-1",
            vec![QuestTrack {
                key: STAGE.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 2,
            }],
            vec![
                transition("begin", QuestComparison::Eq(0), 1, 50),
                transition("again", QuestComparison::Eq(1), 2, 10),
            ],
        )
        .expect("quest fixture"),
    )
}

/// The prerequisites' real initializer validates the exact content policy against the current
/// admitted fence. Attaching that immutable binding is a fixture input, not a new authority API.
async fn initialize_progression(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Arc<SessionProgressionBinding>> {
    let content = CharacterProgressionContent::decode(&section_bytes()).map_err(debug)?;
    let binding = Arc::new(SessionProgressionBinding::from_content(
        content
            .policy_for("profile-1", "ruleset-1", "content-1")
            .map_err(debug)?,
    ));
    let fence = owner
        .current_quest_fence(session())
        .await
        .map_err(|()| "current fixture fence unavailable")?
        .ok_or("fixture session ended")?;
    let outcome = owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        ProgressionInitializationOutcome::Initialized(_)
    ));
    assert_ne!(binding.reward_revision, "content-1");
    Ok(binding)
}

fn bind(owner: &ComposedFreshAdmission<'_, '_, '_>, binding: Arc<SessionProgressionBinding>) {
    owner
        .progression_sessions
        .lock()
        .expect("session bindings")
        .insert(session(), binding);
}

async fn commit_pending(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    name: &str,
    command_id: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session())
        .await
        .map_err(|()| "current fixture fence unavailable")?
        .ok_or("fixture session ended")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!("oteryn:quest-transition/fixture.admission.{name}"),
                cause: QuestCause::Command(CommandRef::new(
                    session(),
                    CommandId::new(command_id).map_err(debug)?,
                )),
            },
            Arc::clone(owner.quest_catalogue.as_ref().expect("quest catalogue")),
        )
        .await
        .map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = outcome else {
        return Err(format!("expected pending XP transition, got {outcome:?}").into());
    };
    assert!(receipt.experience.is_some());
    Ok(())
}

/// Independent durable outputs: total XP, XP receipt count and open quest XP obligations.
async fn durable(harness: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT total_experience, \
                (SELECT count(*) FROM game_character_xp_receipts), \
                (SELECT count(*) FROM game_character_quest_xp_obligations) \
           FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

#[test]
fn native_quest_admission_and_refresh_pay_each_pending_xp_once() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_native_once", false, false).await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let mut previous = dependencies.owner(&harness, &authority);
            previous.quest_catalogue = Some(catalogue());
            let binding = Box::pin(initialize_progression(&previous)).await?;
            commit_pending(&previous, "begin", 1).await?;
            assert_eq!(durable(&harness).await?, (0, 0, 1));
            // A new native owner has no session quest copy or sequencer cursor to reuse.
            drop(previous);
            let mut owner = dependencies.owner(&harness, &authority);
            owner.quest_catalogue = Some(catalogue());
            bind(&owner, binding);
            owner.admit_quest_session(&dependencies.admitted).await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
            owner.admit_quest_session(&dependencies.admitted).await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
            commit_pending(&owner, "again", 2).await?;
            owner.refresh_quest_session(session(), false).await;
            assert_eq!(durable(&harness).await?, (60, 2, 0));
            owner.refresh_quest_session(session(), false).await;
            assert_eq!(durable(&harness).await?, (60, 2, 0));
            let sessions = owner.quest_sessions.lock().expect("quest sessions");
            let entry = sessions.get(&session()).expect("admitted copy");
            assert!(
                entry
                    .copy
                    .as_ref()
                    .expect("usable copy")
                    .xp_obligations()
                    .is_empty()
            );
            assert!(entry.retry_at.is_none());
        }
        harness.cleanup().await
    })
}

#[test]
fn native_unbound_quest_admission_defers_without_retry_and_later_binding_pays_once() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_native_unbound", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.quest_catalogue = Some(catalogue());
            let binding = Box::pin(initialize_progression(&owner)).await?;
            commit_pending(&owner, "begin", 1).await?;
            for _ in 0..2 {
                owner.admit_quest_session(&dependencies.admitted).await;
                assert_eq!(durable(&harness).await?, (0, 0, 1));
                let sessions = owner.quest_sessions.lock().expect("quest sessions");
                let entry = sessions.get(&session()).expect("admitted copy");
                assert_eq!(
                    entry
                        .copy
                        .as_ref()
                        .expect("usable copy")
                        .xp_obligations()
                        .len(),
                    1
                );
                assert!(
                    entry.retry_at.is_none(),
                    "unbound alone cannot request retry"
                );
                assert!(
                    entry.experience_deferred_logged,
                    "deferral marker survives resume"
                );
            }
            bind(&owner, binding);
            owner.admit_quest_session(&dependencies.admitted).await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
            owner.admit_quest_session(&dependencies.admitted).await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
        }
        harness.cleanup().await
    })
}

#[test]
fn native_quest_xp_unknown_attempt_retries_only_after_backoff_and_pays_once() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_native_retry", false, false).await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.quest_catalogue = Some(catalogue());
            bind(&owner, Box::pin(initialize_progression(&owner)).await?);
            commit_pending(&owner, "begin", 1).await?;
            // Only the XP transaction fails. Current-fence reads and quest-copy loading stay real.
            sqlx::query("CREATE FUNCTION quest_admission_fail_xp() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'quest XP injected unavailable outcome' USING ERRCODE='40001'; END $$")
                .execute(&harness.pool).await?;
            sqlx::query("CREATE TRIGGER quest_admission_fail_xp BEFORE INSERT ON game_character_xp_receipts FOR EACH ROW EXECUTE FUNCTION quest_admission_fail_xp()")
                .execute(&harness.pool).await?;
            let before = Instant::now();
            owner.admit_quest_session(&dependencies.admitted).await;
            assert_eq!(durable(&harness).await?, (0, 0, 1));
            let retry_at = owner
                .quest_sessions
                .lock()
                .expect("quest sessions")
                .get(&session())
                .expect("admitted copy")
                .retry_at
                .expect("unknown award retry");
            assert!(retry_at >= before + crate::gameplay_transport::QUEST_OBLIGATION_RETRY);
            sqlx::query("DROP TRIGGER quest_admission_fail_xp ON game_character_xp_receipts")
                .execute(&harness.pool)
                .await?;
            sqlx::query("DROP FUNCTION quest_admission_fail_xp()")
                .execute(&harness.pool)
                .await?;
            owner.refresh_due_quest_sessions().await;
            assert_eq!(durable(&harness).await?, (0, 0, 1), "backoff still holds");
            // Advance only this session's due time; do not replace the production cadence.
            owner
                .quest_sessions
                .lock()
                .expect("quest sessions")
                .get_mut(&session())
                .expect("admitted copy")
                .retry_at = Some(Instant::now());
            owner.refresh_due_quest_sessions().await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
            owner.refresh_due_quest_sessions().await;
            assert_eq!(durable(&harness).await?, (50, 1, 0));
        }
        harness.cleanup().await
    })
}

#[test]
fn native_quest_xp_refuses_generation_replaced_after_current_fence_read() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_native_fence", false, false).await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.quest_catalogue = Some(catalogue());
            bind(&owner, Box::pin(initialize_progression(&owner)).await?);
            commit_pending(&owner, "begin", 1).await?;
            let gate = pause_after_fence();
            let replacement = async {
                let entered =
                    tokio::time::timeout(Duration::from_secs(10), gate.entered.notified()).await;
                if let Err(error) = entered {
                    gate.proceed.notify_one();
                    return Err(error.into());
                }
                // One invariant changes: current connection generation. Identity, lease, scope,
                // policy, quest pin and pending occurrence all retain their valid baseline.
                let changed = sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(id(50).as_slice()).execute(&harness.pool).await;
                gate.proceed.notify_one();
                assert_eq!(changed?.rows_affected(), 1);
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let (_, replacement) = tokio::time::timeout(
                Duration::from_secs(15),
                join_two(owner.refresh_quest_session(session(), true), replacement),
            )
            .await?;
            replacement?;
            assert_eq!(durable(&harness).await?, (0, 0, 1));
            let sessions = owner.quest_sessions.lock().expect("quest sessions");
            let entry = sessions.get(&session()).expect("admitted copy");
            assert_eq!(
                entry
                    .copy
                    .as_ref()
                    .expect("usable copy")
                    .xp_obligations()
                    .len(),
                1
            );
            assert!(
                entry.retry_at.is_none(),
                "definitive authority refusal is not unknown"
            );
        }
        harness.cleanup().await
    })
}

#[test]
fn native_quest_refresh_does_not_recreate_a_removed_session() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_native_removed", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.quest_catalogue = Some(catalogue());
            bind(&owner, Box::pin(initialize_progression(&owner)).await?);
            owner.admit_quest_session(&dependencies.admitted).await;
            commit_pending(&owner, "begin", 1).await?;
            let gate = pause_after_fence();
            let removal = async {
                let entered =
                    tokio::time::timeout(Duration::from_secs(10), gate.entered.notified()).await;
                if let Err(error) = entered {
                    gate.proceed.notify_one();
                    return Err(error.into());
                }
                owner.forget_quest_session(session());
                owner.forget_progression_session(session());
                gate.proceed.notify_one();
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let (_, removal) = tokio::time::timeout(
                Duration::from_secs(15),
                join_two(owner.refresh_quest_session(session(), false), removal),
            )
            .await?;
            removal?;
            assert!(
                !owner
                    .quest_sessions
                    .lock()
                    .expect("quest sessions")
                    .contains_key(&session())
            );
            assert!(owner.progression_binding(session()).is_none());
            assert_eq!(durable(&harness).await?, (0, 0, 1));
        }
        harness.cleanup().await
    })
}
