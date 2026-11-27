//! A fresh Channel publishes its acceptance only after it can build the join snapshot.
use super::fixture::{debug, id};
use super::{Dependencies, TestResult};
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, ExactActorRef, FoundationProtocolError, GameSessionId,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, ConnectionIdentifiers, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, admit_frame_for_join,
    serve_admitted_for_join,
};
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, MessageType, decode_protocol_error, decode_wire_envelope,
    encode_client_bootstrap,
    world_spatial::{ActorPosition, WorldSpatialObservation},
};
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;
use tokio::sync::Notify;

struct Entry {
    admitted: AdmittedSession,
    preparing: Notify,
    proceed: Notify,
    playable: bool,
}

impl ConnectionIdentifiers for Entry {
    fn game_session_id(&self) -> Option<GameSessionId> {
        Some(self.admitted.game_session_id)
    }

    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        self.admitted
            .controller
            .map(|controller| controller.transport)
    }
}

impl FreshAdmissionAuthority for Entry {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Ok(self.admitted)
    }

    async fn repair_pending_spells(&self, _: AdmittedSession) -> bool {
        true
    }

    async fn prepare_quest_callback_resume(&self, _: &mut AdmittedSession) -> bool {
        self.preparing.notify_one();
        self.proceed.notified().await;
        self.playable
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [1; 32],
            actor_position: ActorPosition {
                x: 1,
                y: 1,
                floor: 0,
            },
        })
    }
}

#[test]
fn fresh_join_waits_for_snapshot_or_refuses_before_acceptance() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("fresh-join-acceptance", || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                exercise(FirstEntryOutcome::Positioned, true).await?;
                exercise(FirstEntryOutcome::AwaitingSpellRepair, true).await?;
                exercise(FirstEntryOutcome::Positioned, false).await
            })
    })
}

async fn exercise(first_entry: FirstEntryOutcome, playable: bool) -> TestResult {
    let deps = Dependencies::new()?;
    let entry = Entry {
        admitted: AdmittedSession {
            first_entry,
            ..deps.admitted
        },
        preparing: Notify::new(),
        proceed: Notify::new(),
        playable,
    };
    let bootstrap = encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: b"test-grant",
        client_build_id: "fresh-join-acceptance",
        supported_capabilities: &[],
    })
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = async {
        let (admitted, mut acceptance) =
            admit_frame_for_join(&mut server, &bootstrap, &entry, &entry)
                .await
                .map_err(debug)?;
        let end = serve_admitted_for_join(
            &mut server,
            admitted,
            &entry,
            IDLE_LIVENESS,
            &mut acceptance,
        )
        .await;
        if playable {
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            assert!(acceptance.is_none());
        } else {
            assert_eq!(
                end,
                ConnectionEnd::AdmittedThenClosed(
                    admitted,
                    FoundationProtocolError::SessionJoinUnavailable
                )
            );
            assert!(acceptance.is_some());
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let drive = async {
        entry.preparing.notified().await;
        let first = if first_entry == FirstEntryOutcome::AwaitingSpellRepair {
            // This entry starts its liveness and repair protocol after acceptance.
            let accepted = super::item_look_fixtures::read_frame(&mut client).await?;
            entry.proceed.notify_one();
            accepted
        } else {
            let mut frame = std::pin::pin!(super::item_look_fixtures::read_frame(&mut client));
            use std::future::Future as _;
            let early =
                std::future::poll_fn(|cx| std::task::Poll::Ready(frame.as_mut().poll(cx))).await;
            assert!(early.is_pending(), "acceptance preceded join preparation");
            entry.proceed.notify_one();
            frame.await?
        };
        let envelope = decode_wire_envelope(&first).map_err(debug)?;
        if playable {
            assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                    == MessageType::SnapshotCommit
                {
                    break;
                }
            }
        } else {
            assert_eq!(envelope.message_type(), MessageType::ProtocolError);
            assert_eq!(envelope.connection_generation(), 0);
            assert_eq!(
                decode_protocol_error(envelope.payload())?.error_code,
                FoundationProtocolError::SessionJoinUnavailable.code()
            );
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(serve, drive)
    })
    .await??;
    Ok(())
}
