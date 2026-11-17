//! A durable fresh winner remains discoverable while another pass owns the root.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::ReconciliationDisposition;
use std::time::Duration;

#[test]
fn seam_fresh_commit_busy_spell_read_still_sends_accepted_postgres() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId, MessageType};
    use crate::gameplay_transport::connection::{
        ConnectionIdentifiers, ControlLossResult, FirstEntryOutcome, FreshAdmissionAuthority,
        ResumeAttempt, admit_frame,
    };
    use std::sync::Arc;
    use tokio::io::AsyncReadExt;

    struct Identifiers;
    impl ConnectionIdentifiers for Identifiers {
        fn game_session_id(&self) -> Option<GameSessionId> {
            GameSessionId::decode(&super::fixture::id(91)).ok()
        }
        fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
            AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
        }
    }

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "seam_fresh_spell_busy", true, false)
            .await?;
        let result: TestResult = async {
            let source = super::house_transfer::RecoverySource::start(&h).await?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
            let mut runtime = super::fresh_fixture::fresh::Source::new(i64::try_from(now)?)?;
            runtime.rows.retain(|row| matches!(row.key,
                crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
            let publication = crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(&runtime, i64::try_from(now)?).map_err(debug)?;
            h.root.publish_runtime_readiness(&h.node, &publication).await.map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &source.evidence;
            let token = super::house_transfer::RecoverySource::fresh_token(91)?;
            let frame = oteryn_protocol_oteryn::encode_client_bootstrap(&oteryn_protocol_oteryn::ClientBootstrapValue {
                schema_revision: 1,
                character_id: crate::foundation::CharacterId::decode(&super::fixture::id(41)).map_err(debug)?,
                admission_material: token.as_bytes(),
                client_build_id: "nbgrant-busy-root",
                supported_capabilities: &[],
            }).map_err(debug)?;
            let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
            let finished = tokio::sync::Notify::new();
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let admitting = async {
                let admitted = crate::gameplay_transport::composed_admission::FRESH_SPELL_ADMISSION_BARRIER
                    .scope(barrier.clone(), admit_frame(&mut server, &frame, &owner, &Identifiers)).await;
                finished.notify_one();
                admitted
            };
            let holding = async {
                barrier.0.notified().await;
                let mut held = h.root.try_acquire_ready().map_err(debug)?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                    .fetch_one(&h.pool).await?;
                assert_eq!(count, 1, "the real fresh COMMIT precedes the held-root read");
                barrier.1.notify_one();
                // Keep the real holder until admission finishes, including budget exhaustion.
                finished.notified().await;
                held.return_to_pool_observed_until(std::time::Instant::now() + crate::durability::DB_PASS_DEADLINE).await;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let (admitted, returned) = tokio::time::timeout(Duration::from_secs(20), async {
                tokio::join!(admitting, holding)
            }).await?;
            returned?;
            drop(server);
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            let admitted = admitted.map_err(|error| format!("committed winner closed: {error:?}; reply_bytes={}", output.len()))?;
            assert_eq!(admitted.first_entry, FirstEntryOutcome::AwaitingSpellRepair,
                "unavailable initialization must keep commands fenced");
            let length = u32::from_be_bytes(output.get(..4).ok_or("ServerAccepted frame")?.try_into()?) as usize;
            let reply = crate::foundation::decode_wire_envelope(output.get(4..4 + length).ok_or("complete ServerAccepted frame")?).map_err(debug)?;
            assert_eq!(reply.message_type(), MessageType::ServerAccepted);
            assert_eq!(output.len(), length + 4, "exactly one acceptance");
            h.root.await_ready().await?;
            assert_eq!(owner.lose_control(admitted, Duration::ZERO, None).await,
                ControlLossResult::Recorded, "the acknowledged identity enters the loss lifecycle");
            let recovery = super::house_transfer::RecoverySource::token(92)?;
            let resumed = owner.resume(ResumeAttempt {
                game_session_id: admitted.game_session_id,
                recovery_material: recovery.as_bytes(),
                transport: AuthenticatedTransportRefV1::decode(&[92; 16]).map_err(debug)?,
                last_applied_server_sequence: 0,
                supported_capabilities: &[],
            }).await.map_err(debug)?;
            assert_eq!(resumed.game_session_id, admitted.game_session_id);
            assert!(matches!(resumed.first_entry,
                FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled),
                "the committed session must become playable after holder return: {:?}", resumed.first_entry);
            eprintln!("PG_EXECUTED seam_fresh_spell_busy committed=1 server_accepted=1 input_eligible=false resume=recovered");
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

#[test]
fn seam_fresh_commit_reconciliation_waits_for_busy_root_postgres() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "seam_fresh_commit_busy", false, false)
            .await?;
        let result: TestResult = async {
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let session = deps.admitted.game_session_id;
            let transport = deps.admitted.controller.ok_or("fixture controller")?.transport;
            let held = h.root.try_issue_semantic_pass()?;
            let mut reconcile = std::pin::pin!(owner.reconcile(&request, session, transport));
            // Keep the actual holder occupied past the old five-read/800 ms window.
            // Await the attempted read, rather than sleeping before starting it.
            let premature = tokio::time::timeout(Duration::from_millis(1100), &mut reconcile).await;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(premature.is_err(), "a committed winner cannot become Unknown on a busy holder");
            assert!(matches!(reconcile.await, ReconciliationDisposition::Committed));
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&h.pool).await?;
            assert_eq!(count, 1, "reconciliation must not repeat the admission commit");
            eprintln!("PG_EXECUTED seam_fresh_commit_busy committed=1 reconciled=committed held_ms=1100");
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
