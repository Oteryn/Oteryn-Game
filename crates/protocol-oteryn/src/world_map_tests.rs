#![allow(clippy::expect_used)]

use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const WORLD_MAP_PROTO: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_map_v1.proto");

fn position(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}

fn header(origin: ActorPosition) -> MapViewHeader {
    MapViewHeader {
        content_generation: [0x11; 32],
        bundle_digest: [0x22; 32],
        reset_epoch: 3,
        origin,
        first_visible_floor: first_visible_floor_start(origin.floor),
    }
}

fn item_ref(id: u32) -> MapDefinition {
    MapDefinition::Item(NonZeroU32::new(id).expect("non-zero"))
}

fn ground() -> MapItem {
    MapItem {
        definition: MapDefinition::Terrain(NonZeroU32::new(7).expect("non-zero")),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::BaseOrdinal {
            ordinal: 0,
            object_revision: 0,
        },
        appearance_id: 4_526,
    }
}

fn tile(at: ActorPosition) -> MapTile {
    MapTile {
        position: at,
        items: vec![ground()],
        more: false,
        ground_speed: 150,
    }
}

/// The largest `MapItemV1`: a `base_ordinal` entry with every value at its maximum.
fn largest_item(ordinal: u8) -> MapItem {
    MapItem {
        definition: item_ref(u32::MAX),
        count: MAX_COUNT,
        sub_type: u32::MAX,
        origin: MapOrigin::BaseOrdinal {
            ordinal: ordinal.min(MAX_BASE_ORDINAL),
            object_revision: u64::MAX,
        },
        appearance_id: u16::MAX,
    }
}

/// A 10-entry tile with the largest values at `at`.
fn largest_tile(at: ActorPosition) -> MapTile {
    MapTile {
        position: at,
        items: (0..MAX_TILE_ITEMS as u8).map(largest_item).collect(),
        more: true,
        ground_speed: MAX_GROUND_SPEED,
    }
}

/// Every tile position in view of `origin`, in wire order.
fn window(origin: ActorPosition) -> Vec<ActorPosition> {
    let floors = floors_in_view(origin.floor).expect("floor");
    let mut positions = Vec::new();
    for floor in floors {
        let shift = i32::from(floor - origin.floor);
        for y in 0..VIEW_HEIGHT {
            for x in 0..VIEW_WIDTH {
                positions.push(position(
                    origin.x - VIEW_LEFT + shift + x,
                    origin.y - VIEW_TOP + shift + y,
                    floor,
                ));
            }
        }
    }
    positions
}

/// The largest valid origin: world coordinates at the top of `0..=65,535` and the floor whose
/// visible-floor encoding is longest.
fn far_origin() -> ActorPosition {
    position(65_000, 65_000, -7)
}

fn snapshot_round_trip(snapshot: &WorldMapViewSnapshot) -> Vec<u8> {
    let bytes = encode_world_map_snapshot(snapshot).expect("encode");
    assert_eq!(decode_world_map_snapshot(&bytes).as_ref(), Ok(snapshot));
    bytes
}

/// Re-encodes a snapshot with one tile replaced by raw tile bytes.
fn snapshot_with_raw_tile(origin: ActorPosition, raw_tile: &[u8]) -> Vec<u8> {
    let mut payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: header(origin),
        tiles: Vec::new(),
    })
    .expect("encode");
    push_message_field(&mut payload, 2, raw_tile);
    payload
}

fn raw_position(at: ActorPosition) -> Vec<u8> {
    let mut output = Vec::new();
    push_position(&mut output, 1, &at);
    output
}

/// The tile bytes of one item at `at` with raw item bytes.
fn raw_tile_with_item(at: ActorPosition, raw_item: &[u8]) -> Vec<u8> {
    let mut output = raw_position(at);
    push_message_field(&mut output, 2, raw_item);
    output
}

fn raw_item(fields: &[(u64, u64)]) -> Vec<u8> {
    let mut output = Vec::new();
    for (field, value) in fields {
        push_varint_field(&mut output, *field, *value);
    }
    output
}

