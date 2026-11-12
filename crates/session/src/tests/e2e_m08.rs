//! M08 client replay: the owning reducer is `crates/session/src/map_transport.rs`.
//! `validate_join` admits the map snapshot, `apply` validates/decodes the terrain
//! delta, and `Session::world_map_snapshot` exposes the admitted client map.
use super::*;
use oteryn_protocol_oteryn::world_map::*;

#[test]
fn e2e_m08_replays_terrain_transform_and_rejects_stale_delta() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(replay()))
        .map_err(|_| "M08 replay thread spawn failed")?
        .join()
        .map_err(|_| "M08 replay thread failed")??
}

async fn replay() -> Result<(), BoxError> {
    let (snapshot_payload, delta_payload, revisions, expected) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("m08-snapshot.bin"))?,
                std::fs::read(path.join("m08-delta.bin"))?,
                std::fs::read_to_string(path.join("m08-revisions.txt"))?,
                std::fs::read_to_string(path.join("m08-expected.txt"))?,
            )
        } else {
            fixture()?
        };
    let numbers = revisions
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    if numbers.len() != 4 {
        return Err("invalid M08 capture revision metadata".into());
    }
    let (snapshot_revision, base_revision, new_revision, server_sequence) =
        (numbers[0], numbers[1], numbers[2], numbers[3]);
    let expected = expected
        .split_whitespace()
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [x, y, floor, ordinal, appearance] = expected.as_slice() else {
        return Err("invalid M08 expected metadata".into());
    };
    let snapshot = decode_world_map_snapshot(&snapshot_payload).map_err(|e| format!("{e:?}"))?;
    let content = snapshot.header.content_generation;
    let spatial = oteryn_protocol_oteryn::world_spatial::encode_world_spatial(
        &oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
            content_generation: content,
            actor_position: snapshot.header.origin,
        },
    );
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let selected: &'static [u32] = &[CAPABILITY_WORLD_MAP_VIEW_V1];
    let snapshot_for_peer = snapshot_payload.clone();
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            selected,
            selected,
            &[],
            (1, spatial),
            &[(
                STATE_DOMAIN_WORLD_MAP_VIEW,
                snapshot_revision,
                SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                snapshot_for_peer,
            )],
        )
        .await?;
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    let mut session = Session::admit(client, admission_with(selected)?)
        .await
        .map_err(|e| format!("admit: {e:?}"))?;
    assert_eq!(session.world_map_snapshot(), Some(snapshot));
    let wire = encode_state_delta(
        1,
        server_sequence,
        STATE_DOMAIN_WORLD_MAP_VIEW,
        base_revision,
        new_revision,
        DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
        &delta_payload,
    )?;
    let payload = decode_wire_envelope(&wire)?.payload().to_vec();
    let event = session
        .apply_state_delta(server_sequence, &payload)
        .map_err(|e| format!("apply: {e:?}"))?;
    let SessionEvent::WorldMap(applied) = event else {
        return Err("M08 delta did not produce a world-map event".into());
    };
    let position = ActorPosition {
        x: i32::try_from(*x)?,
        y: i32::try_from(*y)?,
        floor: -i16::try_from(*floor)?,
    };
    assert!(applied.value.tiles.iter().any(|tile| {
        tile.position == position
            && tile.items.iter().any(|item| {
                item.appearance_id == u16::try_from(*appearance).unwrap_or_default()
                    && matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: value, .. } if value == u8::try_from(*ordinal).unwrap_or_default())
            })
    }));
    let binding = session.map_header;
    assert!(
        session
            .apply_state_delta(server_sequence + 1, &payload)
            .is_err()
    );
    assert_eq!(session.map_header, binding);
    drop(session);
    peer.await??;
    Ok(())
}

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String, String), BoxError> {
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let header = MapViewHeader {
        content_generation: CONTENT_GENERATION,
        bundle_digest: [0x22; 32],
        reset_epoch: 3,
        origin: position,
        first_visible_floor: first_visible_floor_start(position.floor),
    };
    let snapshot = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header,
        tiles: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    let delta = encode_world_map_delta(
        &header,
        &WorldMapViewDelta {
            header,
            tiles: vec![MapTile {
                position,
                more: false,
                ground_speed: 0,
                items: vec![MapItem {
                    appearance_id: 594,
                    count: 1,
                    sub_type: 0,
                    definition: MapDefinition::Item(
                        std::num::NonZeroU32::new(1).ok_or("terrain definition")?,
                    ),
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 2,
                        object_revision: 1,
                    },
                }],
            }],
            cleared: vec![],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok((
        snapshot,
        delta,
        "4 4 5 41".into(),
        "100 200 7 2 594\n".into(),
    ))
}
