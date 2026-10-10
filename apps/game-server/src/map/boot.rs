//! MAP-CUTOVER-1a: boot a World's Channel from a configured world bundle (MAP track packets
//! §2.3).
//!
//! [`check`] loads the bundle with the configured pins and refuses a `map_revision` other than
//! the one the bundle names and a start tile without walkable ground; [`CheckedBundle::boot`]
//! then resolves the palette against the served item definitions (MAP-CUTOVER-1b), refuses a
//! start tile a solid item blocks, and returns a [`BundleWorld`]: the shared base, the Channel's
//! map view (an empty overlay and the map facts) and the set of walkable tiles an entry blocks.
//! Nothing durable names the bundle; a restart rebuilds an equal World from the same pins.

use super::facts::{BundleFacts, FactsRefusal, ItemDefinition};
use super::floor::{FloorTable, Ground, palette_mask};
use super::overlay::{ChannelOverlay, TilePos, map_revision};
use super::{BundlePins, LoadError, WorldBase};
use crate::content::static_cell_engine::{EngineeringStaticCellScope, StaticCellEngineError};
use crate::content::{
    CollisionClass, ContentError, CoordinateFrameRef, LogicalCell, MapRevisionRef,
    NativeEntryMovementCells,
};
use crate::foundation::{ChannelId, WorldId};
use crate::movement::speed::{EngineeringGroundSpeed, GroundSpeedSource};
use oteryn_world_bundle::bundle;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

/// The coordinate frame of a Channel's movement cells over a world bundle: native `x`, `y` and
/// legacy `z` (native floor `-z`).
const BUNDLE_FRAME: &str = "oteryn:frame/world-bundle-v3";

const BUNDLE_FRAME_BINDING_DOMAIN: &[u8] = b"OTERYN_WORLD_BUNDLE_FRAME_BINDING/v1\0";

/// What a World's configuration pins for its bundle: the load pins, the `map_revision` the
/// World's readiness names and the start tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPins {
    pub bundle: BundlePins,
    pub map_revision: String,
    pub start: TilePos,
}

/// Why a World refused to boot from its bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootRefusal {
    /// The bundle failed to load (digest, schema, content revision, production or format).
    Load(LoadError),
    /// The configured `map_revision` is not the bundle's `sha256:<digest>`.
    MapRevision,
    /// The configured start tile cannot be entered.
    StartNotEnterable,
    /// A palette Item key the §1.6 item index lacks or places at another `id`.
    ItemReference,
    /// MAP-FLOOR-1: a floor-change tile whose successive destinations never end.
    FloorChain(TilePos),
    /// MAP-FLOOR-1: a served bundle places no tile that changes floors, so its stairs,
    /// ladders and holes did not resolve against the content catalogue.
    NoFloorChanges,
}

impl fmt::Display for BootRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(f, "{error}"),
            Self::MapRevision => f.write_str("map_revision is not the world bundle's revision"),
            Self::StartNotEnterable => f.write_str("the world bundle start tile is not enterable"),
            Self::ItemReference => {
                f.write_str("a world bundle palette item does not match the item index")
            }
            Self::NoFloorChanges => {
                f.write_str("a world bundle places no tile that changes floors")
            }
            Self::FloorChain(tile) => write!(
                f,
                "the world bundle floor change at ({}, {}, {}) never reaches a destination",
                tile.x, tile.y, tile.floor
            ),
        }
    }
}

impl std::error::Error for BootRefusal {}

