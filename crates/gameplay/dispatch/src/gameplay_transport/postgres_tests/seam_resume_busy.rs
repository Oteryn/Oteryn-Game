//! A late busy read must retain progress within the same admission budget.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId, MessageType};
use crate::gameplay_transport::connection::{
    ConnectionIdentifiers, ControlLossResult, FirstEntryOutcome, FreshAdmissionAuthority,
    admit_frame,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;

struct Identifiers;
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        None
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[93; 16]).ok()
    }
}

#[test]
fn seam_resume_busy_loss_read_preserves_progress_postgres() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "seam_resume_busy", false, false).await?;
        let result: TestResult = async {
            let source = super::house_transfer::RecoverySource::start(&h).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &source.evidence;
            deps.initialize(&owner).await;
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                super::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(admitted.game_session_id).await?);
            owner.states.command_ingress.admit(admitted.game_session_id);
            assert_eq!(owner.lose_control(admitted, Duration::ZERO, None).await,
                ControlLossResult::Recorded);
            let token = super::house_transfer::RecoverySource::token(93)?;
            let frame = oteryn_protocol_oteryn::recovery::encode_client_resume(
                &oteryn_protocol_oteryn::recovery::ClientResumeValue {
                    game_session_id: admitted.game_session_id,
                    reconnect_material: token.as_bytes(),
                    last_applied_server_sequence: 0,
                    schema_revision: 1,
                    client_build_id: "nbresume-busy-root",
                    supported_capabilities: &[],
                }).map_err(debug)?;
            let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
            let finished = tokio::sync::Notify::new();
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let resuming = async {
                let result = crate::gameplay_transport::resume::LOSS_READ_TEST_BARRIER
                    .scope(barrier.clone(), admit_frame(&mut server, &frame, &owner, &Identifiers)).await;
                finished.notify_one();
                result
            };
            let holding = async {
                let mut holds = 0;
                loop {
                    tokio::select! {
                        () = finished.notified() => break,
                        () = barrier.0.notified() => {}
                    }
                    let held = h.root.try_issue_semantic_pass()?;
                    holds += 1;
                    barrier.1.notify_one();
                    // Poll admission while the holder is occupied. Repeated background
                    // contention at this read must not restart evidence acquisition.
                    let done = tokio::time::timeout(Duration::from_millis(150), finished.notified()).await.is_ok();
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    if done { break; }
                }
                Ok::<_, Box<dyn std::error::Error>>(holds)
            };
            let started = std::time::Instant::now();
            let (resumed, holds) = tokio::time::timeout(Duration::from_secs(20), async {
                tokio::join!(resuming, holding)
            }).await?;
            let holds = holds?;
            let resumed = resumed.map_err(debug)?;
            assert_eq!(holds, 1, "a busy read must not repeat the evidence refresh");
            assert_eq!(resumed.game_session_id, admitted.game_session_id);
            assert_eq!(resumed.runtime_actor, admitted.runtime_actor);
            assert_eq!(resumed.continuity.connection_generation, 2);
            assert!(resumed.item_fence.is_some());
            let admissions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&h.pool).await?;
            assert_eq!(admissions, 1, "resume must not create a new admission");
            drop(server);
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            let length = u32::from_be_bytes(output.get(..4).ok_or("ServerResumeAccepted frame")?.try_into()?) as usize;
            let reply = crate::foundation::decode_wire_envelope(output.get(4..4 + length).ok_or("complete frame")?).map_err(debug)?;
            assert_eq!(reply.message_type(), MessageType::ServerResumeAccepted);
            assert_eq!(output.len(), length + 4);
            eprintln!("PG_EXECUTED seam_resume_busy holds={holds} generation=2 server_resume_accepted=1 duration_ms={}", started.elapsed().as_millis());
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
