//! The listener's one-bootstrap path uses the real fresh owner and mandatory snapshot reads.
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::RecoverySource;
use super::{Dependencies, TestResult};
use crate::durability::DurabilityRoot;
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
use crate::gameplay_transport::connection::{
    BEFORE_JOIN_SNAPSHOT, ConnectionEnd, ConnectionIdentifiers, FreshAdmissionAuthority,
    IDLE_LIVENESS, LogoutReply, admit_frame_for_join, serve_admitted_for_join,
};
use crate::gameplay_transport::tcp_tls::read_frame;
use oteryn_protocol_oteryn::actor_spell_catalog::CAPABILITY_ACTOR_SPELL_CATALOG_V1;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, FoundationProtocolError, MessageType, decode_protocol_error,
    decode_wire_envelope, encode_client_bootstrap,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt as _;
use tokio::sync::Notify;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Relog,
    Snapshot,
    Exhausted,
}

#[test]
fn w13_logout_then_immediate_relog_waits_for_the_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Relog)
}

#[test]
fn w13_initial_join_snapshot_waits_for_the_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Snapshot)
}

#[test]
fn w13_join_refuses_with_typed_holder_cause_at_deadline_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Exhausted)
}

fn run(admin: String, case: Case) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-join-release", move || {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let mut result = Ok(());
        let lines = oteryn_error_codes::capture(|| {
            result = runtime.block_on(scenario(admin, case));
        });
        result?;
        assert!(lines.iter().any(|line| line.contains("event=admission_retry")
            && line.contains("phase=spell_catalog")
            && line.contains("cause=holder_busy")));
        if case == Case::Relog {
            assert!(lines.iter().any(|line| line.contains("event=admission_retry")
                && line.contains("phase=fresh_admission")
                && line.contains("cause=holder_busy")));
        }
        if case == Case::Exhausted {
            assert!(lines.iter().any(|line| line.contains("event=serve_end")
                && line.contains("reason=join_refused")
                && line.contains("phase=spell_catalog")
                && line.contains("cause=holder_busy")));
        }
        Ok(())
    })
}

struct Identifiers(u8);

impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(self.0)).ok()
    }

    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}

fn bootstrap(nonce: u8, supported: &[u32]) -> TestResult<Vec<u8>> {
    let token = RecoverySource::fresh_token(nonce)?;
    encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: token.as_bytes(),
        client_build_id: "w13-join-release",
        supported_capabilities: supported,
    })
    .map_err(debug)
}

async fn scenario(admin: String, case: Case) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "w13_join_release", true, false).await?;
    let result = exercise(&h, case).await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };

    let source = RecoverySource::start(h).await?;
    let now = i64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs())?;
    let mut readiness = super::fresh_fixture::fresh::Source::new(now)?;
    readiness.rows.retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    h.root.publish_runtime_readiness(
        &h.node, &AdmissionAuthorityPublicationV1::prepare(&readiness, now).map_err(debug)?,
    ).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &source.evidence;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    if case != Case::Exhausted {
        let frame = bootstrap(91, &[])?;
        let (old, pending) = admit_frame_for_join(&mut server, &frame, &owner, &Identifiers(91))
            .await.map_err(debug)?;
        assert!(pending.is_some(), "the old actor is a normal Channel entry");
        // Exercise the terminal release owner and assert its exact successful result.
        assert_eq!(owner.logout(old).await, LogoutReply::Result(LogoutResult::Accepted));
        let terminal: bool = sqlx::query_scalar(
            "SELECT session_state=3 FROM game_durability_reconnect_sessions
                WHERE game_session_id=encode($1,'hex')::uuid",
        ).bind(old.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert!(terminal);
    }

    let supported = [CAPABILITY_ACTOR_SPELL_CATALOG_V1];
    let frame = bootstrap(92, &supported)?;
    let ids = Identifiers(92);
    let (admitted, mut pending) = {
        let mut admission = std::pin::pin!(DurabilityRoot::with_holder_owner(
            admit_frame_for_join(&mut server, &frame, &owner, &ids),
        ));
        if case == Case::Relog {
            let holder = h.root.try_issue_semantic_pass()?;
            assert!(tokio::time::timeout(Duration::from_millis(100), &mut admission).await.is_err());
            holder.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        admission.await.map_err(debug)?
    };
    assert!(admitted.continuity.selected_capabilities.contains(supported[0]));
    assert!(pending.is_some(), "normal fresh Channel admission defers acceptance");

    let barrier = Arc::new((Notify::new(), Notify::new()));
    let returned = Notify::new();
    let finished = Notify::new();
    let serving = async {
        let end = DurabilityRoot::with_holder_owner(BEFORE_JOIN_SNAPSHOT.scope(
            barrier.clone(),
            serve_admitted_for_join(&mut server, admitted, &owner, IDLE_LIVENESS, &mut pending),
        )).await;
        finished.notify_one();
        end
    };
    let contending = async {
        barrier.0.notified().await;
        let mut holder = h.root.try_acquire_ready()?;
        barrier.1.notify_one();
        if case == Case::Exhausted {
            finished.notified().await;
        } else {
            assert!(tokio::time::timeout(
                crate::gameplay_transport::COMMAND_HOLDER_WINDOW + Duration::from_millis(250),
                finished.notified(),
            ).await.is_err(), "snapshot refused while the holder was still settling");
        }
        holder.return_to_pool_observed_until(Instant::now() + crate::durability::DB_PASS_DEADLINE).await;
        returned.notify_one();
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let receiving = async {
        let first = read_frame(&mut client).await?;
        let envelope = decode_wire_envelope(&first).map_err(debug)?;
        if case == Case::Exhausted {
            assert_eq!(envelope.message_type(), MessageType::ProtocolError);
            assert_eq!(envelope.connection_generation(), 0);
            assert_eq!(decode_protocol_error(envelope.payload()).map_err(debug)?.error_code,
                FoundationProtocolError::SessionJoinUnavailable.code());
        } else {
            assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
            loop {
                let frame = read_frame(&mut client).await?;
                match decode_wire_envelope(&frame).map_err(debug)?.message_type() {
                    MessageType::SnapshotCommit => break,
                    MessageType::SnapshotBegin | MessageType::SnapshotChunk => {}
                    kind => return Err(format!("unexpected snapshot frame {kind:?}").into()),
                }
            }
        }
        returned.notified().await;
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, contend, receive) = tokio::time::timeout(Duration::from_secs(22), async {
        tokio::join!(serving, contending, receiving)
    }).await?;
    contend?;
    receive?;
    if case == Case::Exhausted {
        assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, FoundationProtocolError::SessionJoinUnavailable)));
        assert!(pending.is_some());
    } else {
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        assert!(pending.is_none());
    }
    Ok(())
}
