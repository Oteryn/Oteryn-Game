mod appearance_change_tests;
use super::*;
use crate::gameplay_transport::world_spatial::{
    ActorPosition, ChannelEntity, EntityDetail, EntityKind, EntityRef, PLACEHOLDER_ACTOR_DIRECTION,
    PLACEHOLDER_APPEARANCE_REF, PLACEHOLDER_HEALTH_PERCENT, WorldSpatialEntitiesDelta,
    WorldSpatialEntitiesSnapshot, encode_step_intent, encode_step_result, encode_visibility_delta,
    encode_visibility_snapshot,
};
use oteryn_protocol_oteryn::{ClientCommandValue, decode_snapshot_chunk, encode_client_command};
use std::cell::{Cell, RefCell};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

type TestResult = Result<(), Box<dyn Error>>;

const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

const fn at(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 7 }
}

const CONTENT: [u8; 32] = [0x5c; 32];

fn creature(identity: [u8; 16], position: ActorPosition, revision: u64) -> ChannelEntity {
    ChannelEntity {
        source_icons: oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcons::default(
        ),
        motion: None,
        display_name: None,
        health_percent: 100,
        direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
        kind: VisibleKind::Creature,
        identity,
        generation: 1,
        position,
        revision,
    }
}

fn actor() -> ExactActorRef {
    ExactActorRef::transport_fixture(
        WorldId::decode(&uuid_v7(0x33)).expect("world"),
        ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
    )
}

/// The wire entry of an actor of the fixture Channel.
fn wire(kind: EntityKind, identity: [u8; 16], position: ActorPosition) -> WorldSpatialEntity {
    WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind,
        entity: EntityRef {
            identity,
            generation: 1,
        },
        position,
        detail: EntityDetail::Actor {
            direction: PLACEHOLDER_ACTOR_DIRECTION,
            appearance_ref: PLACEHOLDER_APPEARANCE_REF,
            health_percent: PLACEHOLDER_HEALTH_PERCENT,
        },
    }
}

fn own(position: ActorPosition) -> WorldSpatialEntity {
    wire(EntityKind::Player, actor().placement_identity(), position)
}

#[test]
fn corpse_stack_resync_publishes_a_domain_one_replacement() -> TestResult {
    let mut snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT,
        actor_position: at(0, 0),
        own_identity: actor().placement_identity(),
        entities: vec![own(at(0, 0))],
    };
    for marker in [9, 3] {
        snapshot.entities.push(WorldSpatialEntity {
            kind: EntityKind::Corpse,
            entity: EntityRef {
                identity: [marker; 16],
                generation: 0,
            },
            detail: EntityDetail::Object {
                item_definition_ref: 42,
                quantity: 1,
                item_handle: None,
            },
            ..own(at(1, 0))
        });
    }
    let (frames, sequence, revision) = visibility_frames(
        1,
        41,
        5,
        &[6],
        &EntityLights::new(),
        &VisibilityUpdate::StackSnapshot(snapshot.clone()),
    )
    .ok_or("missing replacement frames")?;
    assert_eq!((sequence, revision), (42, 6));
    let mut frames = frames.iter();
    let first = decode_wire_envelope(frames.next().ok_or("missing SnapshotBegin")?)?;
    assert_eq!(first.message_type(), MessageType::SnapshotBegin);
    let begin = oteryn_protocol_oteryn::decode_snapshot_begin(first.payload())?;
    assert_eq!(begin.target_server_sequence, 42);
    let second = decode_wire_envelope(frames.next().ok_or("missing SnapshotChunk")?)?;
    assert_eq!(second.message_type(), MessageType::SnapshotChunk);
    let (snapshot_id, domains) = decode_snapshot_chunk(second.payload())?;
    assert_eq!(snapshot_id, begin.snapshot_id);
    let [domain] = domains.as_slice() else {
        return Err("expected one replacement domain".into());
    };
    assert_eq!(
        (domain.domain_id, domain.revision, domain.snapshot_type),
        (1, 6, 2)
    );
    let replacement =
        oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot(
            domain.payload,
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(replacement, snapshot);
    Ok(())
}

/// The Channel owner as domain 1 sees it: the own actor, which steps east, and the others.
struct Channel {
    own: Cell<ActorPosition>,
    others: RefCell<Vec<ChannelEntity>>,
}

impl Channel {
    fn new(others: Vec<ChannelEntity>) -> Self {
        Self {
            own: Cell::new(at(100, 100)),
            others: RefCell::new(others),
        }
    }

    fn observation(&self) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: CONTENT,
            actor_position: self.own.get(),
        }
    }
}

impl FreshAdmissionAuthority for Channel {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(self.observation())
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        let mut position = self.own.get();
        position.x += 1;
        self.own.set(position);
        StepOutcome {
            disposition: StepDisposition::Moved,
            moved_to: Some(self.observation()),
        }
    }

    async fn player_status(
        &self,
        _: AdmittedSession,
        _: EntityRef,
    ) -> Option<oteryn_protocol_oteryn::zones_nologout::PlayerStatus> {
        Some(oteryn_protocol_oteryn::zones_nologout::PlayerStatus {
            presence: oteryn_protocol_oteryn::zones_nologout::PlayerPresence::Away,
            no_logout: true,
        })
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let own = actor.placement_identity();
        let mut entities = vec![ChannelEntity {
            source_icons:
                oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcons::default(),
            motion: None,
            display_name: None,
            health_percent: 100,
            direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
            kind: VisibleKind::Player,
            identity: own,
            generation: 1,
            position: self.own.get(),
            revision: u64::try_from(self.own.get().x).ok()?,
        }];
        entities.extend(self.others.borrow().iter().copied());
        Some(ChannelEntities {
            content_generation: CONTENT,
            observer: own,
            entities,
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
    }
}

fn session(selected: &[u32], continuity: SessionContinuity) -> AdmittedSession {
    AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22)).expect("session"),
        world_id: WorldId::decode(&uuid_v7(0x33)).expect("world"),
        channel_id: ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
        runtime_actor: Some(actor()),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                PRODUCTION_OFFERED_CAPABILITIES,
                selected,
            )
            .expect("select"),
            ..continuity
        },
        item_fence: None,
    }
}

