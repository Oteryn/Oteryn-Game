//! GAME-JOIN-DOMAIN-FIX-1: one object domain per World over the served connection (MAP-WIRE-1
//! contract §3, ARCH-MAP-WIRE-1 §1.5). A session that selected capability 18 gets domain 17 and
//! never domain 2 on join, resume or delta; its door states arrive as domain 17 object
//! revisions. A session without 18 keeps domain 2 as before.

use super::*;
use crate::foundation::{
    DomainSnapshot, ExactActorRef, GameSessionId, MessageType, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, EarnedNotice, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity, StepOutcome,
    UseCommand, UseOutcome, serve_admitted,
};
use crate::gameplay_transport::item_view::InventoryItems;
use crate::gameplay_transport::world_object::{
    COMMAND_TYPE_USE_INTENT, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    WorldObjectOverlayEntry, encode_use_intent,
};
use crate::gameplay_transport::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, WorldSpatialObservation,
    encode_step_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_snapshot_chunk, decode_state_delta, encode_client_command,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn Error>>;

const DOOR_AT: TilePos = tp(21, 20, -7);
const DOOR_REVISION: u64 = 3;

/// A Channel serving the fixture bundle: domain 17 through the connection's own map view, the
/// own actor stepping south, and the USE outcome and legacy overlay the case injects.
struct BundleChannel {
    fixture: RefCell<Fixture>,
    position: Cell<TilePos>,
    overlay: Option<WorldObjectOverlayEntry>,
    use_outcome: UseOutcome,
}

impl BundleChannel {
    fn new() -> Self {
        let mut fixture = Fixture::new(vec![
            (ACTOR, vec![item(GRASS)]),
            (DOOR_AT, vec![item(GRASS), item(DOOR)]),
        ]);
        // The door's overlay state: the object revision domain 17 carries for it.
        fixture
            .facts
            .revisions
            .insert(key(DOOR_AT, 1), DOOR_REVISION);
        Self {
            fixture: RefCell::new(fixture),
            position: Cell::new(ACTOR),
            overlay: None,
            use_outcome: UseOutcome::rejected(),
        }
    }

    fn observation(&self) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [7; 32],
            actor_position: at(self.position.get()),
        }
    }
}

impl FreshAdmissionAuthority for BundleChannel {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(self.observation())
    }

    async fn observe_world_object_overlay(&self) -> Option<WorldObjectOverlayEntry> {
        self.overlay.clone()
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        let mut position = self.position.get();
        position.y += 1;
        self.position.set(position);
        StepOutcome {
            disposition: StepDisposition::Moved,
            moved_to: Some(self.observation()),
        }
    }

    async fn use_object(
        &self,
        _actor: ExactActorRef,
        _use: UseCommand,
        _target: WorldObjectTarget,
    ) -> UseOutcome {
        self.use_outcome.clone()
    }

    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems::default())
    }

    async fn observe_world_map(
        &self,
        _actor: ExactActorRef,
        view: &mut SessionMapView,
        items: &mut SessionItemView,
    ) -> Option<Result<Option<MapUpdate>, MapViewError>> {
        let fixture = self.fixture.borrow();
        Some(view.update(items, &fixture.source(), at(self.position.get())))
    }
}

fn session(supported: &[u32], continuity: SessionContinuity) -> AdmittedSession {
    AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid(3)).expect("session"),
        world_id: world(),
        channel_id: channel(),
        runtime_actor: Some(ExactActorRef::transport_fixture(world(), channel())),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                supported,
            )
            .expect("select"),
            ..continuity
        },
        item_fence: None,
    }
}

fn command(generation: u64, id: u64, command_type: u32, payload: &[u8]) -> Vec<u8> {
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id: id,
            command_type,
            payload,
        },
    )
    .expect("command");
    let mut framed = u32::try_from(frame.len())
        .expect("length")
        .to_be_bytes()
        .to_vec();
    framed.extend_from_slice(&frame);
    framed
}

fn step(generation: u64, id: u64) -> Vec<u8> {
    command(
        generation,
        id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &encode_step_intent(StepDirection::South),
    )
}

