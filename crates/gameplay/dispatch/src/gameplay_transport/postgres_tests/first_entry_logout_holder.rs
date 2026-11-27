use std::{sync::Arc, time::Duration};

use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
    logout::{
        CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
        encode_logout_intent,
    },
};
use tokio::io::DuplexStream;

use super::autowalk_fixtures::{read, send};
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::{
    DurabilityRoot, fresh_admission::FreshAdmissionStore,
};
use crate::foundation::GameSessionState;
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

#[test]
fn first_entry_logout_retries_a_fenced_release_and_commits_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, false)
}

#[test]
fn first_entry_logout_release_budget_exhaustion_returns_typed_busy_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, true)
}

fn run(admin: String, exhaust: bool) -> TestResult {
    scenario_thread::run("first-entry-logout-holder", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin, exhaust))
    })
}

async fn scenario(admin: String, exhaust: bool) -> TestResult {
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

    let release = Arc::new(LogoutReleaseProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = DurabilityRoot::with_holder_owner(LOGOUT_RELEASE_PROBE.scope(
        release.clone(),
        serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS),
    ));
    let driving = async {
        loop {
            let frame = read(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }

        let started = tokio::time::Instant::now();
        send_logout(&mut client, 1).await?;
        release.before_conditions.notified().await;
        if exhaust {
            // Spend part of the original release window before the first failure.
            tokio::time::sleep(COMMAND_HOLDER_WINDOW).await;
        }
        let pass = if exhaust {
            let _fixture_deadline = h.root.boot_pass_deadline(
                AdmissionRetryBudget::holder_wait_window() + crate::durability::DB_PASS_DEADLINE,
            );
            h.root.issue_ready_semantic_pass().await?
        } else {
            h.root.issue_ready_semantic_pass().await?
        };
        release.resume_conditions.notify_one();
        release.retry_observed.notified().await;
        assert_eq!(release.first_retry.get(), Some(&ReleaseRetryStage::Conditions));
        assert_eq!(owner.states.fence_holders.counts().len(), 1);
        assert!(owner.runtime.lock().await
            .bound_attacker_lease(deps.actor, admitted.game_session_id).is_err());
        if exhaust {
            // A window started at the first failure would still be running here.
            tokio::time::timeout(
                AdmissionRetryBudget::holder_wait_window() - Duration::from_millis(250),
                release.retry_exhausted.notified(),
            )
            .await?;
            assert_eq!(owner.states.fence_holders.counts().len(), 1);
            // The budget has expired. Let settlement prove the session is still active.
            pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
            let (status, result) = tokio::time::timeout(
                AdmissionRetryBudget::holder_wait_window() + Duration::from_secs(1),
                logout_result(&mut client, 1),
            )
            .await??;
            assert!(started.elapsed() >= AdmissionRetryBudget::holder_wait_window());
            assert_eq!((status, result), (CommandStatus::Rejected, LogoutResult::Busy));
            h.root.await_ready().await?;
            assert_eq!(
                store.current_session(admitted.game_session_id).await?.session_state(),
                GameSessionState::Active,
            );
            assert!(owner.states.fence_holders.counts().is_empty());
            assert!(owner.runtime.lock().await
                .bound_attacker_lease(deps.actor, admitted.game_session_id).is_ok());
            send_logout(&mut client, 1).await?;
            assert_eq!(
                logout_result(&mut client, 1).await?,
                (CommandStatus::DuplicateReplay, LogoutResult::Busy),
            );
            send_logout(&mut client, 2).await?;
        } else {
            pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
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
    assert_eq!(
        release.conditions_entries.load(std::sync::atomic::Ordering::Relaxed),
        if exhaust { 2 } else { 1 },
        "a condition retry must stay at its original checkpoint"
    );
    assert_ne!(release.retry_count.load(std::sync::atomic::Ordering::Relaxed), 0);
    eprintln!("first-entry logout holder regression reached durable assertions exhaust={exhaust}");
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
