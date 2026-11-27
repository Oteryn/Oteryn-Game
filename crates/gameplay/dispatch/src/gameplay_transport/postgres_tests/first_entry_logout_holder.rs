use std::{sync::Arc, time::Duration};

use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
    logout::{
        CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
        encode_logout_intent,
    },
};
use tokio::{io::DuplexStream, sync::Notify};

use super::autowalk_fixtures::{read, send};
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::{
    DurabilityError, DurabilityRoot, activity_handoff::ActivityHandoffOwner,
    fresh_admission::FreshAdmissionStore,
};
use crate::foundation::GameSessionState;
use crate::gameplay_transport::activity_recovery::ACTIVITY_INPUT_HOLDER_BUSY;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::logout::{
    LOGOUT_RELEASE_PROBE, LogoutReleaseProbe, ReleaseRetryStage,
};
use crate::gameplay_transport::{AdmissionRetryBudget, COMMAND_HOLDER_WINDOW, item_fence_of};

#[derive(Clone, Copy)]
enum ContentionPoint {
    ActivityInput,
    ReleaseConditions,
}

#[test]
fn first_entry_logout_waits_for_activity_input_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, ContentionPoint::ActivityInput, false)
}

#[test]
fn first_entry_logout_returns_typed_busy_and_replays_it_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, ContentionPoint::ActivityInput, true)
}

#[test]
fn first_entry_logout_retries_a_fenced_release_and_commits_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, ContentionPoint::ReleaseConditions, false)
}

#[test]
fn first_entry_logout_release_budget_exhaustion_returns_typed_busy_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, ContentionPoint::ReleaseConditions, true)
}

#[test]
fn activity_scope_read_yields_holder_to_other_commands_on_admission_lock_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("activity-scope-lock", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(activity_scope_lock(admin))
    })
}

async fn activity_scope_lock(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "activity_scope_lock", true, false).await?;
    prepare_current_session(&h).await?;
    let session = Dependencies::new()?.admitted.game_session_id;
    let handoffs = ActivityHandoffOwner::new(h.root.clone());
    let mut writer = h.pool.begin().await?;
    sqlx::query("LOCK TABLE game_durability_admission_account_guards IN ROW EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await?;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), handoffs.transfer_recovery(session)).await?,
        Err(DurabilityError::RootUnavailable)
    ));
    // Another command can take the holder while the writer still holds the admission lock.
    h.root
        .try_issue_semantic_pass()?
        .run(|_, _| Box::pin(async { Ok(()) }))
        .await?;
    writer.rollback().await?;
    assert!(handoffs.transfer_recovery(session).await?.is_none());
    h.cleanup().await
}

fn run(admin: String, point: ContentionPoint, exhaust: bool) -> TestResult {
    scenario_thread::run("first-entry-logout-holder", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin, point, exhaust))
    })
}