fn decode_item_fields(fields: &[(u64, u64)]) -> Result<WorldMapViewSnapshot> {
    let origin = position(100, 100, -7);
    decode_world_map_snapshot(&snapshot_with_raw_tile(
        origin,
        &raw_tile_with_item(origin, &raw_item(fields)),
    ))
}

#[test]
fn snapshots_and_deltas_round_trip() {
    let origin = position(100, 200, -7);
    let mut tiles: Vec<MapTile> = [
        position(92, 194, -7),
        position(100, 200, -7),
        position(109, 207, -7),
        position(100, 201, -6),
        position(107, 207, 0),
    ]
    .into_iter()
    .map(tile)
    .collect();
    tiles[1].items.push(MapItem {
        definition: item_ref(3_031),
        count: 100,
        sub_type: 0,
        origin: MapOrigin::Handle(NonZeroU64::new(9).expect("non-zero")),
        appearance_id: 3_031,
    });
    tiles[1].items.push(MapItem {
        definition: item_ref(2_160),
        count: 1,
        sub_type: 2,
        origin: MapOrigin::DisplayOnly,
        appearance_id: 0,
    });
    tiles[2].items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 63,
        object_revision: 4,
    };
    tiles[2].more = true;
    tiles[3].ground_speed = 0;
    let snapshot = WorldMapViewSnapshot {
        header: header(origin),
        tiles,
    };
    snapshot_round_trip(&snapshot);
    snapshot_round_trip(&WorldMapViewSnapshot {
        header: header(origin),
        tiles: Vec::new(),
    });

    let view = snapshot.header;
    let mut moved = header(position(101, 199, -7));
    moved.first_visible_floor = -7;
    let delta = WorldMapViewDelta {
        header: moved,
        tiles: vec![tile(position(110, 193, -7))],
        cleared: vec![position(93, 193, -7), position(101, 199, -7)],
    };
    let bytes = encode_world_map_delta(&view, &delta).expect("encode");
    assert_eq!(decode_world_map_delta(&view, &bytes), Ok(delta));
}

#[test]
fn ground_speed_round_trips_at_its_bound_and_fails_closed_above_it() {
    let origin = position(100, 100, -7);
    let mut full = tile(origin);
    full.ground_speed = MAX_GROUND_SPEED;
    let mut absent = tile(origin);
    absent.ground_speed = 0;
    for tile in [full, absent.clone()] {
        snapshot_round_trip(&WorldMapViewSnapshot {
            header: header(origin),
            tiles: vec![tile],
        });
    }
    assert_eq!(tile_len(&absent) + 3, tile_len(&tile(origin)));

    let mut over = tile(origin);
    over.ground_speed = MAX_GROUND_SPEED + 1;
    let snapshot = WorldMapViewSnapshot {
        header: header(origin),
        tiles: vec![over],
    };
    assert_eq!(
        encode_world_map_snapshot(&snapshot),
        Err(WorldMapWireError::Malformed)
    );
    let mut raw = raw_tile_with_item(origin, &raw_item(&[(8, 7), (2, 1), (4, 0)]));
    push_varint_field(&mut raw, 4, u64::from(MAX_GROUND_SPEED) + 1);
    assert_eq!(
        decode_world_map_snapshot(&snapshot_with_raw_tile(origin, &raw)),
        Err(WorldMapWireError::Malformed)
    );
}

#[test]
fn definition_references_are_family_tagged_and_exactly_one() {
    let origin = position(100, 100, -7);
    let mut both = tile(origin);
    both.items.push(MapItem {
        definition: item_ref(2_160),
        ..ground()
    });
    let bytes = snapshot_round_trip(&WorldMapViewSnapshot {
        header: header(origin),
        tiles: vec![both],
    });
    // The Terrain ground carries field 8 and the Item field 1.
    assert!(bytes.windows(2).any(|pair| pair == [0x40, 7]));
    assert!(bytes.windows(3).any(|triple| triple == [0x08, 0xf0, 0x10]));

    for fields in [
        // both
        &[(1, 5), (8, 7), (2, 1), (4, 0)][..],
        // neither
        &[(2, 1), (4, 0)][..],
        // a zero Item reference, written explicitly
        &[(1, 0), (2, 1), (4, 0)][..],
    ] {
        let mut item = Vec::new();
        for (field, value) in fields {
            item.push((*field as u8) << 3);
            push_varint(&mut item, *value);
        }
        let payload = snapshot_with_raw_tile(origin, &raw_tile_with_item(origin, &item));
        assert_eq!(
            decode_world_map_snapshot(&payload),
            Err(WorldMapWireError::Malformed),
            "{fields:?}"
        );
    }
    // A zero Terrain reference, written explicitly.
    let item = [0x40, 0, 0x10, 1, 0x20, 0];
    assert_eq!(
        decode_world_map_snapshot(&snapshot_with_raw_tile(
            origin,
            &raw_tile_with_item(origin, &item)
        )),
        Err(WorldMapWireError::Malformed)
    );
}