impl From<LoadError> for BootRefusal {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<FactsRefusal> for BootRefusal {
    fn from(refusal: FactsRefusal) -> Self {
        match refusal {
            FactsRefusal::Load(error) => Self::Load(error),
            FactsRefusal::ItemReference => Self::ItemReference,
        }
    }
}

/// A Channel booted from a world bundle: the shared base, its empty overlay, the start tile and
/// the walkable tiles a wall or a solid item blocks.
#[derive(Debug)]
pub struct BundleWorld {
    base: Arc<WorldBase>,
    map: Arc<BundleMap>,
    map_revision: String,
    start: TilePos,
    blocked: Arc<BTreeSet<TilePos>>,
}

/// Two boots of the same pins are equal: the same bundle, revision, start, blocked tiles and
/// Channel; the overlays are both empty.
impl PartialEq for BundleWorld {
    fn eq(&self, other: &Self) -> bool {
        self.base.digest() == other.base.digest()
            && self.base.content_revision() == other.base.content_revision()
            && self.base.tile_count() == other.base.tile_count()
            && self.base.entry_count() == other.base.entry_count()
            && self.map_revision == other.map_revision
            && self.start == other.start
            && self.blocked == other.blocked
            && self.map.overlay.world_id() == other.map.overlay.world_id()
            && self.map.overlay.channel_id() == other.map.overlay.channel_id()
            && self.map.overlay.tiles().next().is_none()
            && other.map.overlay.tiles().next().is_none()
    }
}

impl Eq for BundleWorld {}

/// Boots `world_id`'s Channel `channel_id` from the bundle bytes: [`check`], then
/// [`CheckedBundle::boot`].
pub fn boot(
    data: &[u8],
    pins: &BootPins,
    world_id: WorldId,
    channel_id: ChannelId,
    item: impl Fn(&str) -> Option<ItemDefinition>,
) -> Result<BundleWorld, BootRefusal> {
    check(data.to_vec(), pins.clone())?.boot(world_id, channel_id, item)
}

/// The checks that need no item definition, which a node runs before any durable step: the
/// bundle loads with its pins, `map_revision` is the bundle's and the start tile's ground is
/// walkable.
pub fn check(data: Vec<u8>, pins: BootPins) -> Result<CheckedBundle, BootRefusal> {
    let base = Arc::new(super::load(&data, &pins.bundle)?);
    let revision = map_revision(&base);
    if revision != pins.map_revision {
        return Err(BootRefusal::MapRevision);
    }
    if !enterable(&base, &BTreeSet::new(), pins.start) {
        return Err(BootRefusal::StartNotEnterable);
    }
    Ok(CheckedBundle {
        data,
        base,
        map_revision: revision,
        start: pins.start,
    })
}

/// A bundle that passed [`check`] and waits for the served item definitions.
#[derive(Debug)]
pub struct CheckedBundle {
    data: Vec<u8>,
    base: Arc<WorldBase>,
    map_revision: String,
    start: TilePos,
}

impl CheckedBundle {
    /// Builds the World. `item` serves the definition of the Item a palette key names; a palette
    /// Item key it does not serve at its palette `id` refuses the boot (§1.6), and so does a
    /// start tile a solid item blocks.
    pub fn boot(
        self,
        world_id: WorldId,
        channel_id: ChannelId,
        item: impl Fn(&str) -> Option<ItemDefinition>,
    ) -> Result<BundleWorld, BootRefusal> {
        let (facts, blocked, floors) = scan(&self.data, &item)?;
        let blocked = Arc::new(blocked);
        if !enterable(&self.base, &blocked, self.start) {
            return Err(BootRefusal::StartNotEnterable);
        }
        floors
            .validate(|(x, y, z)| {
                let pos = u16::try_from(x)
                    .ok()
                    .zip(u16::try_from(y).ok())
                    .zip(i8::try_from(z).ok().filter(|z| (0..=15).contains(z)));
                match pos {
                    Some(((x, y), z)) => {
                        let pos = TilePos { x, y, floor: -z };
                        if self.base.tile(pos.x, pos.y, pos.floor).is_none() {
                            Ground::Absent
                        } else if enterable(&self.base, &blocked, pos) {
                            Ground::Walkable
                        } else {
                            Ground::Blocked
                        }
                    }
                    None => Ground::Absent,
                }
            })
            .map_err(BootRefusal::FloorChain)?;
        Ok(BundleWorld {
            map: Arc::new(BundleMap {
                overlay: ChannelOverlay::new(Arc::clone(&self.base), world_id, channel_id),
                facts,
                floors,
            }),
            base: self.base,
            map_revision: self.map_revision,
            start: self.start,
            blocked,
        })
    }
}

/// The map facts of the bundle, the walkable-or-not tiles one of whose top-level entries is
/// a `wall` Terrain or an Item whose definition is solid, and the tiles whose top-level entries
/// carry a catalogue `floor_change` fact (MAP-FLOOR-1).
fn scan(
    data: &[u8],
    item: &impl Fn(&str) -> Option<ItemDefinition>,
) -> Result<(BundleFacts, BTreeSet<TilePos>, FloorTable), BootRefusal> {
    let mut facts = None;
    let mut masks: Vec<u8> = Vec::new();
    let mut floors = FloorTable::default();
    let mut refused = None;
    let mut blocked = BTreeSet::new();
    let visited = bundle::visit(data, |manifest, sector| {
        if facts.is_none() {
            match BundleFacts::palette(manifest, item) {
                Ok(palette) => {
                    masks = manifest
                        .palette
                        .iter()
                        .map(|entry| palette_mask(entry.family, &entry.key))
                        .collect();
                    facts = Some(palette);
                }
                Err(refusal) => {
                    refused = Some(refusal);
                    return Err(oteryn_world_bundle::Error::Format(
                        "palette does not match the item index".into(),
                    ));
                }
            }
        }
        let Some(facts) = facts.as_mut() else {
            return Ok(());
        };
        for tile in &sector.tiles {
            let blocks = tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .any(|item| facts.blocks(item.palette));
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            if blocks {
                blocked.insert(pos);
            }
            let mask = tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .fold(0, |mask, item| {
                    mask | masks.get(item.palette as usize).copied().unwrap_or(0)
                });
            floors.add(pos, mask);
        }
        facts.push(manifest, &sector);
        Ok(())
    });
    if let Some(refusal) = refused {
        return Err(refusal.into());
    }
    let visited = visited.map_err(LoadError::from)?;
    let mut facts = match facts {
        Some(facts) => facts,
        None => BundleFacts::palette(&visited.manifest, item)?,
    };
    facts.finish();
    Ok((facts, blocked, floors))
}

/// The one ground speed switch point of a bundle World (§2.4): the server paces every step with
/// the Engineering 150 source until MAP-CLIENT-1, not the bundle's stored ground speed.
fn ground_speed(pos: TilePos) -> u16 {
    EngineeringGroundSpeed.ground_speed(LogicalCell {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        z: -i32::from(pos.floor),
    })
}

/// The ground speed domain 17 sends for `pos`, whose base ground stores `stored` (0 without a
/// ground item): the speed the server paces with ([`ground_speed`]), so server and client stay
/// on the Engineering 150 source until MAP-CLIENT-1; 0 without a ground item.
pub(crate) fn view_ground_speed(pos: TilePos, stored: u16) -> u16 {
    if stored == 0 { 0 } else { ground_speed(pos) }
}

/// A tile a player can enter: its ground is walkable and no entry blocks it.
fn enterable(base: &WorldBase, blocked: &BTreeSet<TilePos>, pos: TilePos) -> bool {
    base.tile(pos.x, pos.y, pos.floor)
        .is_some_and(|tile| tile.walkable())
        && !blocked.contains(&pos)
}

/// The native tile of a movement cell: `z` is legacy `z` in `0..=15`, native floor `-z`.
fn tile_pos(cell: LogicalCell) -> Option<TilePos> {
    let z = i8::try_from(cell.z).ok().filter(|z| (0..=15).contains(z))?;
    Some(TilePos {
        x: u16::try_from(cell.x).ok()?,
        y: u16::try_from(cell.y).ok()?,
        floor: -z,
    })
}

impl BundleWorld {
    pub fn base(&self) -> &Arc<WorldBase> {
        &self.base
    }

