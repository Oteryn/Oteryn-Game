//! BUNDLE-TILES-1: the spell tile facts (`OTERYN_NATIVE_SPELL_TILES/v1` semantics,
//! [`QualifiedSpellTile`]) of a World booted from its world bundle, derived at boot from facts the
//! format v4 bundle and the served item definitions already state. There is no new bundle field
//! and no new format version.
//!
//! Each fact has one source; a fact without one is never guessed:
//!
//! - **Ground.** The base's ground item ([`WorldBase`], §1.3: the first top-level entry whose
//!   palette `terrain` is a `ground` record): present or not, its stored `ground_speed` and its
//!   source id. Server pacing still goes through the one switch point,
//!   [`super::boot::BundleWorld::ground_speed`].
//! - **Source ids.** The Tibia id of the palette key ([`super::palette_appearance`]). The
//!   generation's Item profiles bind `ots/item_server_id` to the same digits as the key (for
//!   example `oteryn:item.tibia.i1629` and external id `1629` in
//!   `tools/content-schema/native-gameplay/door-item-profiles.json`). `top_source_ids` are the
//!   top-level entries other than the ground, in payload order.
//! - **`block_solid`.** A non-walkable ground, or a tile the boot blocks: a `wall` Terrain, an
//!   Item whose definition is solid or does not say (unless the Item key is routed to a `ground`
//!   Terrain record, or to a `border` the terrain catalogue proves walkable), or a sealed door ([`super::facts::BundleFacts::push`]).
//! - **`block_projectile`.** A `wall` Terrain, or a top-level Item whose served definition blocks
//!   projectiles; a definition that does not say blocks (`boot_bundle_items` in `node/serve.rs`).
//! - **`floor_change`.** A top-level entry with a catalogue `floor_change` fact
//!   ([`super::floor::palette_mask`], MAP-FLOOR-1). The rope spots (ground 386 and 21965) carry
//!   no such fact: the genuine Canary capture (`canary-thalom-world.json`, item policy 386) records them
//!   as walkable ground with `floor_change` false, and the rope spell matches them by ground id.
//! - **`protection_zone`.** Tile flag bit 0 (format document, OTBM `PROTECTIONZONE`).
//! - **`immovable_block_solid`, `immovable_block_item`, `immovable_nonfield_block_item`.** No
//!   source: the bundle and the served definitions state neither `movable` nor `block_pathfind`.
//!   They fail closed as `true`, so rope, levitate and house-kick relocation refuse every bundle
//!   tile as a destination. Follow-up BUNDLE-TILES-IMMOVABLE-1 sources them.
//! - **PvP zones.** Retained OTBM tile flags, using the pinned Canary loader's priority:
//!   protection zone, then NoPvP (bit 2), then PvP (bit 4). ADMIT-CAP-2.
//! - **Source step facts** (height, floor-change kinds) stay unknown
//!   ([`QualifiedSpellTile::source_step`] is `None`).
//!
//! A tile is **unknown** (`SpellTileLookupError::Unknown`, so a cast or attack on it is refused)
//! when one of its top-level entries has no source id, when it holds a pickupable entry or a
//! usable door (state the Channel changes at runtime, as mutable placements of the source world
//! are unknown), or when the boot did not visit it. A position without a bundle tile is absent.
//!
//! Any Canary or Crystal semantics beyond the repository documents and captures named above are
//! OTS_HYPOTHESIS_ONLY.

use super::WorldBase;
use super::door::DoorBook;
use super::facts::ItemDefinition;
use super::overlay::TilePos;
use super::view::placement_of;
use crate::content::{NativeSpellTileFlags, QualifiedSpellTile};
#[cfg(any(test, feature = "test-support"))]
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_world_bundle::bundle::{Family, Manifest, Sector, TerrainKind};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

pub use crate::content::bundle_spell_tiles::BundleSpellTiles;
use crate::content::bundle_spell_tiles::UNKNOWN;

