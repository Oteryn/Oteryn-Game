//! A fresh admission completes while other owners keep using the single durability holder.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async as run;
use super::{Dependencies, TestResult};
use std::time::Duration;

#[test]
fn fresh_admission_completes_while_other_owners_share_the_holder_postgres() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
    use crate::gameplay_transport::FreshAdmissionAttempt;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "fresh_holder_contended", true, false)
            .await?;
        let result: TestResult = async {
            let source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let mut runtime = super::fresh_fixture::fresh::Source::new(i64::try_from(now)?)?;
            runtime.rows.retain(|row| {
                matches!(
                    row.key,
                    crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)
                )
            });
            let publication =
                crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(
                    &runtime,
                    i64::try_from(now)?,
                )
                .map_err(debug)?;
            h.root
                .publish_runtime_readiness(&h.node, &publication)
                .await
                .map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &source.evidence;
            let token = super::house_transfer_fixtures::RecoverySource::fresh_token(91)?;
            let session = GameSessionId::decode(&super::fixture::id(91)).map_err(debug)?;
            let attempt = FreshAdmissionAttempt {
                character_id: CharacterId::decode(&super::fixture::id(41)).map_err(debug)?,
                admission_material: token.as_bytes(),
                game_session_id: session,
                transport: AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?,
                supported_capabilities: &[],
            };

            // Other owners keep the single holder about three quarters busy with short passes and
            // retry it through the ordinary command budget whenever they find it busy, as a served
            // session and the Channel cadences do on a node while two players join
            // (holder_window: 77 % held).
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut competitors = Vec::new();
            for think in [13_u64, 17, 21, 25] {
                let root = h.root.clone();
                let stop = stop.clone();
                competitors.push(tokio::spawn(async move {
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = crate::gameplay_transport::AdmissionRetryBudget::for_command(&root)
                            .retry_durability("contended_holder", async || {
                                root.try_issue_semantic_pass()?
                                    .run(|_, _| {
                                        Box::pin(async {
                                            tokio::time::sleep(Duration::from_millis(5)).await;
                                            Ok(())
                                        })
                                    })
                                    .await
                            })
                            .await;
                        tokio::time::sleep(Duration::from_millis(think)).await;
                    }
                }));
            }
            let admitted = owner.admit(attempt).await;
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            for competitor in competitors {
                competitor.await?;
            }
            let admitted = admitted.map_err(|refusal| {
                format!("a contended holder refused the fresh admission: {refusal:?}")
            })?;
            assert_eq!(admitted.game_session_id, session);
            assert!(matches!(
                admitted.first_entry,
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned
                    | crate::gameplay_transport::connection::FirstEntryOutcome::Reconciled
            ), "committed admission must be playable: {:?}", admitted.first_entry);
            eprintln!("PG_EXECUTED fresh_holder_contended admitted=1");
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

#[test]
fn fresh_spell_repair_becomes_playable_without_reconnect_postgres() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId, MessageType};
    use crate::gameplay_transport::connection::{
        ConnectionEnd, ConnectionIdentifiers, FirstEntryOutcome, FreshAdmissionAuthority,
        LivenessPolicy, admit_frame, serve_admitted,
    };
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        let h = Harness::create_with_admission_seed(admin, "fresh_spell_repair_live", true, false)
            .await?;
        let result: TestResult = async {
            let source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
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
            let token = super::house_transfer_fixtures::RecoverySource::fresh_token(91)?;
            let frame = oteryn_protocol_oteryn::encode_client_bootstrap(&oteryn_protocol_oteryn::ClientBootstrapValue {
                schema_revision: 1,
                character_id: crate::foundation::CharacterId::decode(&super::fixture::id(41)).map_err(debug)?,
                admission_material: token.as_bytes(),
                client_build_id: "nbgrant-busy-root",
                supported_capabilities: &[],
            }).map_err(debug)?;
            let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
            let release = tokio::sync::Notify::new();
            let returned = tokio::sync::Notify::new();
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let admitting = async {
                let admitted = crate::gameplay_transport::composed_admission::FRESH_SPELL_ADMISSION_BARRIER
                    .scope(barrier.clone(), admit_frame(&mut server, &frame, &owner, &Identifiers)).await;
                let admitted = admitted.map_err(|error| format!("fresh admission: {error:?}"))?;
                assert_eq!(admitted.first_entry, FirstEntryOutcome::AwaitingSpellRepair);
                release.notify_one();
                returned.notified().await;
                h.root.await_ready().await?;
                let mut stale = admitted;
                stale.continuity.connection_generation = 999;
                stale.item_fence.as_mut().ok_or("admitted fence")?.connection_generation =
                    crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
                assert!(!owner.repair_pending_spells(stale).await,
                    "a retired connection must not borrow the current fence");
                let end = serve_admitted(&mut server, admitted, &owner, LivenessPolicy {
                    interval: Duration::from_millis(100), missed_limit: 30,
                }).await;
                match end {
                    ConnectionEnd::AdmittedThenDisconnected(admitted) => {
                        assert_eq!(admitted.first_entry, FirstEntryOutcome::Reconciled);
                    }
                    end => return Err(format!("connection did not become playable: {end:?}").into()),
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let holding = async {
                barrier.0.notified().await;
                let mut held = h.root.try_acquire_ready().map_err(debug)?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                    .fetch_one(&h.pool).await?;
                assert_eq!(count, 1, "the real fresh COMMIT precedes the held-root read");
                barrier.1.notify_one();
                // Keep the real holder until admission finishes, including budget exhaustion.
                release.notified().await;
                held.return_to_pool_observed_until(std::time::Instant::now() + crate::durability::DB_PASS_DEADLINE).await;
                returned.notify_one();
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let reading = async {
                let mut accepted = 0;
                loop {
                    let length = client.read_u32().await? as usize;
                    let mut frame = vec![0; length];
                    client.read_exact(&mut frame).await?;
                    let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
                    match envelope.message_type() {
                        MessageType::ServerAccepted => accepted += 1,
                        MessageType::LivenessProbe => {
                            let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
                            let ack = oteryn_protocol_oteryn::encode_liveness_ack(envelope.connection_generation(), probe, 0).map_err(debug)?;
                            client.write_u32(u32::try_from(ack.len())?).await?;
                            client.write_all(&ack).await?;
                        }
                        MessageType::SnapshotBegin | MessageType::SnapshotChunk => {}
                        MessageType::SnapshotCommit => break,
                        kind => return Err(format!("unexpected pre-play frame: {kind:?}").into()),
                    }
                }
                assert_eq!(accepted, 1, "no reconnect or second admission");
                client.shutdown().await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            let (served, returned, read) = tokio::time::timeout(Duration::from_secs(25), async {
                tokio::join!(admitting, holding, reading)
            }).await?;
            served?;
            returned?;
            read?;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&h.pool).await?;
            assert_eq!(count, 1);
            eprintln!("PG_EXECUTED fresh_spell_repair_live committed=1 server_accepted=1 snapshot_commit=1 reconnect=0");
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
