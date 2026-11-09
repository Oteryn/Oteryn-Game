//! Visibility completion must leave room for input after a slow browse owner read.

use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_view::{InventoryItems, ItemViewSnapshotDomain, ViewItem};
use crate::gameplay_transport::world_map::{MapUpdate, MapViewError};
use crate::gameplay_transport::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::browse_field::{
    BrowseFieldIntent, COMMAND_TYPE_BROWSE_FIELD_INTENT, encode_browse_field_intent,
};
use oteryn_protocol_oteryn::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewIntent, encode_container_view_intent,
};
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, WorldTilePosition};
use oteryn_protocol_oteryn::world_map::{
    MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
};
use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
use std::cell::Cell;
use tokio::io::DuplexStream;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const FIELD: WorldTilePosition = WorldTilePosition {
    x: 300,
    y: 300,
    floor: -7,
};

fn observation() -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: [7; 32],
        actor_position: ActorPosition {
            x: FIELD.x,
            y: FIELD.y,
            floor: 7,
        },
    }
}

#[derive(Default)]
struct SlowOwner {
    idle_cue: Cell<bool>,
    slow_browse: Cell<bool>,
    slow_visibility: Cell<bool>,
    browse_reads: Cell<usize>,
    sound_reads: Cell<usize>,
    step_calls: Cell<usize>,
    last_step: Cell<Option<StepDirection>>,
    pending_successor: Cell<Option<AdmittedSession>>,
    switch_checks: Cell<usize>,
}

impl FreshAdmissionAuthority for SlowOwner {
    async fn observe_current_actor_movement_effects(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        after: u64,
    ) -> Vec<(u64, crate::map::overlay::TilePos, u16)> {
        let Ok(current) = admitted(false) else {
            return Vec::new();
        };
        if current.runtime_actor != Some(actor) || current.game_session_id != session {
            return Vec::new();
        }
        self.observe_movement_effects(after)
    }
    fn observe_movement_effects(
        &self,
        after: u64,
    ) -> Vec<(u64, crate::map::overlay::TilePos, u16)> {
        if self.idle_cue.get() && after == 0 {
            vec![(
                1,
                crate::map::overlay::TilePos {
                    x: 300,
                    y: 300,
                    floor: -7,
                },
                11,
            )]
        } else {
            Vec::new()
        }
    }
    async fn step(&self, _actor: ExactActorRef, direction: StepDirection) -> StepOutcome {
        self.step_calls.set(self.step_calls.get() + 1);
        self.last_step.set(Some(direction));
        StepOutcome::rejected()
    }

    async fn pending_activity_switch(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<crate::gameplay_transport::activity_runtime::ActivityConnectionSwitch>, ()>
    {
        let Some(destination) = self.pending_successor.get() else {
            return Ok(None);
        };
        if admitted.game_session_id == destination.game_session_id {
            return Ok(None);
        }
        let checks = self.switch_checks.get();
        self.switch_checks.set(checks + 1);
        if checks != 1 {
            return Ok(None);
        }
        self.pending_successor.set(None);
        use oteryn_protocol_oteryn::activity_scope::{
            ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
        };
        let payload = encode_activity_scope(&ActivityScopeSnapshot {
            handoff: uuid_v7(0x67),
            source_session: *admitted.game_session_id.as_bytes(),
            session: *destination.game_session_id.as_bytes(),
            scope: ActivityScope::Channel {
                world: *destination.world_id.as_bytes(),
                channel: *destination.channel_id.as_bytes(),
            },
            scope_generation: 1,
            connection_generation: 1,
            lease_generation: 1,
            // This authority-double transport fixture resumes with a bound
            // command1; acknowledged production handoffs have separate tests.
            requires_ack: false,
            selection: destination
                .continuity
                .selected_capabilities
                .as_slice()
                .to_vec(),
        })
        .map_err(|_| ())?;
        Ok(Some(
            crate::gameplay_transport::activity_runtime::ActivityConnectionSwitch {
                movement_effect_start: None,
                source: admitted,
                destination,
                payload,
            },
        ))
    }

    async fn observe_sounds(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _after: u64,
    ) -> Option<(
        u64,
        Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>,
    )> {
        self.sound_reads.set(self.sound_reads.get() + 1);
        Some((1, Vec::new()))
    }

    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        if self.slow_visibility.get() {
            tokio::time::sleep(VISIBILITY_REFRESH * 2).await;
        }
        Some(observation())
    }

    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems::default())
    }

    async fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<Vec<(EquipmentSlot, ViewItem)>> {
        Some(Vec::new())
    }

    async fn observe_container(
        &self,
        _actor: ExactActorRef,
        _fence: Option<CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<ContainerObservation> {
        assert_eq!(
            target,
            ItemKey::BrowseField {
                x: FIELD.x,
                y: FIELD.y,
                floor: FIELD.floor,
            }
        );
        self.browse_reads.set(self.browse_reads.get() + 1);
        if self.slow_browse.get() {
            // A valid owner read longer than the existing cadence reproduces the starvation.
            tokio::time::sleep(VISIBILITY_REFRESH * 2).await;
        }
        Some(ContainerObservation::BrowseField {
            position: FIELD,
            entries: Vec::new(),
            lease: None,
        })
    }

    async fn observe_world_map(
        &self,
        _actor: ExactActorRef,
        view: &mut SessionMapView,
        _items: &mut SessionItemView,
    ) -> Option<Result<Option<MapUpdate>, MapViewError>> {
        if view.revision() != 0 {
            return Some(Ok(None));
        }
        // This fixed map owner serves a valid empty snapshot, then no changes. The test
        // exercises the real admission setup with all browse capability dependencies.
        let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [7; 32],
                bundle_digest: [8; 32],
                reset_epoch: 0,
                origin: ActorPosition {
                    x: FIELD.x,
                    y: FIELD.y,
                    floor: FIELD.floor,
                },
                first_visible_floor: -7,
            },
            tiles: Vec::new(),
        })
        .ok()?;
        *view = SessionMapView::starting_after(1);
        Some(Ok(Some(MapUpdate::Snapshot(ItemViewSnapshotDomain {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
            revision: 1,
            snapshot_type: 1,
            payload,
        }))))
    }
}

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

