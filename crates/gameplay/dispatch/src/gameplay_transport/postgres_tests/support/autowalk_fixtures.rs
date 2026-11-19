use super::fixture::debug;

use super::TestResult;

use crate::foundation::MovementLocalPosition;

use crate::gameplay_transport::connection::AdmittedSession;

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::map::overlay::TilePos;

use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

use oteryn_protocol_oteryn::{
    ClientCommandValue, MessageType, decode_command_result,
    decode_state_delta, decode_wire_envelope, encode_client_command,
};

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

pub(super) const DOOR: &str = "oteryn:item.tibia.i1629";

pub(super) const COIN: &str = "oteryn:item.tibia.i3031";

pub(super) fn pos(x: u16, y: u16) -> TilePos {
    TilePos { x, y, floor: -7 }
}

pub(super) fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

pub(super) fn bundle(door: u32, coin: u32) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let manifest = Manifest {
        subareas: Default::default(),
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![
            PaletteEntry {
                key: "terrain:grass".into(),
                family: Family::Terrain,
                id: 0,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                    blocks_projectile: None,
                }),
            },
            PaletteEntry {
                key: "terrain:wall".into(),
                family: Family::Terrain,
                id: 1,
                terrain: Some(Terrain {
                    kind: TerrainKind::Wall,
                    walkable: None,
                    ground_speed: None,
                    blocks_projectile: None,
                }),
            },
            PaletteEntry {
                key: DOOR.into(),
                family: Family::Item,
                id: door,
                terrain: None,
            },
            PaletteEntry {
                key: COIN.into(),
                family: Family::Item,
                id: coin,
                terrain: None,
            },
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 0..=6 {
        for x in 0..=14 {
            let mut palettes = vec![0];
            // One finite wall forces a detour; a second separates an unreachable room.
            if (x == 3 && y <= 2) || x == 9 {
                palettes.push(1);
            }
            if ((x == 6 || x == 12) && y == 1) || (x, y) == (1, 6) {
                palettes.push(2);
            }
            if (x, y) == (1, 5) || (x, y) == (7, 5) {
                palettes.push(3);
            }
            tiles.push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: palettes
                    .into_iter()
                    .map(|palette| Item {
                        palette,
                        depth: 0,
                        attrs: Attrs::default(),
                    })
                    .collect(),
            });
        }
    }
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
    ))
}

pub(super) fn object(
    digest: [u8; 32],
    pos: TilePos,
    revision: u64,
) -> TestResult<WorldObjectTarget> {
    let mut placement = digest.to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(pos, 1)
            .ok_or("placement")?
            .to_be_bytes(),
    );
    Ok(WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
}

pub(super) async fn send(
    client: &mut DuplexStream,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult {
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload,
        },
    )?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    Ok(())
}

pub(super) async fn read(client: &mut DuplexStream) -> TestResult<Vec<u8>> {
    let len = tokio::time::timeout(Duration::from_secs(20), client.read_u32()).await??;
    let mut frame = vec![0; usize::try_from(len)?];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

pub(super) async fn result(client: &mut DuplexStream, id: u64) -> TestResult<(Vec<u8>, usize)> {
    let mut steps = 0;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == 1
        {
            steps += 1;
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            return Ok((result.payload.to_vec(), steps));
        }
    }
}

pub(super) async fn place(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let mut movement = runtime.borrow_movement_position();
    loop {
        let before = movement
            .read(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?;
        let mut at = before.position();
        if at == destination {
            return Ok(());
        }
        if at.x != destination.x {
            at.x += (destination.x - at.x).signum();
        } else {
            at.y += (destination.y - at.y).signum();
        }
        movement.commit_cardinal(before, at).map_err(debug)?;
    }
}