async fn scenario(admin: String, point: ContentionPoint, exhaust: bool) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "first_entry_logout_holder", true, false)
        .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(&h, &authority);
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    track(&owner, &deps)?;

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    admitted.item_fence = Some(
        item_fence_of(store.current_session_at(admitted.game_session_id).await?.0)
            .ok_or("first-entry item fence")?,
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_LOGOUT_V1, CAPABILITY_BOSS_ENCOUNTER_V1],
    )
    .ok_or("logout and activity capabilities")?;
    assert_eq!(admitted.continuity.next_command_id, 1);

    let busy = Arc::new(Notify::new());
    let release = Arc::new(LogoutReleaseProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = DurabilityRoot::with_holder_owner(async {
        let connection = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
        match point {
            ContentionPoint::ActivityInput => {
                ACTIVITY_INPUT_HOLDER_BUSY.scope(busy.clone(), connection).await
            }
            ContentionPoint::ReleaseConditions => {
                LOGOUT_RELEASE_PROBE.scope(release.clone(), connection).await
            }
        }
    });
    let driving = async {
        loop {
            let frame = read(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }

        let started = tokio::time::Instant::now();
        let mut pass = Some(match point {
            ContentionPoint::ActivityInput => {
                let pass = if exhaust {
                    let _fixture_deadline = h.root.boot_pass_deadline(
                        AdmissionRetryBudget::holder_wait_window() + crate::durability::DB_PASS_DEADLINE,
                    );
                    h.root.issue_ready_semantic_pass().await?
                } else {
                    h.root.issue_ready_semantic_pass().await?
                };
                send_logout(&mut client, 1).await?;
                busy.notified().await;
                pass
            }
            ContentionPoint::ReleaseConditions => {
                send_logout(&mut client, 1).await?;
                release.before_conditions.notified().await;
                if exhaust {
                    // Spend part of the original release window before the first failure.
                    tokio::time::sleep(COMMAND_HOLDER_WINDOW).await;
                }
                let pass = if exhaust {
                    // Only the competing fixture pass needs to outlive the release window.
                    let _fixture_deadline = h.root.boot_pass_deadline(
                        AdmissionRetryBudget::holder_wait_window()
                            + crate::durability::DB_PASS_DEADLINE,
                    );
                    h.root.issue_ready_semantic_pass().await?
                } else {
                    h.root.issue_ready_semantic_pass().await?
                };
                release.resume_conditions.notify_one();
                release.retry_observed.notified().await;
                assert_eq!(release.first_retry.get(), Some(&ReleaseRetryStage::ConditionsHolder));
                assert_eq!(owner.states.fence_holders.counts().len(), 1);
                pass
            }
        });
        if exhaust {
            if matches!(point, ContentionPoint::ReleaseConditions) {
                // A window started at the first failure would still be running here.
                tokio::time::timeout(
                    AdmissionRetryBudget::holder_wait_window() - Duration::from_millis(250),
                    release.retry_exhausted.notified(),
                )
                .await?;

            }
            let (status, result) = tokio::time::timeout(
                AdmissionRetryBudget::holder_wait_window() + Duration::from_secs(1),
                logout_result(&mut client, 1),
            )
            .await??;
            assert!(started.elapsed() >= AdmissionRetryBudget::holder_wait_window());
            assert_eq!((status, result), (CommandStatus::Rejected, LogoutResult::Busy));
            if let Some(pass) = pass.take() {
                pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
            }
            h.root.await_ready().await?;
            assert_eq!(
                store.current_session(admitted.game_session_id).await?.session_state(),
                GameSessionState::Active,
            );
            assert!(owner.states.fence_holders.counts().is_empty());
            send_logout(&mut client, 1).await?;
            assert_eq!(
                logout_result(&mut client, 1).await?,
                (CommandStatus::DuplicateReplay, LogoutResult::Busy),
            );
            send_logout(&mut client, 2).await?;
        } else {
            pass.ok_or("checked-out holder")?.run(|_, _| {
                Box::pin(async {
                    tokio::time::sleep(COMMAND_HOLDER_WINDOW + Duration::from_millis(250)).await;
                    Ok(())
                })
            })
            .await?;
        }
        let id = if exhaust { 2 } else { 1 };
        assert_eq!(
            logout_result(&mut client, id).await?,
            (CommandStatus::Accepted, LogoutResult::Accepted),
        );
        TestResult::Ok(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::try_join!(async { TestResult::Ok(serving.await) }, driving)
    })
    .await??;
    assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)), "{end:?}");
    assert_eq!(
        store.current_session(admitted.game_session_id).await?.session_state(),
        GameSessionState::Terminal,
    );
    let positions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(positions, 1, "the accepted logout commits its final position");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn send_logout(client: &mut DuplexStream, id: u64) -> TestResult {
    send(client, id, COMMAND_TYPE_LOGOUT_INTENT, &encode_logout_intent()).await
}

async fn logout_result(
    client: &mut DuplexStream,
    id: u64,
) -> TestResult<(CommandStatus, LogoutResult)> {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            return Ok((result.status, decode_logout_result(result.payload).map_err(debug)?));
        }
    }
}