fn admitted(browse: bool) -> TestResult<AdmittedSession> {
    let world_id = WorldId::decode(&uuid_v7(0x33))?;
    let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
    let supported: &[u32] = if browse {
        &[4, 6, 12, 14, 18, 230]
    } else {
        &[6]
    };
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                supported,
            )
            .ok_or("select fixture capabilities")?,
            ..SessionContinuity::FRESH
        },
        item_fence: None,
    })
}

fn command_frame(id: u64, command_type: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
    Ok(encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type,
            payload,
        },
    )?)
}

async fn execute_ready_frame(
    stream: &mut DuplexStream,
    authority: &SlowOwner,
    session: &mut ServeState,
) -> TestResult {
    let next = poll::next(stream, authority, session).await;
    assert!(
        matches!(next, Next::Frame(Ok(_))),
        "the ready client command must run before a second visibility refresh"
    );
    let input = receive(stream, authority, session, next)
        .await
        .map_err(|_| "receive ready frame")?
        .ok_or("ready frame did not reach dispatch")?;
    command::execute(stream, authority, session, input)
        .await
        .map_err(|_| "dispatch ready frame")?;
    Ok(())
}

async fn scenario(browse: bool) -> TestResult {
    let authority = SlowOwner::default();
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let mut admitted = admitted(browse)?;
    let actor = admitted.runtime_actor.ok_or("fixture actor")?;
    let mut session = setup::prepare(
        &mut server,
        &mut admitted,
        &authority,
        IDLE_LIVENESS,
        actor,
        observation(),
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "prepare connection")?;
    refresh::sounds(&mut server, &authority, &mut session)
        .await
        .map_err(|_| "sound refresh")?;
    assert_eq!(
        authority.sound_reads.get(),
        0,
        "clients without capability 23 never read the sound owner"
    );
    let mut command_id = 1;
    if browse {
        let payload = encode_browse_field_intent(&BrowseFieldIntent::Open { position: FIELD })
            .map_err(|error| format!("{error:?}"))?;
        write_frame(
            &mut client,
            &command_frame(command_id, COMMAND_TYPE_BROWSE_FIELD_INTENT, &payload)?,
        )
        .await?;
        execute_ready_frame(&mut server, &authority, &mut session).await?;
        assert_eq!(
            session.container_views.as_ref().map(|v| v.open_views()),
            Some(1)
        );
        command_id += 1;
    }
    let first = poll::next(&mut server, &authority, &mut session).await;
    assert!(matches!(first, Next::Visibility));
    let close = encode_container_view_intent(&ContainerViewIntent::Close { view_id: 15 })
        .map_err(|error| format!("{error:?}"))?;
    write_frame(
        &mut client,
        &command_frame(command_id, COMMAND_TYPE_CONTAINER_VIEW_INTENT, &close)?,
    )
    .await?;
    authority.browse_reads.set(0);
    authority.slow_browse.set(browse);
    authority.slow_visibility.set(!browse);
    let started = tokio::time::Instant::now();
    assert!(
        receive(&mut server, &authority, &mut session, first)
            .await
            .map_err(|_| "slow visibility refresh")?
            .is_none()
    );
    let finished = tokio::time::Instant::now();
    assert!(finished - started >= VISIBILITY_REFRESH * 2);
    if browse {
        assert_eq!(authority.browse_reads.get(), 1);
        execute_ready_frame(&mut server, &authority, &mut session).await?;
        assert_eq!(
            session.container_views.as_ref().map(|v| v.open_views()),
            Some(0)
        );
        assert_eq!(session.admitted.continuity.next_command_id, command_id + 1);
        assert_eq!(authority.browse_reads.get(), 1);
        assert!(matches!(
            poll::next(&mut server, &authority, &mut session).await,
            Next::Visibility
        ));
        assert!(tokio::time::Instant::now() - finished >= VISIBILITY_REFRESH);
    } else {
        // Sessions without capability 230 retain their pre-existing scheduling semantics.
        assert!(matches!(
            poll::next(&mut server, &authority, &mut session).await,
            Next::Visibility
        ));
        assert_eq!(tokio::time::Instant::now(), finished);
    }
    Ok(())
}