impl crate::content::bundle_spell_tiles::BundleTileBase for WorldBase {
    fn digest(&self) -> [u8; 32] {
        self.digest()
    }

    fn tile_index(&self, x: u16, y: u16, floor: i8) -> Option<usize> {
        self.tile_index(x, y, floor)
    }
}

/// OTBM `PROTECTIONZONE`, tile flag bit 0.
const PROTECTION_ZONE: u32 = 1;

// Canary 99902524e052f37574194466c2949c576e4ab269:
// src/io/io_definitions.hpp:73-76; src/io/iomap.cpp:167-173.
// Bundle format §5 / compiler equivalence retain the source tile flags unchanged.
fn pvp_zones(flags: u32) -> (bool, bool) {
    let no_pvp = flags & PROTECTION_ZONE == 0 && flags & (1 << 2) != 0;
    let pvp = flags & PROTECTION_ZONE == 0 && !no_pvp && flags & (1 << 4) != 0;
    (no_pvp, pvp)
}

/// The spell tile facts of one palette entry.
#[derive(Debug, Clone, Copy)]
struct PaletteSpell {
    source_id: Option<u32>,
    wall: bool,
    blocks_projectile: bool,
    pickupable: bool,
}

/// The distinct facts of a tile, interned once.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Shape {
    ground: Option<(u32, u16)>,
    tops: Vec<u32>,
    block_solid: bool,
    block_projectile: bool,
    floor_change: bool,
    protection_zone: bool,
    pvp_zones: (bool, bool),
}

impl Shape {
    fn tile(&self) -> QualifiedSpellTile {
        QualifiedSpellTile::from_bundle(
            self.ground,
            self.tops.clone(),
            NativeSpellTileFlags {
                block_solid: self.block_solid,
                block_projectile: self.block_projectile,
                // No source (BUNDLE-TILES-IMMOVABLE-1): the blocking value refuses relocation.
                immovable_block_solid: true,
                immovable_block_item: true,
                immovable_nonfield_block_item: true,
                floor_change: self.floor_change,
                protection_zone: self.protection_zone,
            },
            Some(self.pvp_zones),
        )
    }
}

/// Collects the spell tile facts of a bundle's sectors during the boot scan.
#[derive(Debug)]
pub struct SpellTilesBuilder {
    base: Arc<WorldBase>,
    palette: Option<Vec<PaletteSpell>>,
    tiles: Vec<u32>,
    shapes: Vec<QualifiedSpellTile>,
    interned: HashMap<Shape, u32>,
    visited: usize,
    /// A visited tile the base does not hold, or a shape count past `u32`.
    inconsistent: bool,
}

impl SpellTilesBuilder {
    pub fn new(base: Arc<WorldBase>) -> Self {
        let tiles = vec![UNKNOWN; base.tile_count()];
        Self {
            base,
            palette: None,
            tiles,
            shapes: Vec::new(),
            interned: HashMap::new(),
            visited: 0,
            inconsistent: false,
        }
    }