#[test]
fn every_malformed_item_fails_closed() {
    // A valid baseline: Item 5, count 1, ordinal 0.
    assert!(decode_item_fields(&[(1, 5), (2, 1), (4, 1)]).is_ok());
    for fields in [
        // unknown field
        &[(1, 5), (2, 1), (4, 1), (10, 1)][..],
        // repeated field
        &[(1, 5), (2, 1), (2, 1), (4, 1)][..],
        // a count outside its range: absent (zero) and 101
        &[(1, 5), (4, 1)][..],
        &[(1, 5), (2, 101), (4, 1)][..],
        // a base ordinal of 64
        &[(1, 5), (2, 1), (4, 64)][..],
        // a zero handle
        &[(1, 5), (2, 1)][..],
        // display_only not true
        &[(1, 5), (2, 1), (6, 2)][..],
        // two origins
        &[(1, 5), (2, 1), (4, 1), (5, 9)][..],
        &[(1, 5), (2, 1), (5, 9), (6, 1)][..],
        // no origin
        &[(1, 5), (2, 1)][..],
        // an object revision on a handle and on a display_only entry
        &[(1, 5), (2, 1), (5, 9), (9, 1)][..],
        &[(1, 5), (2, 1), (6, 1), (9, 1)][..],
        // an appearance above 65,535
        &[(1, 5), (2, 1), (4, 1), (7, 65_536)][..],
        // a definition reference above u32
        &[(1, 1 << 32), (2, 1), (4, 1)][..],
    ] {
        assert_eq!(
            decode_item_fields(fields),
            Err(WorldMapWireError::Malformed),
            "{fields:?}"
        );
    }
    // An explicit zero handle and an explicit false display_only.
    for item in [[0x08, 5, 0x10, 1, 0x28, 0], [0x08, 5, 0x10, 1, 0x30, 0]] {
        let origin = position(100, 100, -7);
        assert_eq!(
            decode_world_map_snapshot(&snapshot_with_raw_tile(
                origin,
                &raw_tile_with_item(origin, &item)
            )),
            Err(WorldMapWireError::Malformed)
        );
    }
    // A wrong wire type on a known field.
    let origin = position(100, 100, -7);
    let item = [0x0a, 1, 5, 0x10, 1, 0x20, 0];
    assert_eq!(
        decode_world_map_snapshot(&snapshot_with_raw_tile(
            origin,
            &raw_tile_with_item(origin, &item)
        )),
        Err(WorldMapWireError::Malformed)
    );

    // The encoder refuses the same values.
    let mut item = ground();
    item.count = 0;
    let mut zero_count = tile(origin);
    zero_count.items = vec![item];
    item.count = MAX_COUNT + 1;
    let mut big_count = tile(origin);
    big_count.items = vec![item];
    let mut big_ordinal = tile(origin);
    big_ordinal.items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 64,
        object_revision: 0,
    };
    for tile in [zero_count, big_count, big_ordinal] {
        assert_eq!(
            encode_world_map_snapshot(&WorldMapViewSnapshot {
                header: header(origin),
                tiles: vec![tile],
            }),
            Err(WorldMapWireError::Malformed)
        );
    }
}

