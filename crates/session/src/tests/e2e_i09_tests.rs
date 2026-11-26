//! CLIENT-I09 replay for furniture rotation.
//!
//! The owning client path is `crates/session/src/map_transport.rs::apply`, which validates and
//! decodes domain 17, advances its revision, and emits `SessionEvent::WorldMap`; callers observe
//! that client-visible value through `crates/session/src/session_access.rs::Session::take_events`.
//! No consumer exists under `crates/client-ui/src/features/` or
//! `crates/client-runtime/src/features/`; `tools/dev-client/src/lib.rs::take_events` delegates to
//! this session owner.
//!
//! Capture files in `E2E_ALL_CAPTURE_DIR` (separate text fields with one ASCII space):
//! - `i09-map-before.bin`: raw `GatedSnapshot::payload` from
//!   `Session::gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)` after walking and before USE.
//! - `i09-map-before.txt`: decimal `predecessor_sequence domain_id snapshot_type revision\n`
//!   as UTF-8. The last three fields come from that same `GatedSnapshot`. Use the first
//!   rotation map delta's `server_sequence - 1` as the predecessor sequence, even when
//!   command results or other domains consumed sequence numbers after the map was saved.
//!   Require the saved revision to match the delta's base and no intervening map replacement.
//! - `i09-map-delta.bin`: raw `encode_world_map_delta(&delta.value.header, &delta.value)`
//!   bytes from the rotation `SessionEvent::WorldMap` returned by `Session::take_events`.
//! - `i09-expected.txt`: decimal `base_revision new_revision server_sequence definition appearance\n`
//!   as UTF-8, using the applied delta and its rotated item.
//! - `i09-result.bin`: raw USE `GatedCommandResult::payload`, retained as server-side evidence.

use super::*;
use oteryn_protocol_oteryn::world_map::{
    self, MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewDelta,
    WorldMapViewSnapshot,
};
use std::{num::NonZeroU32, path::Path};

#[test]
fn furniture_rotation_real_payload_replays_and_malformed_delta_is_atomic() -> Result<(), BoxError> {
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    run_replay(directory.as_deref().map(Path::new))
}

#[test]
fn furniture_rotation_replays_from_captured_predecessor() -> Result<(), BoxError> {
    let header = MapViewHeader {
        content_generation: [0x44; 32],
        bundle_digest: [0x55; 32],
        ..synthetic_header()
    };
    let (delta, (base, new, _, definition, appearance)) = synthetic_fixture(header)?;
    let mut before = world_map::decode_world_map_delta(&header, &delta)
        .map_err(|error| format!("fixture decode: {error:?}"))?;
    let chair = &mut before.tiles[0].items[1];
    chair.origin = MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: 0,
    };
    chair.appearance_id = 2358;
    let snapshot = world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
        header,
        tiles: before.tiles,
    })
    .map_err(|error| format!("fixture snapshot: {error:?}"))?;
    let directory = std::env::temp_dir().join(format!(
        "oteryn-i09-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&directory)?;
    let result = (|| {
        std::fs::write(directory.join("i09-map-before.bin"), snapshot)?;
        std::fs::write(
            directory.join("i09-map-before.txt"),
            format!(
                "167 {} {} {base}\n",
                world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            ),
        )?;
        std::fs::write(directory.join("i09-map-delta.bin"), delta)?;
        std::fs::write(
            directory.join("i09-expected.txt"),
            format!("{base} {new} 168 {definition} {appearance}\n"),
        )?;
        run_replay(Some(&directory))
    })();
    std::fs::remove_dir_all(directory)?;
    result
}

fn run_replay(root: Option<&Path>) -> Result<(), BoxError> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(scope, || block_on(replay(root))?)?
            .join()
            .map_err(|_| "I09 replay thread panicked")?
    })
}