    /// Records the tiles of one sector of `manifest`. `blocked` holds the tiles the boot blocks
    /// (at least those of this sector), `item` serves the Item definitions and `masks` holds the
    /// catalogue floor-change mask of each palette entry.
    pub fn push(
        &mut self,
        manifest: &Manifest,
        sector: &Sector,
        blocked: &BTreeSet<TilePos>,
        item: &impl Fn(&str) -> Option<ItemDefinition>,
        masks: &[u8],
    ) {
        let palette = self.palette.get_or_insert_with(|| {
            manifest
                .palette
                .iter()
                .map(|entry| {
                    let definition = (entry.family == Family::Item)
                        .then(|| item(&entry.key))
                        .flatten();
                    PaletteSpell {
                        source_id: super::palette_appearance(&entry.key).map(u32::from),
                        wall: entry
                            .terrain
                            .is_some_and(|terrain| terrain.kind == TerrainKind::Wall),
                        blocks_projectile: definition
                            .is_none_or(|definition| definition.blocks_projectile),
                        pickupable: definition.is_some_and(|definition| definition.pickupable),
                    }
                })
                .collect()
        });
        for tile in &sector.tiles {
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            let Some(index) = self.base.tile_index(pos.x, pos.y, pos.floor) else {
                self.inconsistent = true;
                continue;
            };
            let Some(view) = self.base.tile(pos.x, pos.y, pos.floor) else {
                self.inconsistent = true;
                continue;
            };
            self.visited += 1;
            let ground_ordinal = view.ground().map(|(ordinal, _)| usize::from(ordinal));
            let mut ground = None;
            let mut tops = Vec::new();
            let mut known = true;
            let mut block_projectile = false;
            let mut floor_change = false;
            for (ordinal, entry) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let Some(facts) = palette.get(entry.palette as usize) else {
                    known = false;
                    break;
                };
                let Some(source_id) = facts.source_id else {
                    known = false;
                    break;
                };
                if facts.pickupable {
                    known = false;
                    break;
                }
                block_projectile |= facts.wall || facts.blocks_projectile;
                floor_change |= masks
                    .get(entry.palette as usize)
                    .is_some_and(|mask| *mask != 0);
                if Some(ordinal) == ground_ordinal {
                    ground = Some((source_id, view.stored_ground_speed()));
                } else {
                    tops.push(source_id);
                }
            }
            if !known || ground.is_some() != ground_ordinal.is_some() {
                continue;
            }
            let shape = Shape {
                ground,
                tops,
                block_solid: (ground.is_some() && !view.walkable()) || blocked.contains(&pos),
                block_projectile,
                floor_change,
                protection_zone: tile.flags & PROTECTION_ZONE != 0,
                pvp_zones: pvp_zones(tile.flags),
            };
            let id = match self.interned.get(&shape) {
                Some(id) => *id,
                None => {
                    let Some(id) = u32::try_from(self.shapes.len())
                        .ok()
                        .filter(|id| *id != UNKNOWN)
                    else {
                        self.inconsistent = true;
                        continue;
                    };
                    self.shapes.push(shape.tile());
                    self.interned.insert(shape, id);
                    id
                }
            };
            self.tiles[index] = id;
        }
    }

    /// The bundle's spell tiles: every tile with a usable door ([`DoorBook`], MAP-DOOR-1) is
    /// unknown, since its state changes at runtime. Refuses a scan that disagrees with the base.
    pub fn finish(
        mut self,
        doors: &DoorBook,
    ) -> Result<BundleSpellTiles, super::boot::BootRefusal> {
        if self.inconsistent || self.visited != self.base.tile_count() {
            return Err(super::LoadError::Bundle(oteryn_world_bundle::Error::Format(
                "spell tiles disagree with the world base".into(),
            ))
            .into());
        }
        for key in doors.keys() {
            if let Some((pos, _)) = placement_of(key)
                && let Some(index) = self.base.tile_index(pos.x, pos.y, pos.floor)
            {
                self.tiles[index] = UNKNOWN;
            }
        }
        self.shapes.shrink_to_fit();
        Ok(BundleSpellTiles {
            base: self.base,
            tiles: self.tiles,
            shapes: self.shapes,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests {
    #![cfg_attr(not(test), allow(dead_code, unused_imports))]
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::content::{LogicalCell, NativeEntryMovementCells, SpellTileLookupError};
    use crate::foundation::{ChannelId, WorldId};
    use crate::map::BundlePins;
    use crate::map::boot::{BootPins, BundleWorld, boot};
    use std::num::NonZeroU32;

    pub mod movement_arena;
    mod admit_cap_2 {
        include!("admit_cap_2_bundle_zones_tests.rs");
    }

    /// The rope spot ground items (`tools/content-schema/world-authoring/README.md`, *Floor
    /// changes*). Their ground id is carried as `ground_source_id`; they are not `floor_change`.
    const ROPE_SPOTS: [u32; 2] = [386, 21965];

    pub fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }

    /// Palette (all Item entries, as the compiler routes them): 0 grass `i100`, 1 water `i598`
    /// (not walkable), 2 wall `i1100`, 3 east stair `i1950`, 4 rope spot `i386`, 5 coin `i3031`
    /// (pickupable), 6 `item:box` (no source id), 7 `i2000` (no profile), 8 door `i1629`.
    const KEYS: [&str; 9] = [
        "oteryn:item.tibia.i100",
        "oteryn:item.tibia.i598",
        "oteryn:item.tibia.i1100",
        "oteryn:item.tibia.i1950",
        "oteryn:item.tibia.i386",
        "oteryn:item.tibia.i3031",
        "item:box",
        "oteryn:item.tibia.i2000",
        "oteryn:item.tibia.i1629",
    ];

    /// One row at legacy `z` 7 (native -7); see [`KEYS`] for the palette.
    fn bundle(custom: Option<Vec<oteryn_world_bundle::writer::Sector>>) -> (Vec<u8>, BundlePins) {
        use oteryn_world_bundle::sector::{Attrs, Item, Tile};
        use oteryn_world_bundle::writer::{
            self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
        let terrain = |kind, walkable: Option<bool>, speed: Option<u16>| {
            Some(Terrain {
                kind,
                walkable,
                ground_speed: speed,
            })
        };
        let terrains = [
            terrain(TerrainKind::Ground, Some(true), Some(150)),
            terrain(TerrainKind::Ground, Some(false), Some(0)),
            terrain(TerrainKind::Wall, None, None),
            None,
            terrain(TerrainKind::Ground, Some(true), Some(120)),
            None,
            None,
            None,
            None,
        ];
        let manifest = Manifest {
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
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
                max_x: if custom.is_some() { 128 } else { 32 },
                max_y: if custom.is_some() { 128 } else { 32 },
                floors: vec![-7],
            },
            palette: KEYS
                .iter()
                .zip(terrains)
                .enumerate()
                .map(|(id, (key, terrain))| PaletteEntry {
                    key: (*key).into(),
                    family: Family::Item,
                    id: id as u32,
                    terrain,
                })
                .collect(),
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
            npcs: Default::default(),
        };
        let tile = |x, flags, house, palettes: &[u32]| Tile {
            x,
            y: 0,
            flags,
            house,
            zones: Vec::new(),
            items: palettes
                .iter()
                .map(|palette| Item {
                    palette: *palette,
                    depth: 0,
                    attrs: Attrs::default(),
                })
                .collect(),
        };
        let sector = Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![
                tile(1, 0, 0, &[0]),
                tile(2, 1, 0, &[0]),
                tile(3, 0, 0, &[1]),
                tile(4, 0, 0, &[0, 2]),
                tile(5, 0, 0, &[0, 3]),
                tile(6, 0, 0, &[4]),
                tile(7, 0, 0, &[0, 5]),
                tile(8, 0, 0, &[0, 6]),
                tile(9, 0, 0, &[0, 7]),
                tile(10, 0, 0, &[7]),
                tile(11, 0, 0, &[0, 8]),
                tile(12, 0, 1, &[0, 8]),
                tile(13, 0, 0, &[0]),
            ],
        };
        let bytes = bundle::write(
            &manifest,
            &custom.unwrap_or_else(|| vec![sector]),
            &Default::default(),
        )
        .expect("written bundle");
        let digest = bundle::read(&bytes).expect("read bundle").digest;
        let pins = BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        };
        (bytes, pins)
    }

    /// The served definitions: `(solid, blocks_projectile, pickupable)` per [`KEYS`] entry, and
    /// the door's open item `i1630`. `i2000` has no profile, so the join's defaults apply.
    fn items(key: &str) -> Option<ItemDefinition> {
        if key == "oteryn:item.tibia.i1630" {
            return Some(ItemDefinition {
                reference: NonZeroU32::new(1631)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            });
        }
        let index = KEYS.iter().position(|known| *known == key)?;
        let (solid, blocks_projectile, pickupable) = match index {
            0 | 3 | 4 => (Some(false), false, false),
            1 => (Some(true), false, false),
            2 | 6 | 8 => (Some(true), true, false),
            5 => (Some(false), false, true),
            _ => (None, true, false),
        };
        Some(ItemDefinition {
            reference: NonZeroU32::new(u32::try_from(index).ok()? + 1)?,
            solid,
            blocks_projectile,
            pickupable,
            door: index == 8,
        })
    }

    /// The synthetic bundle World of these tests, started at x 1; x 2 is a protection zone.
    pub fn world() -> BundleWorld {
        world_from_bundle(bundle(None))
    }

    fn world_from_bundle((bytes, bundle): (Vec<u8>, BundlePins)) -> BundleWorld {
        let mut revision = String::from("sha256:");
        for byte in bundle.digest {
            revision.push_str(&format!("{byte:02x}"));
        }
        let pins = BootPins {
            bundle,
            map_revision: revision,
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        };
        boot(
            &bytes,
            &pins,
            WorldId::decode(&id(1)).unwrap(),
            ChannelId::decode(&id(2)).unwrap(),
            items,
        )
        .expect("the synthetic bundle boots")
    }

    fn cells(world: &BundleWorld) -> (NativeEntryMovementCells, NativeEntryMovementCells) {
        let room = crate::content::qualify_native_entry_room(WorldId::decode(&id(1)).unwrap())
            .expect("entry room");
        let cells = world
            .movement_cells(room.movement_cells())
            .expect("bundle cells");
        (room.movement_cells().clone(), cells)
    }

    fn at(x: i32) -> LogicalCell {
        LogicalCell { x, y: 0, z: 7 }
    }

    const fn flags(
        block_solid: bool,
        block_projectile: bool,
        floor_change: bool,
        protection_zone: bool,
    ) -> NativeSpellTileFlags {
        NativeSpellTileFlags {
            block_solid,
            block_projectile,
            immovable_block_solid: true,
            immovable_block_item: true,
            immovable_nonfield_block_item: true,
            floor_change,
            protection_zone,
        }
    }

    #[test]
    fn bundle_tiles_derive_each_fact_from_the_bundle_and_the_served_items() {
        let world = world();
        let (_, cells) = cells(&world);
        let tiles = cells.spell_tiles();
        let lookup = |x| tiles.lookup(cells.scope(), at(x));
        let tile = |ground, tops: &[u32], flags| {
            QualifiedSpellTile::from_bundle(ground, tops.to_vec(), flags, Some((false, false)))
        };
        // Grass: a walkable ground with its id and stored speed, nothing on top.
        let grass = lookup(1).expect("grass");
        assert_eq!(
            grass,
            &tile(Some((100, 150)), &[], flags(false, false, false, false))
        );
        assert!(grass.ground_present());
        assert_eq!(grass.ground_source_id(), Some(100));
        assert_eq!(grass.ground_speed(), Some(150));
        // Height/floor-change source facts remain unknown; retained flags qualify zones.
        assert_eq!(grass.source_step(), None);
        assert_eq!(grass.no_pvp_zone(), Some(false));
        assert_eq!(grass.pvp_zone(), Some(false));
        // Tile flag bit 0 is the protection zone.
        assert!(lookup(2).expect("zone").flags().protection_zone);
        // A non-walkable ground blocks.
        assert_eq!(
            lookup(3).expect("water"),
            &tile(Some((598, 0)), &[], flags(true, false, false, false))
        );
        // A wall blocks movement and projectiles.
        assert_eq!(
            lookup(4).expect("wall"),
            &tile(Some((100, 150)), &[1100], flags(true, true, false, false))
        );
        // A catalogue floor change.
        assert_eq!(
            lookup(5).expect("stair"),
            &tile(Some((100, 150)), &[1950], flags(false, false, true, false))
        );
        // An Item without a profile blocks movement and projectiles.
        assert_eq!(
            lookup(9).expect("unprofiled"),
            &tile(Some((100, 150)), &[2000], flags(true, true, false, false))
        );
        // No ground item: absent ground, unknown speed.
        let groundless = lookup(10).expect("groundless");
        assert!(!groundless.ground_present());
        assert_eq!(groundless.ground_source_id(), None);
        assert_eq!(groundless.ground_speed(), None);
        assert_eq!(groundless.top_source_ids(), &[2000]);
        // A sealed (house) door is a static wall.
        assert_eq!(
            lookup(12).expect("sealed door"),
            &tile(Some((100, 150)), &[1629], flags(true, true, false, false))
        );
    }

    #[test]
    fn bundle_spell_tiles_pvp_zone_precedence() {
        use oteryn_world_bundle::sector::{Attrs, Item, Tile};
        use oteryn_world_bundle::writer::Sector;

        // Keep zone-specific cells out of the shared World's boundary fixture.
        let cases = [
            (1, 0, false, false, false),
            (2, 4, false, true, false),
            (3, 16, false, false, true),
            (4, 5, true, false, false),
            (5, 17, true, false, false),
            (6, 20, false, true, false),
            (7, 8, false, false, false),
            (8, 9, true, false, false),
            (9, 12, false, true, false),
            (10, 24, false, false, true),
            (11, 29, true, false, false),
        ];
        let world = world_from_bundle(bundle(Some(vec![Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: cases
                .iter()
                .map(|&(x, flags, _, _, _)| Tile {
                    x,
                    y: 0,
                    flags,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                })
                .collect(),
        }])));
        let (_, cells) = cells(&world);
        for (x, flags, protection, no_pvp, pvp) in cases {
            let tile = cells
                .spell_tiles()
                .lookup(cells.scope(), at(i32::from(x)))
                .expect("zone tile");
            assert_eq!(tile.flags().protection_zone, protection, "x {x}");
            assert_eq!(tile.no_pvp_zone(), Some(no_pvp), "x {x}");
            assert_eq!(tile.pvp_zone(), Some(pvp), "x {x}");
            let base_tile = world
                .overlay()
                .base()
                .tile(x, 0, -7)
                .expect("base zone tile");
            assert_eq!(base_tile.protection_zone(), protection, "x {x}");
            assert_eq!(base_tile.no_logout(), flags & 8 != 0, "x {x}");
        }
    }

    #[test]
    fn bundle_tiles_without_a_source_are_unknown_and_outside_the_map_absent() {
        let world = world();
        let (entry, cells) = cells(&world);
        let tiles = cells.spell_tiles();
        let lookup = |cell| tiles.lookup(cells.scope(), cell);
        // A pickupable entry, a palette key without a source id and a usable door.
        for x in [7, 8, 11] {
            assert_eq!(lookup(at(x)), Err(SpellTileLookupError::Unknown), "x {x}");
        }
        assert!(world.facts().doors().at(TilePos {
            x: 11,
            y: 0,
            floor: -7
        }));
        // No tile there, a floor without tiles, and coordinates outside the native plane.
        for cell in [
            at(0),
            at(20),
            LogicalCell { x: 1, y: 0, z: 6 },
            LogicalCell { x: 1, y: 0, z: 16 },
            LogicalCell { x: -1, y: 0, z: 7 },
        ] {
            assert_eq!(lookup(cell), Err(SpellTileLookupError::Absent), "{cell:?}");
        }
        // The entry room's own scope, and its own tiles, do not open the bundle's cells.
        assert_eq!(
            tiles.lookup(entry.scope(), at(1)),
            Err(SpellTileLookupError::ScopeMismatch)
        );
        assert_eq!(
            entry.spell_tiles().lookup(cells.scope(), at(1)),
            Err(SpellTileLookupError::ScopeMismatch)
        );
    }

    #[test]
    fn bundle_rope_spots_keep_their_ground_id_and_change_no_floor() {
        let world = world();
        let (_, cells) = cells(&world);
        let rope = cells
            .spell_tiles()
            .lookup(cells.scope(), at(6))
            .expect("rope spot");
        assert_eq!(rope.ground_source_id(), Some(ROPE_SPOTS[0]));
        assert_eq!(rope.ground_speed(), Some(120));
        assert!(!rope.flags().floor_change);
        for id in ROPE_SPOTS {
            assert_eq!(
                super::super::floor::palette_mask(
                    Family::Item,
                    &format!("oteryn:item.tibia.i{id}")
                ),
                0,
                "rope spot {id}"
            );
        }
    }

    #[test]
    fn bundle_tiles_fail_closed_on_immovable_facts() {
        let world = world();
        let (_, cells) = cells(&world);
        let grass = cells
            .spell_tiles()
            .lookup(cells.scope(), at(1))
            .expect("grass")
            .flags();
        assert!(grass.immovable_block_solid);
        assert!(grass.immovable_block_item);
        assert!(grass.immovable_nonfield_block_item);
    }

    fn store(cells: &NativeEntryMovementCells) -> Arc<BundleSpellTiles> {
        match cells.index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                Arc::clone(&index.map().spell_tiles)
            }
            _ => panic!("bundle cells"),
        }
    }

    #[test]
    fn bundle_tiles_intern_equal_tiles_and_two_boots_are_equal() {
        let (first, second) = (world(), world());
        let (tiles, other) = (store(&cells(&first).1), store(&cells(&second).1));
        assert!(!Arc::ptr_eq(&tiles, &other));
        assert_eq!(tiles, other);
        assert_eq!(tiles.tiles.len(), first.base().tile_count());
        // 13 tiles: 3 unknown, and the grass at x 13 shares the shape of x 1. The coin and the box
        // intern no shape; the usable door's shape (not blocked, unlike the sealed door) stays
        // interned and its tile is unknown.
        assert_eq!(
            tiles
                .tiles
                .iter()
                .filter(|shape| **shape == UNKNOWN)
                .count(),
            3
        );
        assert_eq!(tiles.shapes.len(), 10);
        let (_, cells) = cells(&first);
        let lookup = |x| cells.spell_tiles().lookup(cells.scope(), at(x));
        assert!(std::ptr::eq(
            lookup(1).expect("x 1"),
            lookup(13).expect("x 13")
        ));
    }

    /// The served Item definitions of the active generation, joined as `boot_bundle_items` in
    /// `node/serve.rs` joins them.
    fn generation_item<'a>(
        state: &'a crate::content::native_gameplay::NativeGameplayState,
    ) -> impl Fn(&str) -> Option<ItemDefinition> + 'a {
        use crate::content::{ReferenceItemField, ReferenceItemType};
        move |key: &str| {
            let index = state.item_index()?;
            let revision = index.revision(key)?;
            let reference = index.definition_ref("Item", key, revision)?;
            let profile = state
                .item_policy(key, revision)
                .map(|policy| policy.record());
            let pickupable = profile.is_some_and(|profile| {
                matches!(
                    &profile.semantics.physical,
                    ReferenceItemField::Known(physical)
                        if physical.pickupable == ReferenceItemField::Known(true)
                )
            });
            let door = profile.is_some_and(|profile| {
                matches!(
                    &profile.semantics.classification,
                    ReferenceItemField::Known(classification)
                        if classification.item_type
                            == ReferenceItemField::Known(ReferenceItemType::Door)
                )
            });
            let attributes = profile.map(|profile| &profile.attributes);
            Some(ItemDefinition {
                reference,
                solid: attributes.and_then(|attributes| attributes.blocks_movement),
                blocks_projectile: attributes
                    .and_then(|attributes| attributes.blocks_projectile)
                    .unwrap_or(true),
                pickupable,
                door,
            })
        }
    }

    /// The committed World pin's real artifact (`pin-check` output, path in
    /// `OTERYN_PINNED_WORLD_BUNDLE`) boots with the generation's Item definitions and every tile
    /// of its base answers a spell tile lookup under the bundle scope.
    #[test]
    #[ignore = "needs the pinned artifact: OTERYN_PINNED_WORLD_BUNDLE=<pin-check .otwb>"]
    fn pinned_bundle_tiles_answer_every_base_tile() {
        const ITEM_KEYS: &[u8] =
            include_bytes!("../../../../../tools/content-schema/native-gameplay/item-keys.json");
        let path = std::env::var("OTERYN_PINNED_WORLD_BUNDLE").expect("OTERYN_PINNED_WORLD_BUNDLE");
        let data = std::fs::read(path).expect("the pinned artifact");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|root| root.join("workspace-boundaries.toml").is_file())
            .expect("workspace root")
            .to_path_buf();
        let pin: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("content/world/pins/oteryn.json")).expect("pin"),
        )
        .expect("pin json");
        let hex = pin["digest"].as_str().expect("digest");
        let mut digest = [0u8; 32];
        for (i, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex");
        }
        let text = |key: &str| pin[key].as_str().expect(key).to_owned();
        let start = &pin["entry_start"];
        let start = TilePos {
            x: u16::try_from(start["x"].as_u64().expect("x")).expect("x"),
            y: u16::try_from(start["y"].as_u64().expect("y")).expect("y"),
            floor: i8::try_from(start["floor"].as_i64().expect("floor")).expect("floor"),
        };
        let pins = BootPins {
            bundle: BundlePins {
                digest,
                project_format_version: text("project_format_version"),
                world_schema_version: text("world_schema_version"),
                content_revision: text("content_revision"),
                production: false,
            },
            map_revision: format!("sha256:{hex}"),
            start,
        };
        let controller = crate::content::native_gameplay::test_support::activated_with_item_keys(
            Some(ITEM_KEYS),
        );
        let state = controller
            .active()
            .expect("active")
            .native_gameplay()
            .expect("gameplay");
        let world = crate::map::boot::check(data, pins)
            .expect("check")
            .boot(
                WorldId::decode(&id(1)).unwrap(),
                ChannelId::decode(&id(2)).unwrap(),
                generation_item(state),
            )
            .expect("the pinned artifact boots with the generation's Items");
        let (_, cells) = cells(&world);
        let tiles = cells.spell_tiles();
        let (mut known, mut unknown, mut zone, mut projectile, mut floor_change, mut rope) =
            (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        for (floor, x, y, _) in world.base().tiles() {
            let cell = LogicalCell {
                x: i32::from(x),
                y: i32::from(y),
                z: -i32::from(floor),
            };
            match tiles.lookup(cells.scope(), cell) {
                Ok(tile) => {
                    known += 1;
                    zone += usize::from(tile.flags().protection_zone);
                    projectile += usize::from(tile.flags().block_projectile);
                    floor_change += usize::from(tile.flags().floor_change);
                    if tile
                        .ground_source_id()
                        .is_some_and(|ground| ROPE_SPOTS.contains(&ground))
                    {
                        rope += 1;
                        assert!(!tile.flags().floor_change, "rope spot at {cell:?}");
                    }
                }
                Err(SpellTileLookupError::Unknown) => unknown += 1,
                Err(error) => panic!("{cell:?}: {error:?}"),
            }
        }
        let store = store(&cells);
        eprintln!(
            "tiles {} shapes {} known {known} unknown {unknown} protection_zone {zone} \
             block_projectile {projectile} floor_change {floor_change} rope_spots {rope}",
            world.base().tile_count(),
            store.shapes.len(),
        );
        assert_eq!(known + unknown, world.base().tile_count());
        assert!(known > 0 && zone > 0 && floor_change > 0 && rope > 0);
        let start = tiles
            .lookup(
                cells.scope(),
                LogicalCell {
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    z: -i32::from(start.floor),
                },
            )
            .expect("the start tile is known");
        assert!(start.ground_present() && !start.flags().block_solid);
    }
}