    pub fn overlay(&self) -> &ChannelOverlay {
        &self.map.overlay
    }

    /// The production map facts of the bundle.
    pub fn facts(&self) -> &BundleFacts {
        &self.map.facts
    }

    /// The tiles of this bundle that change floors (MAP-FLOOR-1).
    pub fn floors(&self) -> &FloorTable {
        &self.map.floors
    }

    pub fn map_revision(&self) -> &str {
        &self.map_revision
    }

    /// The Channel pin's map-revision identity of this bundle: SHA-256 over its
    /// `sha256:<digest>` map revision, the form a qualified entry room's pin carries.
    pub fn map_revision_digest(&self) -> [u8; 32] {
        Sha256::digest(self.map_revision.as_bytes()).into()
    }

    /// The Channel pin's frame identity of this bundle in `world`: SHA-256 over the World, the
    /// bundle coordinate frame and the map revision, so no bundle position shares an entry
    /// room's frame binding.
    pub fn frame_binding_digest(&self, world: WorldId) -> [u8; 32] {
        let mut bytes = Vec::with_capacity(160);
        bytes.extend_from_slice(BUNDLE_FRAME_BINDING_DOMAIN);
        bytes.extend_from_slice(world.as_bytes());
        for part in [BUNDLE_FRAME, self.map_revision.as_str()] {
            bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
            bytes.extend_from_slice(part.as_bytes());
        }
        Sha256::digest(&bytes).into()
    }

