//! GAME-JOIN-DOMAIN-FIX-1: one object domain per World over the served connection (MAP-WIRE-1
//! contract §3, ARCH-MAP-WIRE-1 §1.5). A session that selected capability 18 gets domain 17 and
//! never domain 2 on join, resume or delta; its door states arrive as domain 17 object
//! revisions. A fixture World session, which is not offered 18, keeps domain 2 as before.

use super::*;
use crate::foundation::{
    DomainSnapshot, ExactActorRef, GameSessionId, MessageType, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, ITEM_VIEW_OFFERED_CAPABILITIES, OfferedCapability,
    SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, EarnedNotice, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity, StepOutcome,
    UseCommand, UseOutcome, serve_admitted,
};
use crate::gameplay_transport::item_view::InventoryItems;
use crate::gameplay_transport::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, WorldSpatialObservation,
    encode_step_intent,
};
use oteryn_protocol_oteryn::world_map::WorldMapViewSnapshot;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    WorldObjectOverlayEntry, encode_use_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_snapshot_chunk, decode_state_delta, encode_client_command,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn Error>>;

const DOOR_AT: TilePos = tp(21, 20, -7);
const DOOR_REVISION: u64 = 3;

/// A Channel serving the fixture bundle: domain 17 through the connection's own map view, the
/// own actor stepping south, and the USE outcome and legacy overlay the case injects. A committed
/// USE moves the door's object revision on, as a bundle door's toggle does (MAP-DOOR-1).
struct BundleChannel {
    fixture: RefCell<Fixture>,
    position: Cell<TilePos>,
    overlay: Option<WorldObjectOverlayEntry>,
    use_outcome: UseOutcome,
    doors: Option<(ChannelOverlay, crate::map::facts::BundleFacts)>,
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
            doors: None,
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
        target: WorldObjectTarget,
    ) -> UseOutcome {
        if let Some((overlay, facts)) = &self.doors {
            return crate::gameplay_transport::map_door::use_door(
                overlay,
                facts,
                [7; 32],
                self.position.get(),
                |pos| pos == self.position.get(),
                &target,
            );
        }
        if self.use_outcome.disposition == UseDisposition::Committed {
            self.fixture
                .borrow_mut()
                .facts
                .revisions
                .insert(key(DOOR_AT, 1), DOOR_REVISION + 1);
        }
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
        if let Some((overlay, facts)) = &self.doors {
            let source = MapViewSource {
                overlay,
                facts,
                content_generation: [7; 32],
                reset_epoch: 0,
            };
            return Some(view.update(items, &source, at(self.position.get())));
        }
        let fixture = self.fixture.borrow();
        Some(view.update(items, &fixture.source(), at(self.position.get())))
    }
}

/// A session of a World offering `offered`, whose client supports `supported`.
fn session(
    offered: &[OfferedCapability],
    supported: &[u32],
    continuity: SessionContinuity,
) -> AdmittedSession {
    AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid(3)).expect("session"),
        world_id: world(),
        channel_id: channel(),
        runtime_actor: Some(ExactActorRef::transport_fixture(world(), channel())),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(offered, supported)
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

/// What one served connection sent: each snapshot domain with its revision and payload, and
/// each delta's domain with its payload, in order.
struct Sent {
    end: ConnectionEnd,
    snapshot: Vec<(u32, u64, Vec<u8>)>,
    deltas: Vec<(u32, Vec<u8>)>,
    results: Vec<Vec<u8>>,
}

impl Sent {
    fn snapshot_domains(&self) -> Vec<u32> {
        self.snapshot.iter().map(|(domain, _, _)| *domain).collect()
    }

    fn delta_domains(&self) -> Vec<u32> {
        self.deltas.iter().map(|(domain, _)| *domain).collect()
    }

    fn ended(self) -> AdmittedSession {
        match self.end {
            ConnectionEnd::AdmittedThenDisconnected(admitted) => admitted,
            other => panic!("unexpected end {other:?}"),
        }
    }
}

/// Both futures to completion, polled in turn on one task (the crate's tokio has no `join!`).
async fn join<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
    use std::task::Poll;
    let (mut a, mut b) = (std::pin::pin!(a), std::pin::pin!(b));
    let (mut left, mut right) = (None, None);
    std::future::poll_fn(|context| {
        if left.is_none()
            && let Poll::Ready(output) = a.as_mut().poll(context)
        {
            left = Some(output);
        }
        if right.is_none()
            && let Poll::Ready(output) = b.as_mut().poll(context)
        {
            right = Some(output);
        }
        match (left.take(), right.take()) {
            (Some(l), Some(r)) => Poll::Ready((l, r)),
            (l, r) => {
                (left, right) = (l, r);
                Poll::Pending
            }
        }
    })
    .await
}