#[test]
fn every_malformed_tile_and_snapshot_fails_closed() {
    let origin = position(100, 100, -7);
    let valid = [(8, 7), (2, 1), (4, 0)];
    // An empty tile, a tile without a position, a repeated position, `more` not 1, an unknown
    // field and 11 items.
    let empty = raw_position(origin);
    let no_position = {
        let mut output = Vec::new();
        push_message_field(&mut output, 2, &raw_item(&valid));
        output
    };
    let mut repeated = raw_tile_with_item(origin, &raw_item(&valid));
    repeated.extend(raw_position(origin));
    let mut more_two = raw_tile_with_item(origin, &raw_item(&valid));
    push_varint_field(&mut more_two, 3, 2);
    let mut unknown = raw_tile_with_item(origin, &raw_item(&valid));
    push_varint_field(&mut unknown, 5, 1);
    for (name, raw) in [
        ("empty", &empty),
        ("no position", &no_position),
        ("repeated position", &repeated),
        ("more 2", &more_two),
        ("unknown", &unknown),
    ] {
        assert_eq!(
            decode_world_map_snapshot(&snapshot_with_raw_tile(origin, raw)),
            Err(WorldMapWireError::Malformed),
            "{name}"
        );
    }
    let mut eleven = raw_position(origin);
    for _ in 0..=MAX_TILE_ITEMS {
        push_message_field(&mut eleven, 2, &raw_item(&valid));
    }
    assert_eq!(
        decode_world_map_snapshot(&snapshot_with_raw_tile(origin, &eleven)),
        Err(WorldMapWireError::LimitExceeded)
    );
    let mut too_many = tile(origin);
    too_many.items = vec![ground(); MAX_TILE_ITEMS + 1];
    assert_eq!(
        encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: header(origin),
            tiles: vec![too_many],
        }),
        Err(WorldMapWireError::LimitExceeded)
    );

    // A tile outside the window, on each edge and on a floor out of view.
    for outside in [
        position(91, 100, -7),
        position(110, 100, -7),
        position(100, 93, -7),
        position(100, 108, -7),
        position(100, 100, -8),
        position(91, 94, -6),
    ] {
        let snapshot = WorldMapViewSnapshot {
            header: header(origin),
            tiles: vec![tile(outside)],
        };
        assert_eq!(
            encode_world_map_snapshot(&snapshot),
            Err(WorldMapWireError::Malformed),
            "{outside:?}"
        );
        let raw = raw_tile_with_item(outside, &raw_item(&valid));
        assert_eq!(
            decode_world_map_snapshot(&snapshot_with_raw_tile(origin, &raw)),
            Err(WorldMapWireError::Malformed),
            "{outside:?}"
        );
    }

    // Unsorted and duplicate tiles.
    for tiles in [
        vec![tile(position(101, 100, -7)), tile(position(100, 100, -7))],
        vec![tile(position(100, 101, -7)), tile(position(101, 100, -7))],
        vec![tile(position(101, 101, -6)), tile(position(100, 100, -7))],
        vec![tile(origin), tile(origin)],
    ] {
        let snapshot = WorldMapViewSnapshot {
            header: header(origin),
            tiles: tiles.clone(),
        };
        assert_eq!(
            encode_world_map_snapshot(&snapshot),
            Err(WorldMapWireError::Malformed)
        );
        let mut payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: header(origin),
            tiles: Vec::new(),
        })
        .expect("encode");
        for tile in &tiles {
            push_tile(&mut payload, 2, tile);
        }
        assert_eq!(
            decode_world_map_snapshot(&payload),
            Err(WorldMapWireError::Malformed)
        );
    }

    // No header, a repeated header, an unknown snapshot field and trailing garbage.
    let mut no_header = Vec::new();
    push_tile(&mut no_header, 2, &tile(origin));
    let base = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: header(origin),
        tiles: Vec::new(),
    })
    .expect("encode");
    let mut repeated_header = base.clone();
    repeated_header.extend_from_slice(&base);
    let mut unknown_field = base.clone();
    push_varint_field(&mut unknown_field, 3, 1);
    let mut truncated = base.clone();
    truncated.push(0x12);
    for payload in [no_header, repeated_header, unknown_field, truncated] {
        assert_eq!(
            decode_world_map_snapshot(&payload),
            Err(WorldMapWireError::Malformed)
        );
    }
}

