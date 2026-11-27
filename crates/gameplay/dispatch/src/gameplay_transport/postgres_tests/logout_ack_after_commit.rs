//! The ordinary wire acknowledgement must wait for the durable terminal release.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionState;
use crate::gameplay_transport::capabilities::{SelectedCapabilities, test_command};
use crate::gameplay_transport::composed_admission::{
    LOGOUT_RELEASE_COMMIT_BARRIER, LogoutReleaseCommitBarrier,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::logout::{
    CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    encode_logout_intent,
};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::DuplexStream;

#[test]
fn logout_ack_waits_for_durable_release_commit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "logout_ack_commit", true, false).await?;
        let result = case(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

async fn case(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
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
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_LOGOUT_V1])
            .ok_or("logout capability")?;

    let barrier = Arc::new(LogoutReleaseCommitBarrier::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = LOGOUT_RELEASE_COMMIT_BARRIER.scope(
        Arc::clone(&barrier),
        serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS),
    );
    let driving = async {
        let command_id = admitted.continuity.next_command_id;
        let command = test_command(
            command_id,
            COMMAND_TYPE_LOGOUT_INTENT,
            &encode_logout_intent(),
        );
        write_frame(&mut client, &command).await?;
        let mut reply = std::pin::pin!(tokio::task::unconstrained(read_logout_result(&mut client)));
        tokio::select! {
            biased;
            early = &mut reply => return Err(format!("logout replied before reaching release_commit: {early:?}").into()),
            () = barrier.entered.notified() => {}
        }
        // The serving future remains at the barrier until this task gives it permission.
        // Poll the same frame reader so a buffered reply cannot pass the ordering check.
        tokio::select! {
            biased;
            early = &mut reply => return Err(format!("logout replied while release_commit was held: {early:?}").into()),
            () = std::future::ready(()) => {}
        }
        barrier.proceed.notify_one();
        assert_eq!(
            reply.await?,
            (command_id, CommandStatus::Accepted, LogoutResult::Accepted)
        );
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::try_join!(async { Ok(serving.await) }, driving)
    })
    .await
    .map_err(|_| "logout commit barrier or wire reply timed out")??;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "{end:?}"
    );
    assert_eq!(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        GameSessionState::Terminal,
    );
    eprintln!("POSTGRES_CASE_EXECUTED logout_ack_waits_for_durable_release_commit_postgres");
    Ok(())
}

async fn read_logout_result(
    client: &mut DuplexStream,
) -> TestResult<(u64, CommandStatus, LogoutResult)> {
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Ok((
                result.command_id,
                result.status,
                decode_logout_result(result.payload).map_err(debug)?,
            ));
        }
    }
}
