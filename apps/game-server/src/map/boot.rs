//! MAP-CUTOVER-1a: boot a World's Channel from a configured world bundle (MAP track packets
//! §2.3).
//!
//! [`boot`] loads the bundle with the configured pins, refuses a `map_revision` other than the
//! one the bundle names and a start tile a player cannot enter, and returns a [`BundleWorld`]:
//! the shared base, an empty Channel overlay and the set of walkable tiles an entry blocks.
//! Nothing durable names the bundle; a restart rebuilds an equal World from the same pins.

use super::overlay::{ChannelOverlay, TilePos, map_revision};
use super::{BundlePins, LoadError, WorldBase};
use crate::content::static_cell_engine::{EngineeringStaticCellScope, StaticCellEngineError};
use crate::content::{
    CollisionClass, ContentError, CoordinateFrameRef, LogicalCell, MapRevisionRef,
    NativeEntryMovementCells,
};
use crate::foundation::{ChannelId, WorldId};
use crate::movement::speed::{EngineeringGroundSpeed, GroundSpeedSource};
use oteryn_world_bundle::bundle::{self, Family, TerrainKind};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

/// The coordinate frame of a Channel's movement cells over a world bundle: native `x`, `y` and
/// legacy `z` (native floor `-z`).
const BUNDLE_FRAME: &str = "oteryn:frame/world-bundle-v3";

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
}

impl fmt::Display for BootRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(f, "{error}"),
            Self::MapRevision => f.write_str("map_revision is not the world bundle's revision"),
            Self::StartNotEnterable => f.write_str("the world bundle start tile is not enterable"),
        }
    }
}

impl std::error::Error for BootRefusal {}

impl From<LoadError> for BootRefusal {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

/// A Channel booted from a world bundle: the shared base, its empty overlay, the start tile and
/// the walkable tiles a wall or a solid item blocks.
#[derive(Debug)]
pub struct BundleWorld {
    base: Arc<WorldBase>,
    overlay: ChannelOverlay,
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
            && self.overlay.world_id() == other.overlay.world_id()
            && self.overlay.channel_id() == other.overlay.channel_id()
            && self.overlay.tiles().next().is_none()
            && other.overlay.tiles().next().is_none()
    }
}

impl Eq for BundleWorld {}

/// Boots `world_id`'s Channel `channel_id` from the bundle bytes. `solid` answers whether the
/// item definition a palette key names has `block_solid`; an unknown definition (`None`)
/// blocks.
pub fn boot(
    data: &[u8],
    pins: &BootPins,
    world_id: WorldId,
    channel_id: ChannelId,
    solid: impl Fn(&str) -> Option<bool>,
) -> Result<BundleWorld, BootRefusal> {
    let base = Arc::new(super::load(data, &pins.bundle)?);
    let revision = map_revision(&base);
    if revision != pins.map_revision {
        return Err(BootRefusal::MapRevision);
    }
    let blocked = Arc::new(blocked_tiles(data, &solid)?);
    if !enterable(&base, &blocked, pins.start) {
        return Err(BootRefusal::StartNotEnterable);
    }
    Ok(BundleWorld {
        overlay: ChannelOverlay::new(Arc::clone(&base), world_id, channel_id),
        base,
        map_revision: revision,
        start: pins.start,
        blocked,
    })
}

/// The walkable-or-not tiles one of whose top-level entries is a `wall` Terrain or an item
/// whose definition is solid.
fn blocked_tiles(
    data: &[u8],
    solid: &impl Fn(&str) -> Option<bool>,
) -> Result<BTreeSet<TilePos>, LoadError> {
    let mut blocked = BTreeSet::new();
    bundle::visit(data, |manifest, sector| {
        for tile in &sector.tiles {
            let blocks = tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .any(|item| {
                    manifest
                        .palette
                        .get(item.palette as usize)
                        .is_none_or(|entry| {
                            entry
                                .terrain
                                .as_ref()
                                .is_some_and(|terrain| terrain.kind == TerrainKind::Wall)
                                || (entry.family == Family::Item
                                    && solid(&entry.key) != Some(false))
                        })
                });
            if blocks {
                blocked.insert(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    Ok(blocked)
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
        &self.overlay
    }

    pub fn map_revision(&self) -> &str {
        &self.map_revision
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
        EngineeringGroundSpeed.ground_speed(LogicalCell {
            x: i32::from(pos.x),
            y: i32::from(pos.y),
            z: -i32::from(pos.floor),
        })
    }

    /// The Channel's movement cells over this bundle: `entry`'s scope in the bundle's frame and
    /// `map_revision`, with the [`BundleCollisionIndex`].
    pub(crate) fn movement_cells(
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
        };
        Ok(NativeEntryMovementCells::from_bundle(entry, scope, index))
    }
}

/// The collision of a Channel's movement cells over a world bundle: a tile is Walkable when it
/// is enterable, Blocked when it exists but is not, and Absent outside the map.
#[derive(Debug, Clone)]
pub(crate) struct BundleCollisionIndex {
    scope: EngineeringStaticCellScope,
    base: Arc<WorldBase>,
    blocked: Arc<BTreeSet<TilePos>>,
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
                terrain("terrain:grass", 1, TerrainKind::Ground, Some(true)),
                terrain("terrain:lava", 2, TerrainKind::Ground, Some(false)),
                terrain("terrain:wall", 3, TerrainKind::Wall, None),
                item("item:box", 1),
                item("item:coin", 2),
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

    fn solid(key: &str) -> Option<bool> {
        match key {
            "item:box" => Some(true),
            "item:coin" => Some(false),
            _ => None,
        }
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
        let world = boot(&bytes, &pins(load, 2), world_id, channel_id, solid)?;
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
    fn map_cutover_an_unknown_item_definition_blocks() -> Result<(), Box<dyn Error>> {
        let (bytes, load) = bundle();
        let world = boot(
            &bytes,
            &pins(load, 1),
            WorldId::decode(&id(1))?,
            ChannelId::decode(&id(2))?,
            |_| None,
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
}