fn use_door(generation: u64, id: u64) -> Vec<u8> {
    let intent = encode_use_intent(&WorldObjectTarget {
        placement: b"door".to_vec(),
        expected_revision: 0,
    })
    .expect("use intent");
    command(generation, id, COMMAND_TYPE_USE_INTENT, &intent)
}

/// What one served connection sent: each snapshot domain with its payload, and each delta's
/// domain, in order.
struct Sent {
    end: ConnectionEnd,
    snapshot: Vec<(u32, u64, Vec<u8>)>,
    deltas: Vec<u32>,
}

impl Sent {
    fn snapshot_domains(&self) -> Vec<u32> {
        self.snapshot.iter().map(|(domain, _, _)| *domain).collect()
    }

    fn ended(self) -> AdmittedSession {
        match self.end {
            ConnectionEnd::AdmittedThenDisconnected(admitted) => admitted,
            other => panic!("unexpected end {other:?}"),
        }
    }
}

async fn serve(
    channel: &BundleChannel,
    admitted: AdmittedSession,
    client_frames: &[Vec<u8>],
) -> Sent {
    let (mut server, mut client) = tokio::io::duplex(1 << 22);
    for frame in client_frames {
        client.write_all(frame).await.expect("write");
    }
    client.shutdown().await.expect("shutdown");
    let end = serve_admitted(&mut server, admitted, channel, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.expect("read");
    let (mut snapshot, mut deltas, mut cursor) = (Vec::new(), Vec::new(), 0);
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into().expect("length"));
        let frame = &output[cursor + 4..cursor + 4 + length as usize];
        cursor += 4 + length as usize;
        let envelope = decode_wire_envelope(frame).expect("envelope");
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, domains) = decode_snapshot_chunk(envelope.payload()).expect("chunk");
                snapshot.extend(domains.iter().map(|domain: &DomainSnapshot<'_>| {
                    (domain.domain_id, domain.revision, domain.payload.to_vec())
                }));
            }
            MessageType::StateDelta => {
                deltas.push(
                    decode_state_delta(envelope.payload())
                        .expect("delta")
                        .domain_id,
                );
            }
            _ => {}
        }
    }
    Sent {
        end,
        snapshot,
        deltas,
    }
}

fn run(test: impl std::future::Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(test)
}

/// The bundle selection of a client that supports 4, 6 and 18: it selects 18.
const MAP_VIEW: [u32; 3] = [4, 6, 18];

#[test]
fn a_session_with_18_joins_steps_and_resumes_with_domain_17_and_never_domain_2() -> TestResult {
    run(async {
        let channel = BundleChannel::new();
        let joined = serve(
            &channel,
            session(&MAP_VIEW, SessionContinuity::FRESH),
            &[step(1, 1)],
        )
        .await;
        assert!(
            joined
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
        assert!(
            !joined
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        // The step's deltas: domain 1 and domain 17, no domain 2.
        assert!(joined.deltas.contains(&STATE_DOMAIN_WORLD_MAP_VIEW));
        assert!(!joined.deltas.contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY));
        // The door's state is carried by domain 17: its base entry with the overlay revision.
        let (_, _, map) = joined
            .snapshot
            .iter()
            .find(|(domain, _, _)| *domain == STATE_DOMAIN_WORLD_MAP_VIEW)
            .ok_or("domain 17")?;
        let map = decode_world_map_snapshot(map).map_err(|error| format!("{error:?}"))?;
        let door = map
            .tiles
            .iter()
            .find(|tile| tile.position == at(DOOR_AT))
            .ok_or("door tile")?;
        assert_eq!(
            door.items[1].origin,
            MapOrigin::BaseOrdinal {
                ordinal: 1,
                object_revision: DOOR_REVISION
            }
        );

        // The resumed connection's snapshot: domain 17 above the revision sent, no domain 2.
        let lost = joined.ended();
        assert_eq!(lost.continuity.overlay_revision, 0);
        let sent_map_revision = lost.continuity.world_map_revision;
        let resumed_continuity = SessionContinuity {
            connection_generation: 2,
            ..lost.continuity
        };
        let resumed = serve(
            &channel,
            AdmittedSession {
                continuity: resumed_continuity,
                ..lost
            },
            &[step(2, lost.continuity.next_command_id)],
        )
        .await;
        let map_revision = resumed
            .snapshot
            .iter()
            .find(|(domain, _, _)| *domain == STATE_DOMAIN_WORLD_MAP_VIEW)
            .map(|(_, revision, _)| *revision)
            .ok_or("resumed domain 17")?;
        assert!(map_revision > sent_map_revision);
        assert!(
            !resumed
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        assert!(resumed.deltas.contains(&STATE_DOMAIN_WORLD_MAP_VIEW));
        assert!(!resumed.deltas.contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY));
        Ok(())
    })
}

#[test]
fn a_committed_bundle_door_use_sends_no_domain_2_delta_to_a_session_with_18() -> TestResult {
    run(async {
        // A bundle World's door commit carries no domain 2 entry (MAP-DOOR-1).
        let mut channel = BundleChannel::new();
        channel.use_outcome = UseOutcome {
            disposition: UseDisposition::Committed,
            committed: None,
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
        };
        let sent = serve(
            &channel,
            session(&MAP_VIEW, SessionContinuity::FRESH),
            &[use_door(1, 1)],
        )
        .await;
        assert!(!sent.deltas.contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY));
        assert_eq!(sent.ended().continuity.overlay_revision, 0);
        Ok(())
    })
}