    pub fn start(&self) -> TilePos {
        self.start
    }

    /// Whether a player can enter `pos`.
    pub fn enterable(&self, pos: TilePos) -> bool {
        enterable(&self.base, &self.blocked, pos)
    }

    /// The ground speed a step onto `pos` uses. This is the one switch point: Engineering 150
    /// until the map's own ground speed is served (MAP-CLIENT-1).
    pub fn ground_speed(&self, pos: TilePos) -> u16 {
        ground_speed(pos)
    }

    /// The Channel's movement cells over this bundle: `entry`'s scope in the bundle's frame and
    /// `map_revision`, with the bundle collision index, which also carries the Channel's map
    /// view ([`BundleCollisionIndex::map`]).
    pub fn movement_cells(
        &self,
        entry: &NativeEntryMovementCells,
    ) -> Result<NativeEntryMovementCells, ContentError> {
        let mut scope = entry.scope().clone();
        scope.coordinate_frame = CoordinateFrameRef::new(BUNDLE_FRAME)?;
        scope.map_revision = MapRevisionRef::new(&self.map_revision)?;
        let index = BundleCollisionIndex {
            scope: scope.clone(),
            base: Arc::clone(&self.base),
            blocked: Arc::clone(&self.blocked),
            map: Arc::clone(&self.map),
        };
        Ok(NativeEntryMovementCells::from_bundle(entry, scope, index))
    }
}

/// What domain 17 of a bundle World's Channel is composed from: the Channel overlay, which stays
/// empty until MAP-CUTOVER-1c (§1.2), and the bundle's map facts.
#[derive(Debug)]
pub(crate) struct BundleMap {
    pub(crate) overlay: ChannelOverlay,
    pub(crate) facts: BundleFacts,
    pub(crate) floors: FloorTable,
}

/// The collision of a Channel's movement cells over a world bundle: a tile is Walkable when it
/// is enterable, Blocked when it exists but is not, and Absent outside the map.
#[derive(Debug, Clone)]
pub(crate) struct BundleCollisionIndex {
    scope: EngineeringStaticCellScope,
    base: Arc<WorldBase>,
    blocked: Arc<BTreeSet<TilePos>>,
    map: Arc<BundleMap>,
}

impl PartialEq for BundleCollisionIndex {
    fn eq(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.base.digest() == other.base.digest()
            && self.blocked == other.blocked
    }
}

impl Eq for BundleCollisionIndex {}

impl BundleCollisionIndex {
    /// The Channel's map view over the same bundle.
    pub(crate) fn map(&self) -> &BundleMap {
        &self.map
    }