async fn replay(root: Option<&Path>) -> Result<(), BoxError> {
    let ((payload, expected), (predecessor_sequence, map)) = if let Some(root) = root {
        (capture_delta(root)?, capture_predecessor(root)?)
    } else {
        let header = synthetic_header();
        let (payload, expected) = synthetic_fixture(header)?;
        let map = GatedSnapshot {
            domain_id: world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
            snapshot_type: world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            revision: expected.0,
            payload: world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
                header,
                tiles: vec![],
            })
            .map_err(|error| format!("map: {error:?}"))?,
        };
        ((payload, expected), (40, map))
    };
    let (client, mut server) =
        tokio::io::duplex(oteryn_protocol_oteryn::MAX_WIRE_FRAME_BYTES as usize + 4);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        write_frame(
            &mut server,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &[world_map::CAPABILITY_WORLD_MAP_VIEW_V1],
            })?,
        )
        .await?;
        // Admission requires a spatial snapshot from the same content generation as the map.
        let header = world_map::decode_world_map_snapshot(&map.payload)
            .map_err(|error| format!("map: {error:?}"))?
            .header;
        let spatial_payload = encode_world_spatial(&WireSpatialObservation {
            content_generation: header.content_generation,
            actor_position: header.origin,
        });
        for frame in oteryn_protocol_oteryn::encode_snapshot(
            1,
            1,
            predecessor_sequence,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: map.domain_id,
                    revision: map.revision,
                    snapshot_type: map.snapshot_type,
                    payload: &map.payload,
                },
            ],
        )? {
            write_frame(&mut server, &frame).await?;
        }
        write_frame(
            &mut server,
            &encode_state_delta(
                1,
                expected.2,
                world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                expected.0,
                expected.1,
                world_map::DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
                &payload,
            )?,
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let admission = Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: CharacterId::decode(&uuid_v7(4))?,
        admission_material: b"grant",
        client_build_id: "e2e-i09-replay",
        supported_capabilities: &[world_map::CAPABILITY_WORLD_MAP_VIEW_V1],
        deadline: DEADLINE,
    };
    let mut session = Session::admit(client, admission).await?;
    assert_eq!(session.last_server_sequence(), predecessor_sequence);
    // The delta fits in the duplex buffer. Keep the peer open while processing the queued frame.
    let _server = peer.await??;
    session.service_liveness(Duration::from_millis(50)).await?;
    let events = session.take_events();
    let update = events
        .iter()
        .find_map(|event| match event {
            SessionEvent::WorldMap(update) => Some(update),
            _ => None,
        })
        .ok_or("missing world-map event")?;
    let item = update.value.tiles.iter().flat_map(|tile| &tile.items).find(|item| matches!(item.origin, MapOrigin::BaseOrdinal { object_revision, .. } if object_revision == 1)).ok_or("missing rotated item")?;
    assert_eq!(
        (
            update.base_revision,
            update.new_revision,
            update.server_sequence
        ),
        (expected.0, expected.1, expected.2)
    );
    assert_eq!(
        item.definition,
        MapDefinition::Item(NonZeroU32::new(expected.3).ok_or("zero expected definition")?)
    );
    assert_eq!(item.appearance_id, expected.4);
    assert!(map_transport::apply(&mut session, expected.2 + 1, &[0xff]).is_err());
    assert!(session.take_events().is_empty());
    Ok(())
}

type Fixture = (Vec<u8>, (u64, u64, u64, u32, u16));

fn capture_delta(root: &Path) -> Result<Fixture, BoxError> {
    let payload = std::fs::read(root.join("i09-map-delta.bin"))?;
    let values = std::fs::read_to_string(root.join("i09-expected.txt"))?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [base, new, sequence, definition, appearance] = values.as_slice() else {
        return Err("invalid I09 expected capture".into());
    };
    Ok((
        payload,
        (
            *base,
            *new,
            *sequence,
            u32::try_from(*definition)?,
            u16::try_from(*appearance)?,
        ),
    ))
}

fn capture_predecessor(root: &Path) -> Result<(u64, GatedSnapshot), BoxError> {
    let values = std::fs::read_to_string(root.join("i09-map-before.txt"))?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [sequence, domain, snapshot_type, revision] = values.as_slice() else {
        return Err("invalid I09 predecessor capture".into());
    };
    Ok((
        *sequence,
        GatedSnapshot {
            domain_id: u32::try_from(*domain)?,
            snapshot_type: u32::try_from(*snapshot_type)?,
            revision: *revision,
            payload: std::fs::read(root.join("i09-map-before.bin"))?,
        },
    ))
}

fn synthetic_header() -> MapViewHeader {
    MapViewHeader {
        content_generation: CONTENT_GENERATION,
        bundle_digest: [0x22; 32],
        reset_epoch: 0,
        origin: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        first_visible_floor: 0,
    }
}

fn synthetic_fixture(header: MapViewHeader) -> Result<Fixture, BoxError> {
    let definition = NonZeroU32::new(42).ok_or("fixture definition")?;
    let delta = WorldMapViewDelta {
        header,
        tiles: vec![MapTile {
            position: ActorPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
            items: vec![
                MapItem {
                    definition: MapDefinition::Terrain(
                        NonZeroU32::new(1).ok_or("fixture terrain")?,
                    ),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 0,
                        object_revision: 0,
                    },
                    appearance_id: 100,
                },
                MapItem {
                    definition: MapDefinition::Item(definition),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 1,
                        object_revision: 1,
                    },
                    appearance_id: 2359,
                },
            ],
            more: false,
            ground_speed: 100,
        }],
        cleared: vec![],
    };
    Ok((
        world_map::encode_world_map_delta(&header, &delta)
            .map_err(|error| format!("fixture encode: {error:?}"))?,
        (1, 2, 41, definition.get(), 2359),
    ))
}
