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
    slow_browse: Cell<bool>,
    visibility_delay: Cell<std::time::Duration>,
    vitals_delay: Cell<std::time::Duration>,
    browse_reads: Cell<usize>,
    sound_reads: Cell<usize>,
}

impl FreshAdmissionAuthority for SlowOwner {
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
        let delay = self.visibility_delay.get();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        Some(observation())
    }

    async fn tick_vitals(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> Option<(u64, crate::gameplay_transport::actor_spell::ActorVitals)> {
        let delay = self.vitals_delay.get();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        None
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
    authority.visibility_delay.set(if browse {
        std::time::Duration::ZERO
    } else {
        VISIBILITY_REFRESH * 2
    });
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
        execute_ready_frame(&mut server, &authority, &mut session).await?;
        assert_eq!(session.admitted.continuity.next_command_id, command_id + 1);
        // A due refresh still runs after the bounded input turn.
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
fn legacy_slow_visibility_refresh_dispatches_ready_client_before_next_refresh() -> TestResult {
    run(false)
}

#[test]
fn slow_vitals_refresh_services_ready_chat_before_next_vitals() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> TestResult {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(async {
                    let authority = SlowOwner::default();
                    authority
                        .vitals_delay
                        .set(SERENE_EVALUATION + std::time::Duration::from_millis(400));
                    let (mut server, _client) = tokio::io::duplex(1 << 20);
                    let mut admitted = admitted(false)?;
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
                    // Isolate two ready periodic owners. Delay measures from tick
                    // selection, so completing a slow Serene leaves it due again.
                    session.visibility_refresh = None;
                    let started = tokio::time::Instant::now();
                    session.serene = Some(tokio::time::interval_at(
                        started + SERENE_EVALUATION,
                        SERENE_EVALUATION,
                    ));
                    session.chat_refresh = Some(tokio::time::interval_at(
                        started + CHAT_REFRESH,
                        CHAT_REFRESH,
                    ));
                    for refresh in session
                        .serene
                        .iter_mut()
                        .chain(session.chat_refresh.iter_mut())
                    {
                        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    }
                    tokio::time::advance(SERENE_EVALUATION).await;
                    let first = poll::next(&mut server, &authority, &mut session).await;
                    assert!(matches!(first, Next::Serene));
                    assert!(
                        receive(&mut server, &authority, &mut session, first)
                            .await
                            .map_err(|_| "slow vitals refresh")?
                            .is_none()
                    );
                    assert!(
                        matches!(
                            poll::next(&mut server, &authority, &mut session).await,
                            Next::Chat
                        ),
                        "ready chat must run even when Serene is overdue again"
                    );
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "periodic scheduler test thread panicked")?
}

#[test]
fn slow_refresh_answers_probes_and_dispatches_client_input() -> TestResult {
    slow_refresh_session(std::time::Duration::from_millis(650))
}

#[test]
fn refresh_longer_than_probe_interval_keeps_answering_client_in_control() -> TestResult {
    slow_refresh_session(IDLE_LIVENESS.interval + std::time::Duration::from_millis(650))
}

// Unlike pre-buffered, one-command PG connections, the peer stays connected
// across six production liveness intervals while the refresh remains due.
fn slow_refresh_session(delay: std::time::Duration) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> TestResult {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(async {
                    let authority = SlowOwner::default();
                    authority.visibility_delay.set(delay);
                    let started = tokio::time::Instant::now();
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    let served = async {
                        let end = serve_admitted_actor(
                            &mut server,
                            admitted(false)?,
                            &authority,
                            IDLE_LIVENESS,
                        )
                        .await;
                        drop(server);
                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(end)
                    };
                    let peer = async {
                        let mut frames = FrameReader::default();
                        let mut probes = 0;
                        let mut result_seen = false;
                        while probes < 6 || !result_seen {
                            let bytes = frames.next(&mut client).await?;
                            let envelope = decode_wire_envelope(&bytes)?;
                            match envelope.message_type() {
                                MessageType::LivenessProbe => {
                                    let id = oteryn_protocol_oteryn::decode_liveness_probe(
                                        envelope.payload(),
                                    )?;
                                    probes += 1;
                                    let ack =
                                        oteryn_protocol_oteryn::encode_liveness_ack(1, id, 0)?;
                                    write_frame(&mut client, &ack).await?;
                                    if probes == 1 {
                                        let payload = encode_container_view_intent(
                                            &ContainerViewIntent::Close { view_id: 15 },
                                        )
                                        .map_err(|e| format!("{e:?}"))?;
                                        write_frame(
                                            &mut client,
                                            &command_frame(
                                                1,
                                                COMMAND_TYPE_CONTAINER_VIEW_INTENT,
                                                &payload,
                                            )?,
                                        )
                                        .await?;
                                    }
                                }
                                MessageType::CommandResult => result_seen = true,
                                _ => {}
                            }
                        }
                        drop(client);
                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                    };
                    let (mut served, mut peer) = (std::pin::pin!(served), std::pin::pin!(peer));
                    let (mut end, mut peer_done) = (None, None);
                    std::future::poll_fn(|cx| {
                        if end.is_none()
                            && let std::task::Poll::Ready(value) = served.as_mut().poll(cx)
                        {
                            end = Some(value);
                        }
                        if peer_done.is_none()
                            && let std::task::Poll::Ready(value) = peer.as_mut().poll(cx)
                        {
                            peer_done = Some(value);
                        }
                        if end.is_some() && peer_done.is_some() {
                            std::task::Poll::Ready(())
                        } else {
                            std::task::Poll::Pending
                        }
                    })
                    .await;
                    let end = end.ok_or("server incomplete")??;
                    assert!(
                        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                        "answering client lost control"
                    );
                    peer_done.ok_or("peer incomplete")??;
                    eprintln!(
                        "slow_refresh delay_ms={} session_ms={}",
                        delay.as_millis(),
                        started.elapsed().as_millis()
                    );
                    assert!(started.elapsed() >= IDLE_LIVENESS.interval * 6);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "slow refresh test thread panicked")?
}
