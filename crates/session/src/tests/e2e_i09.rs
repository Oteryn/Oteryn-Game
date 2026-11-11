//! CLIENT-I09 replay for furniture rotation.
//!
//! The owning client path is `crates/session/src/map_transport.rs::apply`, which validates and
//! decodes domain 17, advances its revision, and emits `SessionEvent::WorldMap`; callers observe
//! that client-visible value through `crates/session/src/session_access.rs::Session::take_events`.
//! No consumer exists under `crates/client-ui/src/features/` or
//! `crates/client-runtime/src/features/`; `tools/dev-client/src/lib.rs::take_events` delegates to
//! this session owner.
use super::*;
use oteryn_protocol_oteryn::world_map::{
    self, MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewDelta,
    WorldMapViewSnapshot,
};
use std::num::NonZeroU32;

#[test]
fn furniture_rotation_real_payload_replays_and_malformed_delta_is_atomic() -> Result<(), BoxError> {
    block_on(run_replay())?
}

async fn run_replay() -> Result<(), BoxError> {
    let (payload, expected) = fixture()?;
    let header = captured_header(&payload)?;
    let (client, mut server) = tokio::io::duplex(256 * 1024);
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
        let spatial_payload = encode_world_spatial(&WireSpatialObservation {
            content_generation: header.content_generation,
            actor_position: header.origin,
        });
        let map_payload = world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
            header,
            tiles: vec![],
        })
        .map_err(|error| format!("map: {error:?}"))?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            40,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                    revision: expected.0,
                    snapshot_type: world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                    payload: &map_payload,
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
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok::<(), BoxError>(())
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
    peer.await??;
    Ok(())
}

type Fixture = (Vec<u8>, (u64, u64, u64, u32, u16));

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&directory);
        let payload = std::fs::read(root.join("i09-map-delta.bin"))?;
        let values = std::fs::read_to_string(root.join("i09-expected.txt"))?
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [base, new, sequence, definition, appearance] = values.as_slice() else {
            return Err("invalid I09 expected capture".into());
        };
        return Ok((
            payload,
            (
                *base,
                *new,
                *sequence,
                u32::try_from(*definition)?,
                u16::try_from(*appearance)?,
            ),
        ));
    }
    let header = MapViewHeader {
        content_generation: [0x73; 32],
        bundle_digest: [0x94; 32],
        reset_epoch: 7,
        origin: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        first_visible_floor: 0,
    };
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

// Captures use the production encoder, which writes the shared header first. Decode that
// header as an empty snapshot, then let the normal delta decoder validate the complete payload.
fn captured_header(
    payload: &[u8],
) -> Result<oteryn_protocol_oteryn::world_map::MapViewHeader, BoxError> {
    let mut cursor = 0;
    if oteryn_protocol_oteryn::read_varint(payload, &mut cursor)? != 0x0a {
        return Err("map capture does not start with its header".into());
    }
    let length = usize::try_from(oteryn_protocol_oteryn::read_varint(payload, &mut cursor)?)?;
    let end = cursor
        .checked_add(length)
        .ok_or("map header length overflow")?;
    let header = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
        payload.get(..end).ok_or("truncated map header")?,
    )
    .map_err(|error| format!("map header: {error:?}"))?
    .header;
    oteryn_protocol_oteryn::world_map::decode_world_map_delta(&header, payload)
        .map_err(|error| format!("map capture: {error:?}"))?;
    Ok(header)
}

#[test]
fn captured_header_reads_non_fixture_binding_and_rejects_truncation() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::world_map::*;
    let header = MapViewHeader {
        content_generation: [0x73; 32],
        bundle_digest: [0x94; 32],
        reset_epoch: 7,
        origin: ActorPosition {
            x: 100,
            y: 100,
            floor: 0,
        },
        first_visible_floor: 0,
    };
    let payload = encode_world_map_delta(
        &header,
        &WorldMapViewDelta {
            header,
            tiles: vec![],
            cleared: vec![header.origin],
        },
    )
    .map_err(|error| format!("map fixture: {error:?}"))?;
    assert_eq!(captured_header(&payload)?, header);
    assert!(captured_header(&payload[..payload.len() - 1]).is_err());
    Ok(())
}
