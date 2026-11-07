#![allow(clippy::expect_used)]
use super::*;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::world_spatial::{
    ActorPosition, StepDirection, StepDisposition, WorldSpatialObservation,
};
use oteryn_protocol_oteryn::{
    MessageType, decode_snapshot_body, decode_snapshot_chunk_framing, decode_state_delta,
    decode_wire_envelope,
};
use std::cell::Cell;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Authority {
    streams: crate::gameplay_transport::analyser::AnalyserStreams,
    ready: Cell<bool>,
    snapshots: Cell<usize>,
}
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn resume(&self, _: ResumeAttempt<'_>) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }
    async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }
    async fn analyser_snapshot(&self, admitted: AdmittedSession) -> Option<bool> {
        self.snapshots.set(self.snapshots.get() + 1);
        self.streams.snapshot(
            admitted.runtime_actor?,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            None,
        )
    }
    async fn analyser_next(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        let actor = admitted.runtime_actor?;
        if self.ready.replace(false) {
            self.streams.push(
                actor,
                admitted.game_session_id,
                wire::AnalyserFact::Kill { race: None },
            );
        }
        self.streams.next(
            actor,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )
    }
}

#[test]
fn analyser_connection_sends_empty_snapshot_and_revisioned_delta_only_when_selected() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("runtime")
                .block_on(async {
                    for selected in [false, true] {
                        let (runtime, actor, session) =
                            crate::gameplay_transport::actor_spell::tests::runtime_with_player(31);
                        let crate::foundation::RuntimeScopeRefV1::Channel {
                            world_id,
                            channel_id,
                        } = runtime.binding().runtime_scope()
                        else {
                            unreachable!("channel fixture")
                        };
                        let mut continuity = SessionContinuity::FRESH;
                        if selected {
                            continuity.selected_capabilities = SelectedCapabilities::select(
                                PRODUCTION_OFFERED_CAPABILITIES,
                                &[10],
                            )
                            .expect("selection");
                        }
                        let admitted = AdmittedSession {
                            game_session_id: session,
                            world_id,
                            channel_id,
                            runtime_actor: Some(actor),
                            first_entry: FirstEntryOutcome::Positioned,
                            controller: None,
                            continuity,
                            item_fence: None,
                        };
                        let authority = Authority {
                            streams: Default::default(),
                            ready: Cell::new(true),
                            snapshots: Cell::new(0),
                        };
                        let (mut server, mut client) = tokio::io::duplex(1 << 20);
                        client.shutdown().await.expect("peer closes input");
                        let _ =
                            serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
                        drop(server);
                        let mut bytes = Vec::new();
                        client.read_to_end(&mut bytes).await.expect("frames");
                        let (mut snapshots, mut deltas, mut cursor) = (0, 0, 0);
                        while cursor < bytes.len() {
                            let size = u32::from_be_bytes(
                                bytes[cursor..cursor + 4].try_into().expect("length"),
                            ) as usize;
                            cursor += 4;
                            let envelope = decode_wire_envelope(&bytes[cursor..cursor + size])
                                .expect("envelope");
                            cursor += size;
                            if envelope.message_type() == MessageType::SnapshotChunk {
                                let (_, body) = decode_snapshot_chunk_framing(envelope.payload())
                                    .expect("chunk");
                                for domain in decode_snapshot_body(body).expect("domains") {
                                    if domain.domain_id == 15 {
                                        assert_eq!(domain.revision, 1);
                                        assert!(domain.payload.is_empty());
                                        snapshots += 1;
                                    }
                                }
                            }
                            if envelope.message_type() == MessageType::StateDelta {
                                let delta = decode_state_delta(envelope.payload()).expect("delta");
                                if delta.domain_id == 15 {
                                    assert_eq!(
                                        (delta.base_revision, delta.new_revision, delta.delta_type),
                                        (1, 2, 1)
                                    );
                                    assert_eq!(
                                        wire::decode_analyser_facts(delta.payload).expect("facts"),
                                        vec![wire::AnalyserFact::Kill { race: None }]
                                    );
                                    deltas += 1;
                                }
                            }
                        }
                        assert_eq!(
                            (snapshots, deltas, authority.snapshots.get()),
                            if selected { (1, 1, 1) } else { (0, 0, 0) }
                        );
                        if selected {
                            let mut recovered = admitted;
                            recovered.continuity.analyser_revision = 2;
                            let mut domains = Vec::new();
                            assert!(
                                snapshot(&authority, &mut recovered, &mut domains)
                                    .await
                                    .is_ok(),
                                "empty recovery snapshot"
                            );
                            assert_eq!(domains[0].revision, 2);
                            authority.streams.push(
                                actor,
                                session,
                                wire::AnalyserFact::Kill { race: None },
                            );
                            domains.clear();
                            assert!(
                                snapshot(&authority, &mut recovered, &mut domains)
                                    .await
                                    .is_ok(),
                                "discard queued facts"
                            );
                            assert_eq!(domains[0].revision, 3);
                            domains.clear();
                            assert!(
                                snapshot(&authority, &mut recovered, &mut domains)
                                    .await
                                    .is_ok(),
                                "repeated recovery snapshot"
                            );
                            assert_eq!(domains[0].revision, 3);
                        }
                    }
                });
        })
        .expect("scenario thread")
        .join()
        .expect("scenario");
}