fn snapshot_frames(
    generation: u64,
    snapshot_id: u64,
    target: u64,
    revision: u64,
    selected: &[u32],
    entities: Vec<WorldSpatialEntity>,
) -> Vec<Vec<u8>> {
    let snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT,
        actor_position: entities[0].position,
        own_identity: entities[0].entity.identity,
        entities,
    };
    let (snapshot_type, payload) =
        encode_visibility_snapshot(selected, &snapshot).expect("snapshot");
    encode_single_chunk_snapshot(
        generation,
        snapshot_id,
        target,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                snapshot_type,
                payload: &payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &[],
            },
        ],
    )
    .expect("frames")
    .to_vec()
}

fn delta_frame(
    sequence: u64,
    from: u64,
    selected: &[u32],
    delta: &WorldSpatialEntitiesDelta,
) -> Vec<u8> {
    let (delta_type, payload) = encode_visibility_delta(selected, delta).expect("delta");
    encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        from,
        from + 1,
        delta_type,
        &payload,
    )
    .expect("frame")
}

async fn read_one(client: &mut DuplexStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut length = [0_u8; 4];
    client.read_exact(&mut length).await?;
    let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

/// One step, then the client closes: every frame the connection sent.
async fn join_and_step(selected: &[u32]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: 1,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &encode_step_intent(StepDirection::East),
        },
    )?;
    let mut framed = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    framed.extend_from_slice(&frame);
    client.write_all(&framed).await?;
    client.shutdown().await?;
    serve_admitted(
        &mut server,
        session(selected, SessionContinuity::FRESH),
        &channel,
        IDLE_LIVENESS,
    )
    .await;
    drop(server);
    let mut frames = Vec::new();
    while let Ok(frame) = read_one(&mut client).await {
        frames.push(frame);
    }
    Ok(frames)
}

// The scenario future is large; keep it on the heap so the default test stack holds it.
fn run(test: impl Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(Box::pin(test))
}

include!("visibility_tests/interest_set.rs");

#[test]
fn player_status_dispatch_requires_negotiation_and_visible_incarnation()
-> Result<(), Box<dyn Error + Send + Sync>> {
    use oteryn_protocol_oteryn::{MessageType, decode_command_result};
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let mut target = creature([9; 16], at(102, 100), 0);
                target.kind = VisibleKind::Player;
                let channel = Channel::new(vec![target]);
                for (capabilities, incarnation, status, expected) in [
                    (&[6][..], 1_u64, CommandStatus::Rejected, &[][..]),
                    (&[6, 600][..], 2, CommandStatus::Rejected, &[][..]),
                    (&[6, 600][..], 1, CommandStatus::Accepted, &[2, 1][..]),
                ] {
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    let mut admitted = session(&[6], SessionContinuity::FRESH);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        capabilities,
                    )
                    .ok_or("selected capabilities")?;
                    let exercise = async {
                        let mut payload = target.identity.to_vec();
                        payload.extend_from_slice(&incarnation.to_be_bytes());
                        let frame = encode_client_command(
                            1,
                            &ClientCommandValue {
                                command_id: 1,
                                command_type: 6363,
                                payload: &payload,
                            },
                        )?;
                        client
                            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
                            .await?;
                        client.write_all(&frame).await?;
                        loop {
                            let frame = read_one(&mut client).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            if envelope.message_type() == MessageType::CommandResult {
                                let result = decode_command_result(envelope.payload())?;
                                assert_eq!((result.status, result.payload), (status, expected));
                                break;
                            }
                        }
                        client.shutdown().await?;
                        Ok::<(), Box<dyn Error>>(())
                    };
                    let (_, result) = tokio::join!(
                        serve_admitted(&mut server, admitted, &channel, IDLE_LIVENESS),
                        exercise
                    );
                    result?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "player status dispatch thread")??;
    Ok(())
}

/// Keep the published cut fixed across a visibility change while using the real command route.
pub(in crate::gameplay_transport) async fn regression_player_status(
    authority: &impl FreshAdmissionAuthority,
    admitted: AdmittedSession,
    target: EntityRef,
    visibility: &SessionVisibility,
) -> Option<Vec<u8>> {
    let mut payload = target.identity.to_vec();
    payload.extend_from_slice(&target.generation.to_be_bytes());
    match routing::player_status::route(authority, admitted, &payload, Some(visibility)).await {
        Dispatch::Reply(CommandStatus::Accepted, payload) => Some(payload),
        _ => None,
    }
}

pub(in crate::gameplay_transport) fn prime_player_status_clock(
    authority: &impl FreshAdmissionAuthority,
    mut admitted: AdmittedSession,
) {
    admitted
        .continuity
        .gameplay_idle
        .activity(authority.gameplay_idle_now());
    authority.observe_gameplay_idle(admitted);
}
