//! Bounded in-memory Foundation frames; domain payloads use Session's public codecs.
//! Keeping the fixture here avoids adding a production-layer protocol dependency.
use oteryn_session::{
    ActorPosition, Admission, CharacterId, CharacterInventory, EntityRef, ItemEntry, ItemHandle,
    Session,
    container_tree::{ContainerView, ContainerViews},
    item_view, read_frame, world_map, write_frame,
};
use std::{future::Future, num::NonZeroU32, time::Duration};
use tokio::io::DuplexStream;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub const ORIGIN: ActorPosition = ActorPosition {
    x: 0,
    y: 0,
    floor: 0,
};
pub const OWN: EntityRef = EntityRef {
    identity: [0xa0; 16],
    generation: 1,
};
const SELECTED: &[u32] = &[4, 6, 11, 12, 14, 15, 18];

pub fn run<F: Future<Output = Result<(), BoxError>>>(
    test: impl FnOnce() -> F + Send + 'static,
) -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(test())
        })?
        .join()
        .map_err(|_| "item-action scenario thread failed")?
}

pub fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).unwrap_or(ItemHandle::MIN)
}

fn entry(value: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(value),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

pub fn views(source: u64) -> ContainerViews {
    ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: handle(7),
            parent_handle: Some(handle(1)),
            capacity: 20,
            entries: vec![entry(source)],
            browse: None,
        }],
    }
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut encoded = Vec::new();
    while value >= 128 {
        encoded.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    encoded.push(value as u8);
    encoded
}

pub fn scalar(tag: u64, value: u64) -> Vec<u8> {
    if value == 0 {
        Vec::new()
    } else {
        message(&[varint(tag << 3), varint(value)])
    }
}

pub fn field(tag: u64, bytes: &[u8]) -> Vec<u8> {
    message(&[
        varint((tag << 3) | 2),
        varint(bytes.len() as u64),
        bytes.to_vec(),
    ])
}

pub fn message(fields: &[Vec<u8>]) -> Vec<u8> {
    fields.concat()
}

pub fn envelope(kind: u64, generation: u64, sequence: u64, payload: &[u8]) -> Vec<u8> {
    message(&[
        scalar(1, kind),
        scalar(2, generation),
        scalar(3, sequence),
        field(4, payload),
    ])
}

fn uuid(marker: u8) -> [u8; 16] {
    let mut uuid = [marker; 16];
    uuid[6] = 0x70;
    uuid[8] = 0x80;
    uuid
}

pub async fn joined() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let mut accepted = message(&[
        field(1, &uuid(1)),
        field(2, &uuid(2)),
        field(3, &uuid(3)),
        scalar(4, 1),
        scalar(6, 7),
        scalar(7, 1),
        scalar(8, 1),
        scalar(9, 1),
    ]);
    for capability in SELECTED {
        accepted.extend(scalar(10, u64::from(*capability)));
    }
    write_frame(&mut peer, &envelope(2, 0, 0, &accepted)).await?;

    // Domain 1's own actor at the origin, without optional appearance/name capabilities.
    let actor = message(&[
        scalar(1, 1),
        field(2, &OWN.identity),
        scalar(3, OWN.generation),
        field(4, &[]),
        scalar(5, 1),
        scalar(6, 1),
        scalar(7, 100),
    ]);
    let spatial = message(&[
        field(1, &[1; 32]),
        field(2, &[]),
        field(3, &OWN.identity),
        field(4, &actor),
    ]);
    let inventory = item_view::encode_character_inventory_with_equip_drop(
        &CharacterInventory {
            main_backpack: Some(entry(1)),
            entries: vec![entry(2)],
            equipment: vec![],
        },
        true,
    )
    .map_err(|error| format!("inventory: {error:?}"))?;
    let containers = oteryn_session::container_tree::encode_container_views(&views(10))
        .map_err(|error| format!("containers: {error:?}"))?;
    let map = world_map::encode_world_map_snapshot(&world_map::WorldMapViewSnapshot {
        header: world_map::MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: ORIGIN,
            first_visible_floor: 0,
        },
        tiles: vec![world_map::MapTile {
            position: ORIGIN,
            more: false,
            ground_speed: 0,
            items: vec![world_map::MapItem {
                definition: world_map::MapDefinition::Item(NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: world_map::MapOrigin::Handle(handle(40)),
                appearance_id: 0,
            }],
        }],
    })
    .map_err(|error| format!("map: {error:?}"))?;
    let mut body = Vec::new();
    for (id, kind, payload) in [
        (1, 2, spatial),
        (9, 1, inventory),
        (14, 1, containers),
        (17, 1, map),
    ] {
        body.extend(field(
            1,
            &message(&[
                scalar(1, id),
                scalar(2, 1),
                scalar(3, kind),
                field(4, &payload),
            ]),
        ));
    }
    let begin = message(&[
        scalar(1, 1),
        scalar(2, 1),
        scalar(3, body.len() as u64),
        scalar(4, 40),
    ]);
    for (kind, payload) in [
        (11, begin),
        (12, message(&[scalar(1, 1), field(3, &body)])),
        (13, scalar(1, 1)),
    ] {
        write_frame(&mut peer, &envelope(kind, 1, 0, &payload)).await?;
    }
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&uuid(4))?,
            admission_material: b"fixture-grant",
            client_build_id: "item-action-test",
            supported_capabilities: SELECTED,
            deadline: Duration::from_secs(1),
        },
    )
    .await?;
    let _bootstrap = read_frame(&mut peer).await?;
    Ok((session, peer))
}