    pub(crate) fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        if scope != &self.scope {
            return Err(StaticCellEngineError::ScopeMismatch);
        }
        let pos = tile_pos(cell).ok_or(StaticCellEngineError::Absent)?;
        self.base
            .tile(pos.x, pos.y, pos.floor)
            .ok_or(StaticCellEngineError::Absent)?;
        Ok(if enterable(&self.base, &self.blocked, pos) {
            CollisionClass::Walkable
        } else {
            CollisionClass::Blocked
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::{
        ChannelContentPin, ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId,
    };
    use crate::movement::{
        CardinalStep, MovementEngineeringSelection, MovementError, MovementOwnerTurn,
        MovementTurnOutcome,
    };
    use std::error::Error;
    use std::num::NonZeroUsize;

    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }

    /// One row at native floor -7 (legacy `z` 7): grass at x 1..=3, lava (not walkable) at x 4,
    /// grass under a wall at x 5, grass under a solid box at x 6 and under a loose coin at x 7.
    pub(crate) fn bundle() -> (Vec<u8>, BundlePins) {
        bundle_with_items("item:box", "item:coin")
    }

    /// [`bundle`] with `boxed` as palette Item id 0 (the solid box) and `coin` as Item id 1.
    pub(crate) fn bundle_with_items(boxed: &str, coin: &str) -> (Vec<u8>, BundlePins) {
        use oteryn_world_bundle_compiler::bundle::{
            self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
            TerrainKind,
        };
        use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
        let terrain = |key: &str, id, kind, walkable: Option<bool>| PaletteEntry {
            key: key.into(),
            family: Family::Terrain,
            id,
            terrain: Some(Terrain {
                kind,
                walkable,
                ground_speed: walkable.map(|_| 150),
            }),
        };
        let item = |key: &str, id| PaletteEntry {
            key: key.into(),
            family: Family::Item,
            id,
            terrain: None,
        };
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
                max_x: 32,
                max_y: 32,
                floors: vec![-7],
            },
            palette: vec![
                terrain("terrain:grass", 0, TerrainKind::Ground, Some(true)),
                terrain("terrain:lava", 1, TerrainKind::Ground, Some(false)),
                terrain("terrain:wall", 2, TerrainKind::Wall, None),
                item(boxed, 0),
                item(coin, 1),
            ],
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
        };
        let entry = |palette| Item {
            palette,
            depth: 0,
            attrs: Attrs::default(),
        };
        let tile = |x, palettes: &[u32]| Tile {
            x,
            y: 0,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: palettes.iter().map(|palette| entry(*palette)).collect(),
        };
        let sector = Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![
                tile(1, &[0]),
                tile(2, &[0]),
                tile(3, &[0]),
                tile(4, &[1]),
                tile(5, &[0, 2]),
                tile(6, &[0, 3]),
                tile(7, &[0, 4]),
            ],
        };
        let bytes =
            bundle::write(&manifest, &[sector], &Default::default()).expect("written bundle");
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

    /// MAP-FLOOR-1: two floors of one row, whose palette keys are real appearance ids, so the
    /// content catalogue's `floor_change` facts apply. Legacy `z` 8 (native -8) has grass at
    /// x 0..=5 and an east stair (`i1950`) on x 1; `z` 7 (native -7) has grass at x 0..=2 (a solid
    /// box on x 1), a hole (Terrain `i293`, `down`) at x 3 and nothing beyond.
    pub(crate) fn floor_bundle() -> (Vec<u8>, BundlePins) {
        floor_bundle_with(false)
    }

