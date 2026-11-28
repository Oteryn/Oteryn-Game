use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::durability::{DB_PASS_DEADLINE, ROOT_RECOVERY_WINDOW};
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId, MessageType};
use crate::gameplay_transport::composed_admission::FRESH_SPELL_ADMISSION_BARRIER;
use crate::gameplay_transport::connection::join_recovery::{JOIN_RECOVERY, JoinRecovery};
use crate::gameplay_transport::connection::{
    ConnectionEnd, ConnectionIdentifiers, FreshAdmissionAuthority, IDLE_LIVENESS, admit_frame_by,
    serve_admitted_by,
};
use crate::gameplay_transport::tcp_tls::read_frame;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

struct Identifiers;
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(91)).ok()
    }

    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
    }
}

#[test]
fn busy_holder_join_shared_deadline_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            runtime
                .block_on(async move {
                    for expire in [false, true] {
                        let tag = if expire {
                            "join_spell_deadline"
                        } else {
                            "join_spell_release"
                        };
                        let h =
                            Harness::create_with_admission_seed(admin.clone(), tag, true, false)
                                .await?;
                        let result = exercise(&h, expire).await;
                        h.cleanup().await?;
                        result?;
                    }
                    Ok::<_, Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "join deadline PG thread failed")??;
    Ok(())
}

async fn exercise(h: &Harness, expire: bool) -> TestResult {
    let source = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };
    let mut runtime = super::fresh_fixture::fresh::Source::new(now)?;
    runtime
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
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
    let dependencies = Dependencies::new()?;
    let mut owner = dependencies.owner(h, &authority);
    owner.evidence = &source.evidence;
    let token = super::house_transfer_fixtures::RecoverySource::fresh_token(91)?;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: CharacterId::decode(&id(41)).map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "busy-holder-join",
            supported_capabilities: &[],
        },
    )
    .map_err(debug)?;
    if expire {
        // An unissued attempt keeps the same authenticated grant usable.
        let mut held = h.root.try_acquire_ready()?;
        let (mut server, mut peer) = tokio::io::duplex(1 << 20);
        let deadline = tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW;
        let end = admit_frame_by(&mut server, &frame, &owner, &Identifiers, deadline).await;
        assert!(matches!(
            end,
            Err(ConnectionEnd::AdmissionRefused(
                crate::gameplay_transport::connection::AdmissionRefusal::Unavailable
            ))
        ));
        drop(server);
        assert_eq!(
            read_frame(&mut peer).await.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 0);
        assert_eq!(
            held.return_to_pool_observed_until(Instant::now() + DB_PASS_DEADLINE)
                .await,
            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle,
        );
    }
    let barrier = Arc::new((Notify::new(), Notify::new()));
    let release = Notify::new();
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let deadline = tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW;
    let connect = JOIN_RECOVERY.scope(JoinRecovery::default(), async {
        let end = match FRESH_SPELL_ADMISSION_BARRIER
            .scope(
                barrier.clone(),
                admit_frame_by(&mut server, &frame, &owner, &Identifiers, deadline),
            )
            .await
        {
            Ok(admitted) => {
                serve_admitted_by(&mut server, admitted, &owner, IDLE_LIVENESS, Some(deadline))
                    .await
            }
            Err(end) => end,
        };
        drop(server);
        release.notify_one();
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        let recovery = if expire {
            let pending = JOIN_RECOVERY
                .with(|join| join.take_pending())
                .ok_or("missing recovery handoff")?;
            assert_eq!(
                pending.source.game_session_id,
                Identifiers.game_session_id().ok_or("session")?
            );
            Some(pending)
        } else {
            None
        };
        Ok::<_, Box<dyn std::error::Error>>((end, recovery))
    });
    let holding = async {
        barrier.0.notified().await;
        let mut held = h.root.try_acquire_ready()?;
        barrier.1.notify_one();
        if expire {
            release.notified().await;
        } else {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            held.return_to_pool_observed_until(Instant::now() + DB_PASS_DEADLINE)
                .await,
            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle,
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let peer = async {
        let first = read_frame(&mut client).await?;
        let first = crate::foundation::decode_wire_envelope(&first).map_err(debug)?;
        if expire {
            assert_eq!(first.message_type(), MessageType::ProtocolError);
            assert_eq!(first.connection_generation(), 0);
            let error =
                oteryn_protocol_oteryn::decode_protocol_error(first.payload()).map_err(debug)?;
            assert_eq!(error.error_code, 1051);
            assert_eq!(
                read_frame(&mut client).await.unwrap_err().kind(),
                std::io::ErrorKind::UnexpectedEof
            );
        } else {
            assert_eq!(first.message_type(), MessageType::ServerAccepted);
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::SnapshotCommit {
                    break;
                }
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
            }
            drop(client);
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (connected, held, peer) =
        tokio::time::timeout(ROOT_RECOVERY_WINDOW + Duration::from_secs(3), async {
            tokio::join!(connect, holding, peer)
        })
        .await?;
    let (end, recovery) = connected?;
    held?;
    peer?;
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts, 1,
        "the expired committed join preserves its original grant receipt"
    );
    if let (ConnectionEnd::AdmittedThenDisconnected(ended), Some(pending)) = (end, recovery) {
        let ended = owner
            .settle_unpublished_join(&pending, ended)
            .await
            .map_err(|()| "settle committed join")?;
        assert_eq!(
            owner.lose_control(ended, Duration::ZERO, None).await,
            crate::gameplay_transport::connection::ControlLossResult::Recorded,
        );
        let token = super::house_transfer_fixtures::RecoverySource::token(92)?;
        let resumed = super::house_transfer_fixtures::resume_same_session(
            &owner,
            ended.game_session_id,
            &token,
            92,
            &[],
        )
        .await?;
        assert_eq!(resumed.runtime_actor, ended.runtime_actor);
        assert_eq!(resumed.game_session_id, ended.game_session_id);
        assert!(resumed.continuity.connection_generation > ended.continuity.connection_generation);
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            let deadline = tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW;
            let end =
                serve_admitted_by(&mut server, resumed, &owner, IDLE_LIVENESS, Some(deadline))
                    .await;
            drop(server);
            end
        };
        let reading = async {
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
                if envelope.message_type() == MessageType::SnapshotCommit {
                    break;
                }
            }
            drop(client);
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (end, result) = tokio::time::timeout(ROOT_RECOVERY_WINDOW, async {
            tokio::join!(serving, reading)
        })
        .await?;
        result?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(rows, 1, "reconnect preserves one durable controller");
    }
    Ok(())
}