/// Serves one connection whose client writes `client_frames` and holds the connection open for
/// `hold` before it closes.
async fn serve(
    channel: &BundleChannel,
    admitted: AdmittedSession,
    client_frames: &[Vec<u8>],
    hold: Duration,
) -> Sent {
    let (mut server, mut client) = tokio::io::duplex(1 << 22);
    let served = async move {
        let end = serve_admitted(&mut server, admitted, channel, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let client = async move {
        for frame in client_frames {
            client.write_all(frame).await.expect("write");
        }
        tokio::time::sleep(hold).await;
        client.shutdown().await.expect("shutdown");
        let mut output = Vec::new();
        client.read_to_end(&mut output).await.expect("read");
        output
    };
    let (end, output) = join(served, client).await;
    let mut results = Vec::new();
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
                let delta = decode_state_delta(envelope.payload()).expect("delta");
                deltas.push((delta.domain_id, delta.payload.to_vec()));
            }
            MessageType::CommandResult => {
                results.push(
                    oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .expect("command result")
                        .payload
                        .to_vec(),
                );
            }
            _ => {}
        }
    }
    Sent {
        end,
        snapshot,
        deltas,
        results,
    }
}

fn run(test: impl std::future::Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(test)
}

/// The selection of a client that supports 4, 6 and 18: on a bundle World it selects 18.
const MAP_VIEW: [u32; 3] = [4, 6, 18];

/// No hold: the client closes as soon as its frames are written.
const NO_HOLD: Duration = Duration::ZERO;

/// Longer than two of the connection's 250 ms visibility refreshes, which send domain 17 tile
/// changes made by others or by the actor's own `USE` (MAP-WIRE-2).
const PAST_A_REFRESH: Duration = Duration::from_millis(600);

fn bundle_session() -> AdmittedSession {
    session(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &MAP_VIEW,
        SessionContinuity::FRESH,
    )
}

fn map_snapshot(sent: &Sent) -> Result<WorldMapViewSnapshot, Box<dyn Error>> {
    let (_, _, map) = sent
        .snapshot
        .iter()
        .find(|(domain, _, _)| *domain == STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("domain 17")?;
    Ok(decode_world_map_snapshot(map).map_err(|error| format!("{error:?}"))?)
}

fn door_origin(revision: u64) -> MapOrigin {
    MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: revision,
    }
}