fn door_overlay(revision: u64) -> WorldObjectOverlayEntry {
    WorldObjectOverlayEntry {
        content_generation: [7; 32],
        placement: b"door".to_vec(),
        state: b"open".to_vec(),
        revision,
    }
}

#[test]
fn domain_2_state_offered_to_a_session_with_18_fails_closed_instead_of_being_sent_or_dropped()
-> TestResult {
    run(async {
        // A join whose Channel reports a domain 2 overlay sends no snapshot at all.
        let mut channel = BundleChannel::new();
        channel.overlay = Some(door_overlay(1));
        let sent = serve(&channel, session(&MAP_VIEW, SessionContinuity::FRESH), &[]).await;
        assert!(sent.snapshot.is_empty());
        assert!(sent.deltas.is_empty());
        assert!(matches!(
            sent.end,
            ConnectionEnd::AdmittedThenDisconnected(_)
        ));

        // A committed use carrying a domain 2 entry ends the connection without the delta.
        let mut channel = BundleChannel::new();
        channel.use_outcome = UseOutcome {
            disposition: UseDisposition::Committed,
            committed: Some(door_overlay(1)),
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
        };
        let sent = serve(
            &channel,
            session(&MAP_VIEW, SessionContinuity::FRESH),
            &[use_door(1, 1)],
        )
        .await;
        assert!(!sent.snapshot.is_empty());
        assert!(!sent.deltas.contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY));
        assert_eq!(sent.ended().continuity.overlay_revision, 0);
        Ok(())
    })
}

#[test]
fn a_session_without_18_keeps_domain_2_on_join_resume_and_delta_and_never_gets_17() -> TestResult {
    run(async {
        let legacy = [4, 6];
        let mut channel = BundleChannel::new();
        channel.overlay = Some(door_overlay(1));
        channel.use_outcome = UseOutcome {
            disposition: UseDisposition::Committed,
            committed: Some(door_overlay(2)),
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
        };
        let joined = serve(
            &channel,
            session(&legacy, SessionContinuity::FRESH),
            &[use_door(1, 1)],
        )
        .await;
        assert!(
            joined
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        assert!(
            !joined
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
        assert!(joined.deltas.contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY));
        assert!(!joined.deltas.contains(&STATE_DOMAIN_WORLD_MAP_VIEW));
        let lost = joined.ended();
        assert_eq!(lost.continuity.overlay_revision, 2);

        channel.overlay = Some(door_overlay(2));
        let resumed = serve(
            &channel,
            AdmittedSession {
                continuity: SessionContinuity {
                    connection_generation: 2,
                    ..lost.continuity
                },
                ..lost
            },
            &[],
        )
        .await;
        let overlay = resumed
            .snapshot
            .iter()
            .find(|(domain, _, _)| *domain == STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
            .ok_or("resumed domain 2")?;
        assert_eq!(overlay.1, 2);
        assert!(
            !resumed
                .snapshot_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
        Ok(())
    })
}