fn run(browse: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(scenario(browse))
        })?
        .join()
        .map_err(|_| "visibility scheduler test thread panicked")?
}

#[test]
fn browse_slow_visibility_refresh_dispatches_ready_client_before_next_refresh() -> TestResult {
    run(true)
}

#[test]
fn legacy_slow_visibility_refresh_keeps_existing_cadence() -> TestResult {
    run(false)
}

#[test]
fn activity_successor_rejects_already_queued_old_move_before_counter() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> TestResult {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = SlowOwner::default();
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    let old = admitted(false)?;
                    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                        StepDirection::East,
                    );
                    // The non-initiator already sent this source-session command before
                    // another participant's transfer installed the new session.
                    let old_frame = command_frame(1, 1, &payload)?;
                    write_frame(&mut client, &old_frame).await?;
                    let mut successor = old;
                    successor.game_session_id = GameSessionId::decode(&uuid_v7(0x23))?;
                    let actor = successor.runtime_actor.ok_or("actor")?;
                    let mut session = setup::prepare(
                        &mut server,
                        &mut successor,
                        &authority,
                        IDLE_LIVENESS,
                        actor,
                        observation(),
                        integrity::Integrity::default(),
                    )
                    .await
                    .map_err(|_| "successor setup")?;
                    session.require_session_binding = true;
                    assert!(
                        !receive_ready_session_frame(&mut server, &authority, &mut session).await?,
                        "late old frame must be filtered before any command/replacement peek"
                    );
                    assert_eq!(
                        session.commands.next_command_id(),
                        1,
                        "old bytes must not consume successor command1"
                    );
                    assert_eq!(
                        authority.step_calls.get(),
                        0,
                        "old bytes must not reach the destination owner"
                    );
                    let value = ClientCommandValue {
                        command_id: 1,
                        command_type: 1,
                        payload: &payload,
                    };
                    let stale_bound = oteryn_protocol_oteryn::encode_client_command_for_session(
                        1,
                        &value,
                        old.game_session_id.as_bytes(),
                    )?;
                    write_frame(&mut client, &stale_bound).await?;
                    assert!(
                        !receive_ready_session_frame(&mut server, &authority, &mut session).await?,
                        "a command explicitly bound to the source session is equally stale"
                    );
                    assert_eq!(session.commands.next_command_id(), 1);
                    assert_eq!(authority.step_calls.get(), 0);
                    // Direct dispatch must independently reject a previously held frame.
                    command::execute(
                        &mut server,
                        &authority,
                        &mut session,
                        Input {
                            frame: old_frame,
                            arrived_while_buffered: false,
                            buffered_original: false,
                        },
                    )
                    .await
                    .map_err(|_| "direct stale dispatch")?;
                    assert_eq!(session.commands.next_command_id(), 1);
                    assert_eq!(authority.step_calls.get(), 0);
                    let current = oteryn_protocol_oteryn::encode_client_command_for_session(
                        1,
                        &value,
                        successor.game_session_id.as_bytes(),
                    )?;
                    write_frame(&mut client, &current).await?;
                    assert!(
                        receive_ready_session_frame(&mut server, &authority, &mut session).await?
                    );
                    assert_eq!(
                        session.commands.next_command_id(),
                        2,
                        "current bound command1 must execute"
                    );
                    assert_eq!(authority.step_calls.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "activity command fence test panicked")?
}

