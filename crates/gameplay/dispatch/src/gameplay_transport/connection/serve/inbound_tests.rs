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
    slow_visibility: Cell<bool>,
    browse_reads: Cell<usize>,
}

impl FreshAdmissionAuthority for SlowOwner {
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
    let next = poll::next(stream, session).await;
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
    let first = poll::next(&mut server, &mut session).await;
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
            poll::next(&mut server, &mut session).await,
            Next::Visibility
        ));
        assert!(tokio::time::Instant::now() - finished >= VISIBILITY_REFRESH);
    } else {
        // Sessions without capability 230 retain their pre-existing scheduling semantics.
        assert!(matches!(
            poll::next(&mut server, &mut session).await,
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
