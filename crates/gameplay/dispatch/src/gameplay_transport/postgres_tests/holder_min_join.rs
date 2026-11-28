use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::durability::{DB_PASS_DEADLINE, DurabilityRoot};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId, MessageType};
use crate::gameplay_transport::composed_admission::FRESH_CHECKOUT_BARRIER;
use crate::gameplay_transport::connection::{ConnectionIdentifiers, admit_frame};
use crate::gameplay_transport::tcp_tls::read_frame;
use std::{
    future::{Future, poll_fn},
    sync::Arc,
    task::Poll,
    time::Duration,
};
use tokio::{io::AsyncReadExt, sync::Notify};

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
fn holder_min_brief_busy_holder_during_join_admits_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "holder_min_brief", false)
}

#[test]
fn holder_min_join_past_bound_sends_one_e1051_and_reconnect_succeeds_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "holder_min_deadline", true)
}

fn run(admin: String, tag: &'static str, expire: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run(tag, move || {
        crate::bestiary_postgres_harness::runtime()?.block_on(async move {
            let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
            let outcome = exercise(&h, expire).await;
            h.cleanup().await?;
            outcome
        })
    })
}

async fn exercise(h: &Harness, expire: bool) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };
    let source = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
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
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &source.evidence;
    let session = Identifiers.game_session_id().ok_or("session identifier")?;
    let token = super::house_transfer_fixtures::RecoverySource::fresh_token(91)?;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "holder-min",
            supported_capabilities: &[],
        },
    )
    .map_err(debug)?;
    let (mut server, mut peer) = tokio::io::duplex(1 << 20);
    let barrier = Arc::new((Notify::new(), Notify::new()));
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut joining = std::pin::pin!(FRESH_CHECKOUT_BARRIER.scope(
            barrier.clone(), DurabilityRoot::with_holder_owner(admit_frame(&mut server, &frame, &owner, &Identifiers)),
        ));
        tokio::select! {
            _ = barrier.0.notified() => {},
            end = &mut joining => return Err(format!("ended before checkout: {end:?}").into()),
        }
        let mut held = h.root.try_acquire_ready()?;
        barrier.1.notify_one();
        assert!(poll_fn(|cx| Poll::Ready(joining.as_mut().poll(cx))).await.is_pending());
        if expire {
            assert!(matches!(
                joining.as_mut().await,
                Err(crate::gameplay_transport::connection::ConnectionEnd::AdmissionRefused(
                    crate::gameplay_transport::connection::AdmissionRefusal::Classified(
                        crate::foundation::FoundationProtocolError::SessionJoinUnavailable
                    )
                ))
            ));
            assert!(!owner.runtime.lock().await.contains_player_session(session).map_err(debug)?);
            let consumed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts").fetch_one(&h.pool).await?;
            assert_eq!(consumed, 0);
        }
        assert_eq!(
            held.return_to_pool_observed_until(std::time::Instant::now() + DB_PASS_DEADLINE).await,
            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle,
        );
        if !expire {
            tokio::select! {
                _ = barrier.0.notified() => return Err("the join retried the admission round".into()),
                result = &mut joining => {
                    assert!(result.map_err(|error| format!("{error:?}"))?.runtime_actor.is_some());
                },
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    }).await??;
    drop(server);
    let reply = read_frame(&mut peer).await?;
    let envelope = crate::foundation::decode_wire_envelope(&reply).map_err(debug)?;
    if expire {
        assert_eq!(envelope.message_type(), MessageType::ProtocolError);
        assert_eq!(envelope.connection_generation(), 0);
        let error =
            oteryn_protocol_oteryn::decode_protocol_error(envelope.payload()).map_err(debug)?;
        assert_eq!(
            error.error_code,
            crate::foundation::FoundationProtocolError::SessionJoinUnavailable.code()
        );
        assert_eq!(
            error.disposition,
            oteryn_protocol_oteryn::ProtocolDisposition::TransportFatal
        );
        let mut trailing = Vec::new();
        assert_eq!(peer.read_to_end(&mut trailing).await?, 0);
        h.root.await_ready().await?;
        let (mut server, mut peer) = tokio::io::duplex(1 << 20);
        let admitted = admit_frame(&mut server, &frame, &owner, &Identifiers)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(admitted.runtime_actor.is_some());
        let reply = read_frame(&mut peer).await?;
        assert_eq!(
            crate::foundation::decode_wire_envelope(&reply)
                .map_err(debug)?
                .message_type(),
            MessageType::ServerAccepted
        );
    } else {
        assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
    }
    eprintln!("PG_EXECUTED holder_min_join expire={expire}");
    Ok(())
}
