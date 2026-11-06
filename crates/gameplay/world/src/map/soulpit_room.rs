//! Exact Crystal Soulpit geometry, booted through the production bundle reader.
//! The normalized capture and its generator retain source custody; no donor map is shipped.
use super::{
    BundlePins,
    boot::{self, BootPins},
    facts::{ItemDefinition, TerrainPhysics},
    overlay::TilePos,
};
use crate::content::{ActiveGeneration, NativeEntryStart, QualifiedNativeEntryRoom};
use crate::foundation::{ChannelId, WorldId};
use oteryn_world_bundle::{
    bundle::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    },
    sector::{Attrs, Item, Tile},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

mod pin {
    include!("soulpit_capture_pin.rs");
}
const MAP_ONLY: [u32; 47] = [
    11075, 11077, 11079, 11102, 11103, 11104, 11105, 33017, 34943, 34946, 34951, 34953, 42880,
    42887, 42892, 42893, 42894, 42895, 42896, 42897, 42898, 42899, 42900, 42901, 42902, 42910,
    42923, 42933, 43516, 46629, 46630, 46631, 46632, 46633, 46634, 46635, 46636, 46637, 46638,
    46639, 46640, 46641, 46643, 46644, 46645, 46646, 48236,
];
const CAPTURE: &[u8] =
    include_bytes!("../../../../../tools/content-schema/native-gameplay/crystal-soulpit-room.json");
#[derive(Deserialize)]
struct Capture {
    schema: String,
    palette_columns: Vec<String>,
    tile_columns: Vec<String>,
    palette: Vec<Palette>,
    tiles: Vec<Placement>,
}
#[derive(Deserialize)]
#[serde(from = "(u32, bool, u16, bool, bool, bool, bool)")]
struct Palette {
    item: u32,
    ground: bool,
    ground_speed: u16,
    solid: bool,
    projectile: bool,
    pickupable: bool,
    door: bool,
}
#[derive(Deserialize)]
#[serde(from = "(u16, u16, i8, u32, Vec<u32>)")]
struct Placement {
    x: u16,
    y: u16,
    floor: i8,
    flags: u32,
    items: Vec<u32>,
}

impl From<(u32, bool, u16, bool, bool, bool, bool)> for Palette {
    fn from(row: (u32, bool, u16, bool, bool, bool, bool)) -> Self {
        let (item, ground, ground_speed, solid, projectile, pickupable, door) = row;
        Self {
            item,
            ground,
            ground_speed,
            solid,
            projectile,
            pickupable,
            door,
        }
    }
}
impl From<(u16, u16, i8, u32, Vec<u32>)> for Placement {
    fn from(row: (u16, u16, i8, u32, Vec<u32>)) -> Self {
        let (x, y, floor, flags, items) = row;
        Self {
            x,
            y,
            floor,
            flags,
            items,
        }
    }
}

/// Build the dedicated immutable room under the current native definition generation.
/// `base_room` is the qualified source entry room, before the origin bundle view is attached.
/// The returned collision port retains the tiny bundle's map and tile facts by `Arc`.
pub fn qualify_soulpit_room(
    base_room: &QualifiedNativeEntryRoom,
    active: &ActiveGeneration,
    world: WorldId,
    channel: ChannelId,
) -> Result<QualifiedNativeEntryRoom, String> {
    let source = source_world(active, world, channel)?;
    let cells = source
        .movement_cells(base_room.movement_cells())
        .map_err(|e| e.to_string())?;
    base_room
        .bundle_view(
            cells,
            source.frame_binding_digest(world),
            source.map_revision_digest(),
            NativeEntryStart {
                x: 32373,
                y: 31151,
                floor: 8,
            },
        )
        .ok_or_else(|| "Soulpit source room binding".into())
}