#[test]
fn every_malformed_header_fails_closed() {
    let origin = position(100, 100, -7);
    let encode = |header: MapViewHeader| {
        encode_world_map_snapshot(&WorldMapViewSnapshot {
            header,
            tiles: Vec::new(),
        })
    };
    // first_visible_floor 1, one below the origin floor, and -7 with an origin at -10.
    let mut above = header(origin);
    above.first_visible_floor = 1;
    let mut below = header(origin);
    below.first_visible_floor = -8;
    let mut deep = header(position(100, 100, -10));
    deep.first_visible_floor = -7;
    // An origin floor outside -15..=0.
    let mut sky = header(position(100, 100, 1));
    sky.first_visible_floor = 1;
    let mut abyss = header(position(100, 100, -16));
    abyss.first_visible_floor = -16;
    for header in [above, below, deep, sky, abyss] {
        assert_eq!(
            encode(header),
            Err(WorldMapWireError::Malformed),
            "{header:?}"
        );
        let mut payload = Vec::new();
        push_header(&mut payload, &header);
        assert_eq!(
            decode_world_map_snapshot(&payload),
            Err(WorldMapWireError::Malformed),
            "{header:?}"
        );
    }
    // The bounds of each floor are valid: 0 and -7 on the surface, -8 and -10 at -10, -13 and
    // -15 at -15.
    for (floor, visible) in [
        (-7, 0),
        (-7, -7),
        (-10, -8),
        (-10, -10),
        (-15, -13),
        (-15, -15),
    ] {
        let mut valid = header(position(100, 100, floor));
        valid.first_visible_floor = visible;
        assert!(encode(valid).is_ok(), "{floor} {visible}");
    }
    // An absent first_visible_floor is 0, which an underground origin refuses.
    let underground = header(position(100, 100, -10));
    let mut payload = Vec::new();
    push_header(
        &mut payload,
        &MapViewHeader {
            first_visible_floor: 0,
            ..underground
        },
    );
    assert_eq!(
        decode_world_map_snapshot(&payload),
        Err(WorldMapWireError::Malformed)
    );

    // A digest or generation of another length, a missing one and a repeated epoch.
    let raw_header = |generation: Option<&[u8]>, digest: Option<&[u8]>, epochs: usize| {
        let mut fields = Vec::new();
        if let Some(generation) = generation {
            push_message_field(&mut fields, 1, generation);
        }
        if let Some(digest) = digest {
            push_message_field(&mut fields, 2, digest);
        }
        for _ in 0..epochs {
            push_varint_field(&mut fields, 3, 1);
        }
        push_position(&mut fields, 4, &origin);
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &fields);
        payload
    };
    assert!(decode_world_map_snapshot(&raw_header(Some(&[1; 32]), Some(&[2; 32]), 1)).is_ok());
    for payload in [
        raw_header(Some(&[1; 31]), Some(&[2; 32]), 1),
        raw_header(Some(&[1; 32]), Some(&[2; 33]), 1),
        raw_header(None, Some(&[2; 32]), 1),
        raw_header(Some(&[1; 32]), None, 1),
        raw_header(Some(&[1; 32]), Some(&[2; 32]), 2),
    ] {
        assert_eq!(
            decode_world_map_snapshot(&payload),
            Err(WorldMapWireError::Malformed)
        );
    }
}