#[test]
fn a_session_with_18_joins_steps_and_resumes_with_domain_17_and_never_domain_2() -> TestResult {
    run(async {
        let channel = BundleChannel::new();
        let joined = serve(&channel, bundle_session(), &[step(1, 1)], NO_HOLD).await;
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
        assert!(
            joined
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
        assert!(
            !joined
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        // The door's state is carried by domain 17: its base entry with the overlay revision.
        let map = map_snapshot(&joined)?;
        let door = map
            .tiles
            .iter()
            .find(|tile| tile.position == at(DOOR_AT))
            .ok_or("door tile")?;
        assert_eq!(door.items[1].origin, door_origin(DOOR_REVISION));

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
            NO_HOLD,
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
        assert!(
            resumed
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
        assert!(
            !resumed
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        Ok(())
    })
}

#[test]
fn a_committed_bundle_door_use_is_a_domain_17_change_and_no_domain_2_delta_for_a_session_with_18()
-> TestResult {
    run(async {
        // A bundle World's door commit carries no domain 2 entry (MAP-DOOR-1); the toggle moves
        // the door's object revision on.
        let mut channel = BundleChannel::new();
        channel.use_outcome = UseOutcome {
            disposition: UseDisposition::Committed,
            committed: None,
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
        };
        let sent = serve(
            &channel,
            bundle_session(),
            &[use_door(1, 1)],
            PAST_A_REFRESH,
        )
        .await;
        assert!(
            !sent
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        // The USE is followed by a domain 17 delta carrying the door's new object revision.
        let header = map_snapshot(&sent)?.header;
        let door = sent
            .deltas
            .iter()
            .filter(|(domain, _)| *domain == STATE_DOMAIN_WORLD_MAP_VIEW)
            .map(|(_, payload)| decode_world_map_delta(&header, payload))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("{error:?}"))?
            .into_iter()
            .flat_map(|delta| delta.tiles)
            .find(|tile| tile.position == at(DOOR_AT))
            .ok_or("no domain 17 delta for the door after the USE")?;
        assert_eq!(door.items[1].origin, door_origin(DOOR_REVISION + 1));
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
        let sent = serve(&channel, bundle_session(), &[], NO_HOLD).await;
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
        let sent = serve(&channel, bundle_session(), &[use_door(1, 1)], NO_HOLD).await;
        assert!(!sent.snapshot.is_empty());
        assert!(
            !sent
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        assert_eq!(sent.ended().continuity.overlay_revision, 0);
        Ok(())
    })
}

#[test]
fn a_fixture_world_session_keeps_domain_2_on_join_resume_and_delta_and_never_gets_17() -> TestResult
{
    run(async {
        // A fixture World does not offer 18, so a client supporting it selects 4 and 6 only.
        let fixture_world =
            |continuity| session(ITEM_VIEW_OFFERED_CAPABILITIES, &MAP_VIEW, continuity);
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
            fixture_world(SessionContinuity::FRESH),
            &[use_door(1, 1)],
            NO_HOLD,
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
        assert!(
            joined
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
        );
        assert!(
            !joined
                .delta_domains()
                .contains(&STATE_DOMAIN_WORLD_MAP_VIEW)
        );
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
            NO_HOLD,
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

// PARITY-DOOR-PREDICATES-1: generated fixture bundle -> native facts -> real USE
// routing -> map target resolver -> DoorBook -> negotiated command result.
fn key_door_channel(appearance: u32, attrs: Attrs, house: u32) -> BundleChannel {
    use crate::map::boot::tests::bundle_with_items;
    use crate::map::facts::{BundleFacts, ItemDefinition};
    let closed = format!("oteryn:item.tibia.i{appearance}");
    let (bytes, mut pins) = bundle_with_items(&closed, "oteryn:item.tibia.i1000");
    let mut bundle = bundle::read(&bytes).expect("bundle");
    let tile = bundle.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 6)
        .expect("door");
    tile.items[1].attrs = attrs;
    tile.house = house;
    let bytes = bundle::write(&bundle.manifest, &bundle.sectors, &Default::default())
        .expect("generated fixture");
    pins.digest = bundle::read(&bytes).expect("bundle digest").digest;
    let base = Arc::new(crate::map::load(&bytes, &pins).expect("base"));
    let mut facts = BundleFacts::palette(&bundle.manifest, |key| {
        let id: u32 = key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
        Some(ItemDefinition {
            reference: nz(if id == appearance {
                1
            } else if id == 1000 {
                2
            } else {
                id + 1
            }),
            solid: Some(!matches!(id, 1630 | 1639 | 20445 | 4911 | 1000)),
            blocks_projectile: !matches!(id, 1630 | 1639 | 20445 | 4911 | 1000),
            pickupable: id == 1000,
            door: id != 1000,
        })
    })
    .expect("palette facts");
    for sector in &bundle.sectors {
        facts.push(&bundle.manifest, sector, &mut BTreeSet::new());
    }
    facts.finish();
    let mut owner = BundleChannel::new();
    owner.position.set(tp(7, 0, -7));
    owner.doors = Some((ChannelOverlay::new(base, world(), channel()), facts));
    owner
}

#[test]
fn key_door_predicates_through_use_dispatch() -> TestResult {
    use oteryn_protocol_oteryn::world_object::decode_use_result;
    run(async {
        for (appearance, action, expected) in [
            (1629, None, UseDisposition::Committed),
            (1629, Some(4321), UseDisposition::Committed),
            (1628, Some(4321), UseDisposition::Locked),
            (1628, None, UseDisposition::Locked),
            (1629, Some(101), UseDisposition::Locked),
            (1629, Some(1001), UseDisposition::Locked),
            (20444, Some(4321), UseDisposition::Locked),
            (20443, Some(4321), UseDisposition::Committed),
            (5007, Some(4321), UseDisposition::Committed),
        ] {
            let channel = key_door_channel(
                appearance,
                Attrs {
                    action,
                    ..Attrs::default()
                },
                0,
            );
            let (overlay, facts) = channel.doors.as_ref().expect("doors");
            let key = crate::map::boot::tests::door_key();
            let mut placement = overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            // An authorized second direct USE closes the unlocked door. Replaying the old
            // target is stale; locked refusals never consume the object revision.
            let revisions: &[u64] = if expected == UseDisposition::Committed {
                &[0, 0, 1]
            } else {
                &[0]
            };
            let frames: Vec<_> = revisions
                .iter()
                .enumerate()
                .map(|(i, revision)| {
                    let payload = encode_use_intent(&WorldObjectTarget {
                        placement: placement.clone(),
                        expected_revision: *revision,
                    })
                    .expect("use");
                    command(1, i as u64 + 1, COMMAND_TYPE_USE_INTENT, &payload)
                })
                .collect();
            let admitted = session(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6, 11, 15, 18],
                SessionContinuity::FRESH,
            );
            let sent = serve(&channel, admitted, &frames, NO_HOLD).await;
            let results: Vec<_> = sent
                .results
                .iter()
                .map(|payload| decode_use_result(payload).expect("use result"))
                .collect();
            let expected_results = if expected == UseDisposition::Committed {
                vec![expected, UseDisposition::StaleState, expected]
            } else {
                vec![expected]
            };
            assert_eq!(results, expected_results, "{appearance} {action:?}");
            assert_eq!(
                facts.doors().revision(key),
                if expected == UseDisposition::Committed {
                    2
                } else {
                    0
                }
            );
            assert!(facts.doors().blocks(tp(6, 0, -7)));
            assert!(
                !sent
                    .delta_domains()
                    .contains(&STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
            );
        }
        Ok(())
    })
}

#[test]
fn key_door_predicates_preserve_bindings_reach_and_capabilities() -> TestResult {
    use oteryn_protocol_oteryn::world_object::decode_use_result;
    run(async {
        for (appearance, attrs, house, position, caps, expected) in [
            (
                1629,
                Attrs {
                    action: Some(4321),
                    unique: Some(7),
                    ..Attrs::default()
                },
                0,
                tp(7, 0, -7),
                vec![4, 6, 11, 15, 18],
                UseDisposition::NothingToUse,
            ),
            (
                1629,
                Attrs {
                    action: Some(4321),
                    door: Some(1),
                    ..Attrs::default()
                },
                7,
                tp(7, 0, -7),
                vec![4, 6, 11, 15, 18],
                UseDisposition::NothingToUse,
            ),
            (
                1629,
                Attrs {
                    action: Some(4321),
                    ..Attrs::default()
                },
                7,
                tp(7, 0, -7),
                vec![4, 6, 11, 15, 18],
                UseDisposition::NothingToUse,
            ),
            (
                1628,
                Attrs {
                    action: Some(4321),
                    ..Attrs::default()
                },
                0,
                tp(9, 0, -7),
                vec![4, 6, 11, 15, 18],
                UseDisposition::TooFar,
            ),
            (
                1628,
                Attrs::default(),
                0,
                tp(7, 0, -7),
                vec![4, 6, 15, 18],
                UseDisposition::Rejected,
            ),
            (
                1638,
                Attrs {
                    action: Some(4321),
                    ..Attrs::default()
                },
                0,
                tp(7, 0, -7),
                vec![4, 6, 11, 15, 18],
                UseDisposition::NothingToUse,
            ),
        ] {
            let house_door = attrs.door.map(u32::from);
            let channel = key_door_channel(appearance, attrs, house);
            channel.position.set(position);
            let (overlay, facts) = channel.doors.as_ref().expect("doors");
            let key = crate::map::boot::tests::door_key();
            let mut placement = overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let payload = encode_use_intent(&WorldObjectTarget {
                placement,
                expected_revision: 0,
            })
            .expect("use");
            let admitted = session(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &caps,
                SessionContinuity::FRESH,
            );
            let sent = serve(
                &channel,
                admitted,
                &[command(1, 1, COMMAND_TYPE_USE_INTENT, &payload)],
                NO_HOLD,
            )
            .await;
            assert_eq!(sent.results.len(), 1);
            assert_eq!(
                decode_use_result(&sent.results[0]).expect("use result"),
                expected
            );
            assert_eq!(facts.doors().revision(key), 0);
            if house != 0 {
                assert!(facts.house_tile(tp(6, 0, -7)));
                assert_eq!(facts.house_door(tp(6, 0, -7), house), house_door);
                assert!(!facts.doors().contains(key));
            }
        }
        Ok(())
    })
}