fn source_world(
    active: &ActiveGeneration,
    world: WorldId,
    channel: ChannelId,
) -> Result<boot::BundleWorld, String> {
    let hash: String = Sha256::digest(CAPTURE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if hash != pin::CAPTURE_SHA256 {
        return Err("Soulpit source capture digest".into());
    }
    let capture: Capture = serde_json::from_slice(CAPTURE).map_err(|e| e.to_string())?;
    if capture.schema != "OTERYN_SOULPIT_ROOM_CAPTURE/v2"
        || capture.palette_columns
            != [
                "item",
                "ground",
                "ground_speed",
                "solid",
                "projectile",
                "pickupable",
                "door",
            ]
        || capture.tile_columns != ["x", "y", "floor", "flags", "items"]
        || capture.tiles.len() != 579
        || capture.palette.len() != 118
    {
        return Err("Soulpit source capture shape".into());
    }
    let index = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("Soulpit native item index")?;
    let mut palette = Vec::with_capacity(capture.palette.len());
    let mut definitions = BTreeMap::new();
    let mut positions = BTreeMap::new();
    let mut terrain_physics = BTreeMap::new();
    // The map facts index is shared by both palette families. Allocate local Terrain
    // ids around this room's native Item ids, without changing either namespace.
    let native_ids = capture
        .palette
        .iter()
        .filter_map(|item| {
            let key = format!("oteryn:item.tibia.i{}", item.item);
            let revision = index.revision(&key)?;
            Some(index.definition_ref("Item", &key, revision)?.get() - 1)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut terrain_id = 0_u32;
    for (offset, item) in capture.palette.iter().enumerate() {
        let item_key = format!("oteryn:item.tibia.i{}", item.item);
        let (key, family, id) = if let Some(revision) = index.revision(&item_key) {
            let reference = index
                .definition_ref("Item", &item_key, revision)
                .ok_or("Soulpit Item reference")?;
            definitions.insert(
                item_key.clone(),
                ItemDefinition {
                    reference,
                    solid: Some(item.solid),
                    blocks_projectile: item.projectile,
                    pickupable: item.pickupable,
                    door: item.door,
                },
            );
            (item_key, Family::Item, reference.get() - 1)
        } else {
            if !MAP_ONLY.contains(&item.item) || item.pickupable || item.door {
                return Err(format!(
                    "Soulpit unqualified map-only appearance {}",
                    item.item
                ));
            }
            let key = format!("oteryn:terrain.tibia.i{}", item.item);
            terrain_physics.insert(
                key.clone(),
                TerrainPhysics {
                    solid: item.solid,
                    blocks_projectile: item.projectile,
                },
            );
            while native_ids.contains(&terrain_id) {
                terrain_id = terrain_id.checked_add(1).ok_or("Soulpit Terrain id")?;
            }
            let id = terrain_id;
            terrain_id = terrain_id.checked_add(1).ok_or("Soulpit Terrain id")?;
            (key, Family::Terrain, id)
        };
        positions.insert(item.item, u32::try_from(offset).map_err(|e| e.to_string())?);
        let terrain = if item.ground {
            Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(!item.solid),
                ground_speed: Some(item.ground_speed),
            })
        } else if family == Family::Terrain {
            Some(Terrain {
                kind: TerrainKind::Common,
                walkable: None,
                ground_speed: None,
            })
        } else {
            None
        };
        palette.push(PaletteEntry {
            key,
            family,
            id,
            terrain,
        });
    }
    let mut sectors: BTreeMap<(i8, u16, u16), Vec<Tile>> = BTreeMap::new();
    for tile in capture.tiles {
        if tile.floor != 8 {
            return Err("Soulpit captured floor".into());
        }
        let items = tile
            .items
            .into_iter()
            .map(|id| {
                Ok(Item {
                    palette: *positions.get(&id).ok_or("Soulpit captured palette")?,
                    depth: 0,
                    attrs: Attrs::default(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        sectors
            .entry((-tile.floor, tile.y / 32, tile.x / 32))
            .or_default()
            .push(Tile {
                x: tile.x,
                y: tile.y,
                flags: tile.flags,
                house: 0,
                zones: Vec::new(),
                items,
            });
    }
    let sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let manifest = Manifest {
        subareas: Default::default(),
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        compiler_version: "oteryn:soulpit-source-template/v1".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: hash.clone(),
            ..Default::default()
        },
        world: Extent {
            min_x: 32362,
            min_y: 31132,
            max_x: 32391,
            max_y: 31163,
            floors: vec![-8],
        },
        palette,
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports: Vec::new(),
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())
        .map_err(|e| e.to_string())?;
    let digest = bundle::read(&bytes).map_err(|e| e.to_string())?.digest;
    let pins = BootPins {
        bundle: BundlePins {
            digest,
            project_format_version: manifest.identity.project_format_version,
            world_schema_version: manifest.identity.world_schema_version,
            content_revision: hash,
            production: true,
        },
        map_revision: format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        start: TilePos {
            x: 32373,
            y: 31151,
            floor: -8,
        },
    };
    let source = boot::check(bytes, pins)
        .map_err(|e| e.to_string())?
        .boot_with_terrain(
            world,
            channel,
            |key| definitions.get(key).copied(),
            &|key| terrain_physics.get(key).copied(),
        )
        .map_err(|e| e.to_string())?;
    Ok(source)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::unreachable,
        reason = "tests use admitted content and fixed identifiers"
    )]
    use super::*;
    use crate::content::{CollisionClass, LogicalCell};
    use crate::map::boot::BundleCollisionIndexExt;
    #[test]
    fn captured_soulpit_boots_real_source_tiles_and_bounds_native_views() {
        let id = [0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
        let world = WorldId::decode(&id).unwrap();
        let channel = ChannelId::decode(&id).unwrap();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().unwrap();
        let room = qualify_soulpit_room(pin.qualified_room(), active, world, channel).unwrap();
        let template =
            crate::content::bosses::soulpit_template::QualifiedSoulpitRoomTemplate::qualify(
                &room, active,
            )
            .unwrap();
        let cells = template.room().movement_cells();
        for at in template.entry_positions() {
            assert_eq!(
                cells.index().lookup(cells.scope(), *at),
                Ok(CollisionClass::Walkable)
            );
            assert!(matches!(
                cells
                    .spell_tiles()
                    .lookup(cells.scope(), *at)
                    .unwrap()
                    .ground_source_id(),
                Some(46631 | 46633 | 46634)
            ));
        }
        let outside = LogicalCell {
            x: 32361,
            y: 31144,
            z: 8,
        };
        assert!(cells.index().lookup(cells.scope(), outside).is_err());
        assert!(cells.spell_tiles().lookup(cells.scope(), outside).is_err());
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            unreachable!()
        };
        // Native map rendering must retain its actual map owner through the bounded port.
        let _ = index.map();
        assert!(template.room().spawn_source().is_none());
    }
    #[test]
    fn map_only_appearances_retain_physics_without_inventory_definitions() {
        use crate::map::view::MapFacts;
        use oteryn_protocol_oteryn::world_map::MapDefinition;
        let id = [0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
        let world = WorldId::decode(&id).unwrap();
        let channel = ChannelId::decode(&id).unwrap();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().unwrap();
        let index = active.native_gameplay().unwrap().item_index().unwrap();
        for source in MAP_ONLY {
            assert!(
                index
                    .revision(&format!("oteryn:item.tibia.i{source}"))
                    .is_none()
            );
        }
        let source = source_world(active, world, channel).unwrap();
        assert_eq!(source.base().tile_count(), 579);
        let cells = source
            .movement_cells(pin.qualified_room().movement_cells())
            .unwrap();
        let capture: Capture = serde_json::from_slice(CAPTURE).unwrap();
        let palette = capture
            .palette
            .into_iter()
            .map(|entry| (entry.item, entry))
            .collect::<BTreeMap<_, _>>();
        let mut terrain = std::collections::BTreeSet::new();
        let mut native = std::collections::BTreeSet::new();
        let mut transparent_solids = std::collections::BTreeSet::new();
        for placement in capture.tiles {
            let pos = TilePos {
                x: placement.x,
                y: placement.y,
                floor: -placement.floor,
            };
            let tile = source.base().tile(pos.x, pos.y, pos.floor).unwrap();
            assert_eq!(tile.ids().len(), placement.items.len());
            for (ordinal, (&id, appearance)) in tile.ids().iter().zip(placement.items).enumerate() {
                let facts = source.facts().base_entry(pos, ordinal as u8, id).unwrap();
                let expected = &palette[&appearance];
                assert_eq!(u32::from(facts.appearance_id), appearance);
                assert_eq!(facts.blocks_projectile, expected.projectile);
                assert_eq!(facts.pickupable, expected.pickupable);
                if MAP_ONLY.contains(&appearance) {
                    assert!(
                        matches!(facts.definition, MapDefinition::Terrain(reference) if reference.get() == id + 1)
                    );
                    assert!(!facts.pickupable);
                    terrain.insert(appearance);
                    if expected.solid && !expected.projectile {
                        transparent_solids.insert(appearance);
                    }
                } else {
                    assert!(matches!(facts.definition, MapDefinition::Item(_)));
                    native.insert(appearance);
                }
            }
        }
        assert_eq!(terrain.len(), 47);
        assert_eq!(native.len(), 71);
        assert_eq!(transparent_solids.len(), 8);
        let mut qualified = 0;
        for x in 32362..=32390 {
            for y in 31132..=31153 {
                let at = LogicalCell { x, y, z: 8 };
                if cells.index().lookup(cells.scope(), at) == Ok(CollisionClass::Walkable)
                    && cells
                        .spell_tiles()
                        .lookup(cells.scope(), at)
                        .is_ok_and(|tile| !tile.flags().block_projectile)
                {
                    qualified += 1;
                }
            }
        }
        assert_eq!(qualified, 163);
    }
}
