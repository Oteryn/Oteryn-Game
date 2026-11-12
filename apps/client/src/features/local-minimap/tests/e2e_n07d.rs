//! N07d replay: the world-map snapshot a real server admitted, taken from an admitted `Session`
//! (`Session::admit`, then `world_map_snapshot()`), feeds the persisted explored minimap through
//! `Minimap::observe` (`state.rs`); the marker editor (`edit_marker`, `remove_marker`) then works
//! at the server-reported position. Markers are client-local: no server command carries them.
use super::*;
use oteryn_protocol_oteryn::world_map::{
    CAPABILITY_WORLD_MAP_VIEW_V1, MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
    STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewSnapshot, encode_world_map_snapshot,
    first_visible_floor_start,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, WorldSpatialObservation,
    encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, GameSessionId, ServerAcceptedValue, WorldId,
    encode_server_accepted, encode_single_chunk_snapshot,
};
use oteryn_session::{Admission, CharacterId, Session, read_frame, write_frame};
use std::num::NonZeroU32;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

fn id(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

/// The admitted map snapshot payload and the values the server reported for it.
fn captured() -> Result<(Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("n07d-snapshot.bin"))?,
            std::fs::read_to_string(root.join("n07d-expected.txt"))?,
        ));
    }
    let origin = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let ground = |x| MapTile {
        position: ActorPosition { x, ..origin },
        more: false,
        ground_speed: 100,
        items: vec![MapItem {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::BaseOrdinal {
                ordinal: 0,
                object_revision: 1,
            },
            appearance_id: 0,
        }],
    };
    let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [0x22; 32],
            reset_epoch: 3,
            origin,
            first_visible_floor: first_visible_floor_start(origin.floor),
        },
        tiles: vec![ground(99), ground(100), ground(101)],
    })
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        payload,
        format!(
            "revision=4\norigin=100,200,-7\nfirst_visible_floor={}\nreset_epoch=3\ntiles=3\nwalkable=3\n",
            first_visible_floor_start(-7)
        ),
    ))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

async fn admit(payload: Vec<u8>, revision: u64) -> Result<WorldMapViewSnapshot, Error> {
    let snapshot = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&payload)
        .map_err(|error| format!("captured snapshot: {error:?}"))?;
    let spatial = encode_world_spatial(&WorldSpatialObservation {
        content_generation: snapshot.header.content_generation,
        actor_position: snapshot.header.origin,
    });
    let (client, mut server) = tokio::io::duplex(1 << 20);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        write_frame(
            &mut server,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&id(1))?,
                world_id: WorldId::decode(&id(2))?,
                channel_id: ChannelId::decode(&id(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: &[CAPABILITY_WORLD_MAP_VIEW_V1],
            })?,
        )
        .await?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            40,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: 1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
                    revision,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                    payload: &payload,
                },
            ],
        )? {
            write_frame(&mut server, &frame).await?;
        }
        Ok::<(), Error>(())
    });
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&id(4))?,
            admission_material: b"grant",
            client_build_id: "n07d-minimap-test",
            supported_capabilities: &[CAPABILITY_WORLD_MAP_VIEW_V1],
            deadline: Duration::from_secs(5),
        },
    )
    .await
    .map_err(|error| format!("admit: {error:?}"))?;
    peer.await??;
    session
        .world_map_snapshot()
        .ok_or_else(|| "admitted session exposes no map snapshot".into())
}

fn replay() -> Result<(), Error> {
    let (payload, expected) = captured()?;
    let revision: u64 = value(&expected, "revision")?.parse()?;
    let map = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(admit(payload, revision))?;
    let origin: Vec<i32> = value(&expected, "origin")?
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    let [x, y, floor] = origin[..] else {
        return Err("origin needs x,y,floor".into());
    };
    assert_eq!(
        (map.header.origin.x, map.header.origin.y, i32::from(map.header.origin.floor)),
        (x, y, floor)
    );
    assert_eq!(map.header.reset_epoch.to_string(), value(&expected, "reset_epoch")?);
    assert_eq!(map.tiles.len().to_string(), value(&expected, "tiles")?);

    // The server's walkable ground becomes remembered exploration, and only that.
    let mut minimap = Minimap::default();
    minimap.observe(&map);
    assert_eq!(minimap.cells.len().to_string(), value(&expected, "walkable")?);
    let remembered = minimap.cells.clone();
    minimap.observe(&map);
    assert_eq!(minimap.cells, remembered);

    // The tracked view is centred on the server-reported position.
    let floor = i16::try_from(floor)?;
    minimap.follow(x, y, floor);
    assert_eq!(minimap.position(15, 12), Some((x, y, floor)));

    // Markers are edited at that position; an invalid edit leaves them unchanged.
    minimap.edit_marker(x, y, floor, "Depot".into(), 2)?;
    let marker = Marker {
        x,
        y,
        floor,
        label: "Depot".into(),
        color: 2,
    };
    assert_eq!(minimap.markers.first(), Some(&marker));
    assert_eq!(minimap.markers.len(), 1);
    minimap.edit_marker(x, y, floor, "Temple".into(), 1)?;
    assert_eq!(
        minimap.markers,
        [Marker {
            label: "Temple".into(),
            color: 1,
            ..marker
        }]
    );
    let kept = minimap.markers.clone();
    assert!(minimap.edit_marker(x, y, 5, "Above ground".into(), 0).is_err());
    assert!(minimap.edit_marker(x, y, floor, "  ".into(), 0).is_err());
    assert_eq!(minimap.markers, kept);
    minimap.remove_marker(x, y, floor)?;
    assert!(minimap.markers.is_empty());
    assert_eq!(minimap.cells, remembered);
    Ok(())
}

#[test]
fn e2e_n07d_replays_admitted_map_into_the_explored_minimap() -> Result<(), Error> {
    std::thread::Builder::new()
        .name("n07d-minimap".into())
        .stack_size(64 << 20)
        .spawn(|| replay().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "N07d minimap test thread panicked")??;
    Ok(())
}