async fn receive_ready_session_frame(
    stream: &mut DuplexStream,
    authority: &SlowOwner,
    session: &mut ServeState,
) -> TestResult<bool> {
    let next = poll::next(stream, authority, session).await;
    assert!(
        matches!(next, Next::Frame(Ok(_))),
        "actual queued client frame"
    );
    let Some(input) = receive(stream, authority, session, next)
        .await
        .map_err(|_| "session-bound receive")?
    else {
        return Ok(false);
    };
    command::execute(stream, authority, session, input)
        .await
        .map_err(|_| "session-bound dispatch")?;
    Ok(true)
}

/// Actual serve loop and domain99 publication, with a trusted authority double.
/// This tests transport fencing, not SQL admission or a source encounter grant.
#[test]
fn activity_noninitiator_serve_barrier_fences_two_already_queued_frames() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> TestResult {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = SlowOwner::default();
                    let mut source = admitted(false)?;
                    source.continuity.selected_capabilities = SelectedCapabilities::select(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[
                            6,
                            oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                        ],
                    )
                    .ok_or("activity capability")?;
                    let mut destination = source;
                    destination.game_session_id = GameSessionId::decode(&uuid_v7(0x23))?;
                    authority.pending_successor.set(Some(destination));
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                        StepDirection::East,
                    );
                    let old = command_frame(1, 1, &east)?;
                    // First packet is read before the non-initiator's barrier is noticed;
                    // the second remains unread in the socket until after new setup.
                    write_frame(&mut client, &old).await?;
                    write_frame(&mut client, &old).await?;
                    let serving = Box::pin(serve_admitted_actor(
                        &mut server,
                        source,
                        &authority,
                        IDLE_LIVENESS,
                    ));
                    let driving = async move {
                        let mut commits = 0;
                        for _ in 0..64 {
                            let frame = tokio::time::timeout(
                                std::time::Duration::from_secs(2),
                                read_frame(&mut client),
                            )
                            .await??;
                            let envelope = decode_wire_envelope(&frame)?;
                            if envelope.message_type() == MessageType::SnapshotCommit {
                                commits += 1;
                            }
                            if commits == 3 {
                                break;
                            }
                        }
                        assert_eq!(
                            commits, 3,
                            "source initial, actual domain99 barrier, destination initial"
                        );
                        let west = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                            StepDirection::West,
                        );
                        let current = oteryn_protocol_oteryn::encode_client_command_for_session(
                            1,
                            &ClientCommandValue {
                                command_id: 1,
                                command_type: 1,
                                payload: &west,
                            },
                            destination.game_session_id.as_bytes(),
                        )?;
                        write_frame(&mut client, &current).await?;
                        let mut result = false;
                        for _ in 0..64 {
                            let frame = tokio::time::timeout(
                                std::time::Duration::from_secs(2),
                                read_frame(&mut client),
                            )
                            .await??;
                            let envelope = decode_wire_envelope(&frame)?;
                            if envelope.message_type() == MessageType::CommandResult {
                                assert_eq!(
                                    oteryn_protocol_oteryn::decode_command_result(
                                        envelope.payload()
                                    )?
                                    .command_id,
                                    1
                                );
                                result = true;
                                break;
                            }
                        }
                        assert!(result, "fresh bound command1 receives its own result");
                        tokio::io::AsyncWriteExt::shutdown(&mut client).await?;
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    };
                    // This crate intentionally does not enable Tokio macros.
                    // Poll both real socket peers until each completed once.
                    let mut serving = std::pin::pin!(serving);
                    let mut driving = std::pin::pin!(driving);
                    let mut served = false;
                    let mut driven = None;
                    std::future::poll_fn(|cx| {
                        if !served && std::future::Future::poll(serving.as_mut(), cx).is_ready() {
                            served = true;
                        }
                        if driven.is_none()
                            && let std::task::Poll::Ready(result) =
                                std::future::Future::poll(driving.as_mut(), cx)
                        {
                            driven = Some(result);
                        }
                        if served && let Some(result) = driven.take() {
                            return std::task::Poll::Ready(result);
                        }
                        std::task::Poll::Pending
                    })
                    .await?;
                    assert_eq!(authority.step_calls.get(), 1);
                    assert_eq!(
                        authority.last_step.get(),
                        Some(StepDirection::West),
                        "only new-session input reaches owner"
                    );
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "serve barrier test panicked")?
}

#[path = "inbound/native_idle_cues_tests.rs"]
mod native_idle_cues_tests;