#[test]
fn floor_sets_and_the_perspective_shift() {
    assert_eq!(floors_in_view(-7), Some(-7..=0));
    assert_eq!(floors_in_view(0), Some(-7..=0));
    assert_eq!(floors_in_view(-8), Some(-10..=-6));
    assert_eq!(floors_in_view(-10), Some(-12..=-8));
    assert_eq!(floors_in_view(-14), Some(-15..=-12));
    assert_eq!(floors_in_view(-15), Some(-15..=-13));
    assert_eq!(floors_in_view(1), None);
    assert_eq!(floors_in_view(-16), None);
    assert_eq!(window(position(100, 100, -7)).len(), MAX_SNAPSHOT_TILES);
    assert_eq!(window(position(100, 100, -10)).len(), 5 * 18 * 14);
    assert_eq!(window(position(100, 100, -15)).len(), 3 * 18 * 14);

    // Each floor's window is shifted by its floor difference to the origin.
    let origin = position(100, 100, -7);
    assert!(in_window(origin, position(92, 94, -7)));
    assert!(in_window(origin, position(109, 107, -7)));
    assert!(in_window(origin, position(93, 95, -6)));
    assert!(!in_window(origin, position(92, 94, -6)));
    assert!(in_window(origin, position(110, 108, -6)));
    assert!(in_window(origin, position(99, 101, 0)));
    assert!(in_window(origin, position(116, 114, 0)));
    assert!(!in_window(origin, position(117, 114, 0)));
    let deep = position(100, 100, -10);
    assert!(in_window(deep, position(90, 92, -12)));
    assert!(!in_window(deep, position(91, 93, -13)));
}

