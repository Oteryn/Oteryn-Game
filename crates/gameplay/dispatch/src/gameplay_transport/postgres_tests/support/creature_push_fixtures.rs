use super::autowalk_fixtures::{read, send};

use super::fixture::debug;

use super::TestResult;

use crate::foundation::MovementLocalPosition;

use oteryn_protocol_oteryn::creature_push::{CreaturePushIntent, encode_creature_push_intent};

use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use oteryn_protocol_oteryn::{MessageType, decode_command_result, decode_wire_envelope};

use tokio::io::DuplexStream;

/// Grass 0..=12 x 0..=8 on floor 7 (Tibia-keyed, so every tile has spell facts); a wall at (5, 2); every tile with x >= 8 is a protection zone.
pub(super) fn bundle(trap: Option<u16>) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let terrain = |key: &str, id, kind, walkable: Option<bool>| PaletteEntry {
        key: key.into(),
        family: Family::Terrain,
        id,
        terrain: Some(Terrain {
            kind,
            walkable,
            ground_speed: walkable.map(|_| 150),
            blocks_projectile: None,
        }),
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
            terrain(
                "oteryn:terrain.tibia.i4526",
                0,
                TerrainKind::Ground,
                Some(true),
            ),
            terrain("oteryn:terrain.tibia.i1026", 1, TerrainKind::Wall, None),
            terrain(
                "oteryn:terrain.tibia.i5405",
                2,
                TerrainKind::Ground,
                Some(true),
            ),
            PaletteEntry {
                key: "oteryn:item.tibia.i3944".into(),
                family: Family::Item,
                id: 3,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i3945".into(),
                family: Family::Item,
                id: 4,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i12368".into(),
                family: Family::Item,
                id: 5,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i12369".into(),
                family: Family::Item,
                id: 6,
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
    for y in 0..=8 {
        for x in 0..=12 {
            let mut palettes = vec![if (x, y) == (3, 5) { 2 } else { 0 }];
            if let Some(trap) = trap
                && (x, y) == (4, 2)
            {
                palettes.push(if trap == 12368 { 5 } else { 3 });
            }
            if (x, y) == (5, 2) {
                palettes.push(1);
            }
            tiles.push(Tile {
                x,
                y,
                // Tile flag bit 0 is the protection zone (map/spell_tiles.rs).
                flags: u32::from(x >= 8),
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

/// Sends one push and returns its disposition, reading past spatial updates. Like a real
/// client it names the floor in the wire frame the visibility projection exported.
pub(super) async fn push(
    client: &mut DuplexStream,
    id: u64,
    target: EntityRef,
    to: MovementLocalPosition,
) -> TestResult<u8> {
    let payload = encode_creature_push_intent(&CreaturePushIntent {
        target,
        destination: ActorPosition {
            x: to.x,
            y: to.y,
            floor: to.floor,
        },
    })
    .map_err(debug)?;
    raw(client, id, &payload).await
}

pub(super) async fn raw(client: &mut DuplexStream, id: u64, payload: &[u8]) -> TestResult<u8> {
    send(client, id, 98, payload).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            let [8, disposition] = result.payload[..] else {
                return Err(format!("result payload {:?}", result.payload).into());
            };
            return Ok(disposition);
        }
    }
}