    /// [`floor_bundle`], plus a stair up onto a hole whose floor below leads back to the stair
    /// (`cycle`): stair `i1947` (north) on `z` 8 at (3, 3) and (3, 2), the hole on `z` 7 at (3, 2).
    pub(crate) fn floor_bundle_with(cycle: bool) -> (Vec<u8>, BundlePins) {
        use oteryn_world_bundle_compiler::bundle::{
            self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
            TerrainKind,
        };
        use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
        let terrain = |key: &str, id| PaletteEntry {
            key: key.into(),
            family: Family::Terrain,
            id,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        };
        let item = |key: &str, id| PaletteEntry {
            key: key.into(),
            family: Family::Item,
            id,
            terrain: None,
        };
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
                max_x: 32,
                max_y: 32,
                floors: vec![-8, -7],
            },
            palette: vec![
                terrain("oteryn:terrain.tibia.i100", 0),
                terrain("oteryn:terrain.tibia.i293", 1),
                item("oteryn:item.tibia.i1950", 0),
                item("oteryn:item.tibia.i100", 1),
                item("oteryn:item.tibia.i1947", 2),
            ],
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
        };
        let tile = |x, y, palettes: &[u32]| Tile {
            x,
            y,
            flags: 0,
            house: 0,
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
        // Palette indices: 0 grass, 1 hole, 2 east stair, 3 box, 4 north stair.
        let mut lower = vec![
            tile(0, 0, &[0]),
            tile(1, 0, &[0, 2]),
            tile(2, 0, &[0]),
            tile(3, 0, &[0]),
            tile(4, 0, &[0]),
            tile(5, 0, &[0]),
        ];
        let mut upper = vec![
            tile(0, 0, &[0]),
            tile(1, 0, &[0, 3]),
            tile(2, 0, &[0]),
            tile(3, 0, &[1]),
        ];
        if cycle {
            lower.push(tile(3, 2, &[0, 4]));
            lower.push(tile(3, 3, &[0, 4]));
            upper.push(tile(3, 2, &[1]));
        }
        for tiles in [&mut lower, &mut upper] {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
        }
        let sectors = [
            Sector {
                floor: -8,
                sx: 0,
                sy: 0,
                tiles: lower,
            },
            Sector {
                floor: -7,
                sx: 0,
                sy: 0,
                tiles: upper,
            },
        ];
        let bytes =
            bundle::write(&manifest, &sectors, &Default::default()).expect("written bundle");
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

    /// The served definitions of [`floor_bundle`]'s Items: the stair is walkable, the box solid.
    pub(crate) fn floor_items(key: &str) -> Option<ItemDefinition> {
        let (reference, solid) = match key {
            "oteryn:item.tibia.i1950" => (1, Some(false)),
            "oteryn:item.tibia.i100" => (2, Some(true)),
            "oteryn:item.tibia.i1947" => (3, Some(false)),
            _ => return None,
        };
        Some(ItemDefinition {
            reference: std::num::NonZeroU32::new(reference)?,
            solid,
            blocks_projectile: solid == Some(true),
            pickupable: false,
        })
    }

    /// [`pins`] for [`floor_bundle`], starting on legacy `z` 8.
    pub(crate) fn floor_pins(bundle: BundlePins) -> BootPins {
        BootPins {
            start: TilePos {
                x: 0,
                y: 0,
                floor: -8,
            },
            ..pins(bundle, 0)
        }
    }

    /// The served definitions of the test bundle's Items: the §1.6 index places `item:box` at
    /// Item id 0 and `item:coin` at 1.
    pub(crate) fn items(key: &str) -> Option<ItemDefinition> {
        let (reference, solid, pickupable) = match key {
            "item:box" => (1, Some(true), false),
            "item:coin" => (2, Some(false), true),
            _ => return None,
        };
        Some(ItemDefinition {
            reference: std::num::NonZeroU32::new(reference)?,
            solid,
            blocks_projectile: solid == Some(true),
            pickupable,
        })
    }

    fn pins(bundle: BundlePins, x: u16) -> BootPins {
        let mut revision = String::from("sha256:");
        for byte in bundle.digest {
            revision.push_str(&format!("{byte:02x}"));
        }
        BootPins {
            bundle,
            map_revision: revision,
            start: TilePos { x, y: 0, floor: -7 },
        }
    }

    #[test]
    fn map_cutover_channel_walk_uses_the_bundle_index() -> Result<(), Box<dyn Error>> {
        let (bytes, load) = bundle();
        let world_id = WorldId::decode(&id(1))?;
        let channel_id = ChannelId::decode(&id(2))?;
        let world = boot(&bytes, &pins(load, 2), world_id, channel_id, items)?;
        assert_eq!(world.ground_speed(world.start()), 150);
        let room = crate::content::qualify_native_entry_room(world_id)?;
        let cells = world.movement_cells(room.movement_cells())?;
        assert_eq!(cells.scope().map_revision.as_str(), world.map_revision());
        assert!(matches!(
            cells.index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        ));
        // The entry room's own scope does not open the bundle's cells.
        assert_eq!(
            cells.index().lookup(
                room.movement_cells().scope(),
                LogicalCell { x: 2, y: 0, z: 7 }
            ),
            Err(StaticCellEngineError::ScopeMismatch)
        );

        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            channel_id,
            NodeId::decode(&id(3))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world_id),
        )?;
        let reserved = runtime.reserve_fresh_session(GameSessionId::decode(&id(4))?)?;
        let actor = runtime.commit_fresh_session(reserved)?;
        let mut at = runtime.initialize_movement_test_position(
            actor,
            MovementLocalPosition {
                x: 2,
                y: 0,
                floor: 7,
            },
        )?;
        let selection = MovementEngineeringSelection {
            owner_context: at.context(),
            content_scope: cells.scope(),
        };
        let limit = NonZeroUsize::MIN;
        let step = |runtime: &mut ChannelRuntimeV1, at, direction| {
            MovementOwnerTurn::begin(runtime, limit).try_step(
                actor,
                at,
                &selection,
                cells.index(),
                direction,
            )
        };
        // Onto walkable ground.
        at = match step(&mut runtime, at, CardinalStep::East)? {
            MovementTurnOutcome::Applied(snapshot) => snapshot,
            MovementTurnOutcome::Deferred => return Err("deferred step".into()),
        };
        assert_eq!(runtime.borrow_movement_position().read(actor)?, at);
        // Onto non-walkable ground (x 4).
        assert!(matches!(
            step(&mut runtime, at, CardinalStep::East),
            Err(MovementError::Blocked)
        ));
        // Back west twice, then nothing at x 0.
        for _ in 0..2 {
            at = match step(&mut runtime, at, CardinalStep::West)? {
                MovementTurnOutcome::Applied(snapshot) => snapshot,
                MovementTurnOutcome::Deferred => return Err("deferred step".into()),
            };
        }
        assert!(step(&mut runtime, at, CardinalStep::West).is_err());
        assert_eq!(runtime.borrow_movement_position().read(actor)?, at);

        // A wall, a solid item and a loose item, through the same index.
        let class = |x| {
            cells
                .index()
                .lookup(cells.scope(), LogicalCell { x, y: 0, z: 7 })
        };
        assert_eq!(class(4), Ok(CollisionClass::Blocked));
        assert_eq!(class(5), Ok(CollisionClass::Blocked));
        assert_eq!(class(6), Ok(CollisionClass::Blocked));
        assert_eq!(class(7), Ok(CollisionClass::Walkable));
        assert_eq!(class(8), Err(StaticCellEngineError::Absent));
        assert_eq!(
            cells
                .index()
                .lookup(cells.scope(), LogicalCell { x: 1, y: 0, z: 16 }),
            Err(StaticCellEngineError::Absent)
        );
        Ok(())
    }

    #[test]
    fn map_floor_boot_collects_the_catalogue_floor_changes() -> Result<(), Box<dyn Error>> {
        let (bytes, load) = floor_bundle();
        let world = boot(
            &bytes,
            &floor_pins(load),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            floor_items,
        )?;
        let at = |x, floor| TilePos { x, y: 0, floor };
        // The east stair item on z 8 and the `down` hole Terrain on z 7; plain grass, the
        // solid box and an UNKNOWN catalogue fact are no floor change.
        assert_eq!(world.floors().count(), 2);
        assert!(world.floors().contains(at(1, -8)));
        assert!(world.floors().contains(at(3, -7)));
        assert!(!world.floors().contains(at(1, -7)));
        // A fixture bundle without a catalogue key has none.
        let (bytes, load) = bundle();
        let flat = boot(
            &bytes,
            &pins(load, 1),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            items,
        )?;
        assert_eq!(flat.floors().count(), 0);
        Ok(())
    }

    #[test]
    fn map_floor_boot_refuses_a_floor_change_that_never_ends() -> Result<(), Box<dyn Error>> {
        let (bytes, load) = floor_bundle_with(true);
        let refused = boot(
            &bytes,
            &floor_pins(load),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            floor_items,
        );
        assert_eq!(
            refused,
            Err(BootRefusal::FloorChain(TilePos {
                x: 3,
                y: 2,
                floor: -7
            }))
        );
        Ok(())
    }

    #[test]
    fn map_cutover_an_unknown_item_solidity_blocks() -> Result<(), Box<dyn Error>> {
        let (bytes, load) = bundle();
        let world = boot(
            &bytes,
            &pins(load, 1),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            |key| {
                items(key).map(|definition| ItemDefinition {
                    solid: None,
                    ..definition
                })
            },
        )?;
        assert!(world.enterable(TilePos {
            x: 1,
            y: 0,
            floor: -7
        }));
        assert!(!world.enterable(TilePos {
            x: 7,
            y: 0,
            floor: -7
        }));
        Ok(())
    }

    #[test]
    fn map_cutover_a_palette_item_the_index_disagrees_with_refuses_boot()
    -> Result<(), Box<dyn Error>> {
        let (bytes, load) = bundle();
        let (world_id, channel_id) = (WorldId::decode(&id(1))?, ChannelId::decode(&id(2))?);
        let shifted = |key: &str| {
            items(key).map(|definition| ItemDefinition {
                reference: definition.reference.saturating_add(1),
                ..definition
            })
        };
        let missing = |key: &str| items(key).filter(|_| key != "item:coin");
        for refused in [
            boot(
                &bytes,
                &pins(load.clone(), 1),
                world_id,
                channel_id,
                shifted,
            ),
            boot(
                &bytes,
                &pins(load.clone(), 1),
                world_id,
                channel_id,
                missing,
            ),
        ] {
            assert_eq!(refused, Err(BootRefusal::ItemReference));
        }
        Ok(())
    }

    #[test]
    fn map_cutover_facts_send_palette_id_zero_as_one() -> Result<(), Box<dyn Error>> {
        use crate::map::view::{EntryFacts, MapFacts};
        use oteryn_protocol_oteryn::world_map::MapDefinition;
        let (bytes, load) = bundle();
        let world = boot(
            &bytes,
            &pins(load, 1),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            items,
        )?;
        let facts = world.facts();
        let at = |x| TilePos { x, y: 0, floor: -7 };
        let one = std::num::NonZeroU32::MIN;
        // Grass (Terrain id 0) and the box (Item id 0) share compact id 0.
        let grass = facts.base_entry(at(6), 0, 0).ok_or("grass facts")?;
        assert_eq!(grass.definition, MapDefinition::Terrain(one));
        assert_eq!(
            grass.terrain_kind,
            Some(oteryn_world_bundle::bundle::TerrainKind::Ground)
        );
        let solid_box = facts.base_entry(at(6), 1, 0).ok_or("box facts")?;
        assert_eq!(
            solid_box,
            EntryFacts {
                definition: MapDefinition::Item(one),
                terrain_kind: None,
                appearance_id: 0,
                blocks_projectile: true,
                pickupable: false,
                bound: false,
                count: 1,
                sub_type: 0,
            }
        );
        let coin = facts.base_entry(at(7), 1, 1).ok_or("coin facts")?;
        assert_eq!(coin.definition, MapDefinition::Item(one.saturating_add(1)));
        assert!(coin.pickupable && !coin.bound);
        // A compact id that is not the placement's is no fact.
        assert_eq!(facts.base_entry(at(7), 1, 0), None);
        assert!(!facts.house_tile(at(7)));
        assert_eq!(facts.object_revision(0), 0);
        Ok(())
    }
}