#[test]
fn encoded_sizes_stay_within_the_bounds() {
    let origin = far_origin();
    let largest = largest_item(MAX_BASE_ORDINAL);
    assert_eq!(item_len(&largest), 31);
    assert!(item_len(&largest) <= MAX_MAP_ITEM_BYTES);
    let handle = MapItem {
        origin: MapOrigin::Handle(NonZeroU64::new(u64::MAX).expect("non-zero")),
        ..largest
    };
    assert_eq!(item_len(&handle), 29);

    let tiles: Vec<MapTile> = window(origin).into_iter().map(largest_tile).collect();
    for tile in &tiles {
        assert!(framed(tile_len(tile)) <= MAX_MAP_TILE_BYTES, "{tile:?}");
    }
    let mut widest_header = header(origin);
    widest_header.reset_epoch = u64::MAX;
    widest_header.first_visible_floor = -7;
    widest_header.content_generation = [0xff; 32];
    assert!(framed(header_len(&widest_header)) <= MAX_MAP_HEADER_BYTES);

    let snapshot = WorldMapViewSnapshot {
        header: widest_header,
        tiles: tiles.clone(),
    };
    let bytes = snapshot_round_trip(&snapshot);
    assert!(bytes.len() <= MAX_SNAPSHOT_PAYLOAD_BYTES);
    assert_eq!(MAX_SNAPSHOT_PAYLOAD_BYTES, 725_888);
    // Two chunks under FND02-SNAPSHOT-CHUNK-BYTES.
    assert!(bytes.len().div_ceil(524_288) <= 2);

    let delta = WorldMapViewDelta {
        header: MapViewHeader {
            origin: position(origin.x + 1, origin.y, origin.floor),
            ..widest_header
        },
        tiles: tiles[..MAX_DELTA_ENTRIES]
            .iter()
            .filter(|tile| {
                in_window(
                    position(origin.x + 1, origin.y, origin.floor),
                    tile.position,
                )
            })
            .take(MAX_DELTA_ENTRIES)
            .cloned()
            .collect(),
        cleared: Vec::new(),
    };
    let bytes = encode_world_map_delta(&widest_header, &delta).expect("encode");
    assert!(bytes.len() <= MAX_DELTA_PAYLOAD_BYTES);
    assert_eq!(MAX_DELTA_PAYLOAD_BYTES, 93_376);
    assert_eq!(decode_world_map_delta(&widest_header, &bytes), Ok(delta));

    // One tile over the snapshot bound and one entry over the delta bound.
    let mut over = Vec::new();
    push_header(&mut over, &widest_header);
    for _ in 0..=MAX_SNAPSHOT_TILES {
        push_tile(&mut over, 2, &tile(origin));
    }
    assert_eq!(
        decode_world_map_snapshot(&over),
        Err(WorldMapWireError::LimitExceeded)
    );
    let mut too_many = snapshot.clone();
    too_many.tiles.push(tile(origin));
    assert_eq!(
        encode_world_map_snapshot(&too_many),
        Err(WorldMapWireError::LimitExceeded)
    );
    let cleared: Vec<ActorPosition> = window(origin)[..=MAX_DELTA_ENTRIES].to_vec();
    let delta = WorldMapViewDelta {
        header: widest_header,
        tiles: Vec::new(),
        cleared,
    };
    assert_eq!(
        encode_world_map_delta(&widest_header, &delta),
        Err(WorldMapWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    push_header(&mut payload, &widest_header);
    for position in &delta.cleared {
        push_position(&mut payload, 3, position);
    }
    assert_eq!(
        decode_world_map_delta(&widest_header, &payload),
        Err(WorldMapWireError::LimitExceeded)
    );
    assert_eq!(
        decode_world_map_snapshot(&vec![0; MAX_SNAPSHOT_PAYLOAD_BYTES + 1]),
        Err(WorldMapWireError::LimitExceeded)
    );
    assert_eq!(
        decode_world_map_delta(&widest_header, &vec![0; MAX_DELTA_PAYLOAD_BYTES + 1]),
        Err(WorldMapWireError::LimitExceeded)
    );
}

#[test]
fn a_ten_entry_tile_of_base_ordinals_with_the_largest_revisions_fits() {
    let tile = largest_tile(far_origin());
    assert!(tile.items.iter().all(|item| matches!(
        item.origin,
        MapOrigin::BaseOrdinal {
            object_revision: u64::MAX,
            ..
        }
    )));
    assert!(framed(tile_len(&tile)) <= MAX_MAP_TILE_BYTES);
    let bytes = snapshot_round_trip(&WorldMapViewSnapshot {
        header: header(far_origin()),
        tiles: vec![tile],
    });
    assert!(bytes.len() <= MAX_MAP_HEADER_BYTES + MAX_MAP_TILE_BYTES);
}

#[test]
fn deltas_bind_to_the_view_and_follow_the_origin_rule() {
    let origin = position(100, 100, -7);
    let view = header(origin);
    let delta = |header: MapViewHeader, tiles: Vec<MapTile>, cleared: Vec<ActorPosition>| {
        WorldMapViewDelta {
            header,
            tiles,
            cleared,
        }
    };
    let encode_raw = |delta: &WorldMapViewDelta| {
        let mut payload = Vec::new();
        push_header(&mut payload, &delta.header);
        for tile in &delta.tiles {
            push_tile(&mut payload, 2, tile);
        }
        for position in &delta.cleared {
            push_position(&mut payload, 3, position);
        }
        payload
    };

    // Another digest, generation or epoch is a binding mismatch on both sides.
    for other in [
        MapViewHeader {
            bundle_digest: [0x33; 32],
            ..view
        },
        MapViewHeader {
            content_generation: [0x44; 32],
            ..view
        },
        MapViewHeader {
            reset_epoch: 4,
            ..view
        },
    ] {
        let change = delta(other, vec![tile(origin)], Vec::new());
        assert_eq!(
            encode_world_map_delta(&view, &change),
            Err(WorldMapWireError::BindingMismatch)
        );
        assert_eq!(
            decode_world_map_delta(&view, &encode_raw(&change)),
            Err(WorldMapWireError::BindingMismatch)
        );
    }

    // A one-step move (each of the eight directions) is applied; an origin-only move is valid.
    for (dx, dy) in [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ] {
        let moved = delta(
            header(position(origin.x + dx, origin.y + dy, origin.floor)),
            Vec::new(),
            Vec::new(),
        );
        let bytes = encode_world_map_delta(&view, &moved).expect("encode");
        assert_eq!(decode_world_map_delta(&view, &bytes), Ok(moved));
    }
    // A delta that changes only first_visible_floor is valid.
    let roof = delta(
        MapViewHeader {
            first_visible_floor: -7,
            ..view
        },
        Vec::new(),
        Vec::new(),
    );
    let bytes = encode_world_map_delta(&view, &roof).expect("encode");
    assert_eq!(decode_world_map_delta(&view, &bytes), Ok(roof));

    // Two steps away, on another floor, or empty with nothing changed fails closed.
    for bad in [
        delta(header(position(102, 100, -7)), Vec::new(), Vec::new()),
        delta(
            header(position(102, 101, -7)),
            vec![tile(position(102, 101, -7))],
            Vec::new(),
        ),
        delta(
            header(position(100, 100, -6)),
            vec![tile(position(100, 100, -6))],
            Vec::new(),
        ),
        delta(view, Vec::new(), Vec::new()),
    ] {
        assert_eq!(
            encode_world_map_delta(&view, &bad),
            Err(WorldMapWireError::Malformed),
            "{:?}",
            bad.header.origin
        );
        assert_eq!(
            decode_world_map_delta(&view, &encode_raw(&bad)),
            Err(WorldMapWireError::Malformed),
            "{:?}",
            bad.header.origin
        );
    }

    // Tiles and cleared entries are in the new window, each strictly ascending and disjoint.
    let moved = header(position(101, 100, -7));
    for bad in [
        // In the old window only.
        delta(moved, vec![tile(position(92, 100, -7))], Vec::new()),
        delta(moved, Vec::new(), vec![position(92, 100, -7)]),
        // Unsorted cleared entries.
        delta(
            moved,
            Vec::new(),
            vec![position(101, 101, -7), position(101, 100, -7)],
        ),
        // A position both resent and cleared.
        delta(
            moved,
            vec![tile(position(101, 100, -7))],
            vec![position(101, 100, -7)],
        ),
    ] {
        assert_eq!(
            encode_world_map_delta(&view, &bad),
            Err(WorldMapWireError::Malformed)
        );
        assert_eq!(
            decode_world_map_delta(&view, &encode_raw(&bad)),
            Err(WorldMapWireError::Malformed)
        );
    }
}

#[test]
fn the_registries_bind_the_world_map_constants() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let find = |list: &str, id: u32| {
        let matching: Vec<&Value> = protocol[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        matching[0].clone()
    };
    let capability = find("capabilities", CAPABILITY_WORLD_MAP_VIEW_V1);
    assert_eq!(capability["name"], "WORLD_MAP_VIEW_V1");
    assert_eq!(capability["offered"], false);
    assert!(
        capability["offer_gate"]
            .as_str()
            .expect("offer gate")
            .contains("MAP-CUTOVER-1")
    );
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_WORLD_MAP_VIEW_REQUIRES)
    );
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_WORLD_MAP_VIEW])
    );
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_WORLD_MAP_VIEW_V1));

    let schema = "docs/contracts/protocol-oteryn/v1/world_map_v1.proto#";
    let domain = find("state_domains", STATE_DOMAIN_WORLD_MAP_VIEW);
    assert_eq!(domain["name"], "WORLD_MAP_VIEW");
    assert_eq!(domain["capability"], CAPABILITY_WORLD_MAP_VIEW_V1);
    for (key, type_id, name, message, maximum) in [
        (
            "delta_types",
            DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
            "WORLD_MAP_VIEW_DELTA_V1",
            "WorldMapViewDeltaV1",
            MAX_DELTA_PAYLOAD_BYTES,
        ),
        (
            "snapshot_types",
            SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            "WORLD_MAP_VIEW_SNAPSHOT_V1",
            "WorldMapViewSnapshotV1",
            MAX_SNAPSHOT_PAYLOAD_BYTES,
        ),
    ] {
        let types = domain[key].as_array().expect(key);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0]["id"], type_id);
        assert_eq!(types[0]["name"], name);
        assert_eq!(types[0]["max_payload_bytes"].as_u64(), Some(maximum as u64));
        assert_eq!(types[0]["payload_schema"], format!("{schema}{message}"));
        assert!(WORLD_MAP_PROTO.contains(&format!("message {message} {{")));
        assert!(WORLD_MAP_PROTO.contains(&format!("At most {maximum} bytes")));
    }

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |row: &str| {
        resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"] == row)
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    for (row, value) in [
        ("MAPW-RL-01", MAX_TILE_ITEMS),
        ("MAPW-RL-02", MAX_SNAPSHOT_TILES),
        ("MAPW-RL-03", MAX_DELTA_ENTRIES),
        ("MAPW-RL-04", MAX_MAP_VIEW_HANDLES),
    ] {
        assert_eq!(limit(row), Some(value as u64), "{row}");
        assert!(WORLD_MAP_PROTO.contains(row), "{row}");
    }
}
