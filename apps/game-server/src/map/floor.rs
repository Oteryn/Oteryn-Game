//! MAP-FLOOR-1: floor changes (stairs, ladders, holes) of a bundle World.
//!
//! ADR-0021 and `OTERYN_WORLD_BUNDLE_FORMAT_V1` keep floor changes out of the bundle: a tile
//! changes floors because an entry placed on it has a `floor_change` fact in the content
//! catalogue. [`palette_mask`] resolves a palette entry against that catalogue (the generated
//! [`super::floor_catalogue`] table, kept equal to the JSON by a test), the boot collects the
//! flags of every tile into a [`FloorTable`], and a step is resolved by [`resolve`], the Canary
//! `Tile::queryDestination` rules over the table and the bundle collision. The resolved
//! destination is committed by the owner turn through a sealed [`BundleStepProof`], exactly like
//! a source-map step, so nothing but an actual step moves a player across floors.
//!
//! Coordinates here are legacy `z` (`0..=15`, deeper is larger), the runtime floor of a bundle
//! World; [`TilePos`] keeps the native floor `-z`. Not modelled: the source height-3 climb rule,
//! because the bundle carries no height facts.

use super::floor_catalogue::{OBJECT, TERRAIN};
use super::overlay::TilePos;
use super::palette_appearance;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::content::static_cell_engine::StaticCellEngineError;
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, MovementPositionSnapshot,
};
use crate::movement::{CardinalStep, MovementError};
use oteryn_world_bundle::bundle::Family;
use std::collections::BTreeMap;

const DOWN: u8 = 1;
const NORTH: u8 = 2;
const SOUTH: u8 = 4;
const EAST: u8 = 8;
const WEST: u8 = 16;
const SOUTHALT: u8 = 32;
const EASTALT: u8 = 64;

/// The source bound on successive destination queries (`MAP_MAX_LAYERS`).
const MAX_DESTINATIONS: usize = 16;

/// The deepest legacy `z`.
const MAX_Z: i32 = 15;

/// The floor-change flags of the entry a palette key names, as a bit set; 0 for a key that is not
/// a Tibia appearance or whose catalogue fact is not a floor change.
pub(crate) fn palette_mask(family: Family, key: &str) -> u8 {
    let Some(appearance) = palette_appearance(key) else {
        return 0;
    };
    let table = match family {
        Family::Item => OBJECT,
        Family::Terrain => TERRAIN,
    };
    table
        .binary_search_by_key(&appearance, |(id, _)| *id)
        .map_or(0, |found| table[found].1)
}

/// The tiles of a bundle World that change floors, with the OR of the flags of their entries.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FloorTable {
    tiles: BTreeMap<TilePos, u8>,
}

impl FloorTable {
    pub(crate) fn add(&mut self, pos: TilePos, mask: u8) {
        if mask != 0 {
            *self.tiles.entry(pos).or_insert(0) |= mask;
        }
    }

    /// How many tiles change floors.
    pub fn count(&self) -> usize {
        self.tiles.len()
    }

    /// The flags of the tile at `(x, y, z)`; 0 outside the map and on a plain tile.
    fn mask(&self, (x, y, z): Cell) -> u8 {
        pos(x, y, z)
            .and_then(|pos| self.tiles.get(&pos))
            .copied()
            .unwrap_or(0)
    }

    /// Whether the tile at `pos` changes floors.
    pub fn contains(&self, pos: TilePos) -> bool {
        self.tiles.contains_key(&pos)
    }

    /// Every floor-change tile, ascending.
    pub fn positions(&self) -> impl Iterator<Item = TilePos> + '_ {
        self.tiles.keys().copied()
    }

    /// The boot check: following the destinations from every floor-change tile that `ground`
    /// can enter ends within the source bound. A chain that does not end (a cycle) is bad data.
    pub(crate) fn validate(&self, ground: impl Fn(Cell) -> Ground) -> Result<(), TilePos> {
        for tile in self.positions() {
            let cell = (i32::from(tile.x), i32::from(tile.y), -i32::from(tile.floor));
            if ground(cell) == Ground::Walkable
                && follow(self, &ground, cell) == Err(Refusal::Chain)
            {
                return Err(tile);
            }
        }
        Ok(())
    }
}

/// `(x, y, legacy z)`.
pub(crate) type Cell = (i32, i32, i32);

fn pos(x: i32, y: i32, z: i32) -> Option<TilePos> {
    if !(0..=MAX_Z).contains(&z) {
        return None;
    }
    Some(TilePos {
        x: u16::try_from(x).ok()?,
        y: u16::try_from(y).ok()?,
        floor: -i8::try_from(z).ok()?,
    })
}

/// What a bundle holds at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ground {
    Absent,
    Blocked,
    Walkable,
}

/// Why no destination exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The step target is outside the map.
    Absent,
    /// The tile the step ends on is not enterable.
    Blocked,
    /// More than [`MAX_DESTINATIONS`] successive floor changes.
    Chain,
}

/// The tile a player standing on `origin` ends on after stepping `direction`, following the
/// floor changes it enters.
pub(crate) fn resolve(
    table: &FloorTable,
    ground: impl Fn(Cell) -> Ground,
    origin: Cell,
    direction: CardinalStep,
) -> Result<Cell, Refusal> {
    let (dx, dy) = match direction {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    };
    let next = (origin.0 + dx, origin.1 + dy, origin.2);
    match ground(next) {
        Ground::Absent => Err(Refusal::Absent),
        Ground::Blocked => Err(Refusal::Blocked),
        Ground::Walkable => follow(table, &ground, next),
    }
}

/// `internalMoveCreature`'s successive `queryDestination`, from the entered tile `next`.
fn follow(
    table: &FloorTable,
    ground: &impl Fn(Cell) -> Ground,
    mut next: Cell,
) -> Result<Cell, Refusal> {
    let land = |cell: Cell| match ground(cell) {
        Ground::Walkable => Ok(cell),
        _ => Err(Refusal::Blocked),
    };
    for _ in 0..MAX_DESTINATIONS {
        let mask = table.mask(next);
        if mask == 0 {
            return land(next);
        }
        let target = if mask & DOWN != 0 {
            let below = (next.0, next.1, next.2 + 1);
            let shifted = |dx: i32, dy: i32| (below.0 + dx, below.1 + dy, below.2);
            if table.mask(shifted(0, -1)) & SOUTHALT != 0 {
                shifted(0, -2)
            } else if table.mask(shifted(-1, 0)) & EASTALT != 0 {
                shifted(-2, 0)
            } else if ground(below) != Ground::Absent {
                let below_mask = table.mask(below);
                let mut offset = (0, 0);
                for (flag, dx, dy) in [
                    (NORTH, 0, 1),
                    (SOUTH, 0, -1),
                    (SOUTHALT, 0, -2),
                    (EAST, -1, 0),
                    (EASTALT, -2, 0),
                    (WEST, 1, 0),
                ] {
                    if below_mask & flag != 0 {
                        offset = (offset.0 + dx, offset.1 + dy);
                    }
                }
                shifted(offset.0, offset.1)
            } else {
                return land(next);
            }
        } else {
            let mut offset = (0, 0);
            for (flag, dx, dy) in [
                (NORTH, 0, -1),
                (SOUTH, 0, 1),
                (SOUTHALT, 0, 2),
                (EAST, 1, 0),
                (EASTALT, 2, 0),
                (WEST, -1, 0),
            ] {
                if mask & flag != 0 {
                    offset = (offset.0 + dx, offset.1 + dy);
                }
            }
            (next.0 + offset.0, next.1 + offset.1, next.2 - 1)
        };
        if ground(target) == Ground::Absent {
            return land(next);
        }
        next = target;
    }
    Err(Refusal::Chain)
}

/// The floor table of the bundle `cells` run over; `None` on a fixture or source World.
pub(crate) fn table_of(cells: &NativeEntryMovementCells) -> Option<&FloorTable> {
    match cells.index() {
        NativeMovementCollisionIndex::Bundle(index) => Some(&index.map().floors),
        _ => None,
    }
}

/// What the bundle `cells` hold at `cell`.
fn ground_of(cells: &NativeEntryMovementCells) -> impl Fn(Cell) -> Ground + '_ {
    move |(x, y, z)| match cells.index().lookup(cells.scope(), LogicalCell { x, y, z }) {
        Ok(CollisionClass::Walkable) => Ground::Walkable,
        Ok(CollisionClass::Blocked) => Ground::Blocked,
        Err(_) => Ground::Absent,
    }
}

fn movement_error(refusal: Refusal) -> MovementError {
    match refusal {
        Refusal::Absent => MovementError::Cell(StaticCellEngineError::Absent),
        Refusal::Blocked => MovementError::Blocked,
        Refusal::Chain => MovementError::CapacityExceeded,
    }
}

/// A step of a bundle World proven against its actual collision and floor table. Private
/// construction consumes the actual cells, not a caller's target.
pub(crate) struct BundleStepProof<'a> {
    cells: &'a NativeEntryMovementCells,
    table: &'a FloorTable,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
    destination: MovementLocalPosition,
}

/// Proves `actor`'s step `direction` from its current position over the bundle `cells`.
pub(crate) fn prepare_bundle_step<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
) -> Result<BundleStepProof<'a>, MovementError> {
    let table = table_of(cells).ok_or(MovementError::NotQualified)?;
    qualify_origin(runtime, cells, actor, session, expected)?;
    let destination = destination(cells, table, expected.position(), direction)?;
    Ok(BundleStepProof {
        cells,
        table,
        actor,
        session,
        expected,
        direction,
        destination,
    })
}

fn qualify_origin(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
) -> Result<(), MovementError> {
    if runtime.actor_spell_reserved(actor)
        || cells.scope().world_id != runtime.binding().world_id()
        || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        || expected.context() != runtime.pinned_movement_context()
        || runtime
            .read_actor_position(actor)
            .map_err(MovementError::Actor)?
            != expected
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return Err(MovementError::NotQualified);
    }
    Ok(())
}

fn destination(
    cells: &NativeEntryMovementCells,
    table: &FloorTable,
    origin: MovementLocalPosition,
    direction: CardinalStep,
) -> Result<MovementLocalPosition, MovementError> {
    let floor = i32::from(origin.floor);
    let (x, y, z) = resolve(
        table,
        ground_of(cells),
        (origin.x, origin.y, floor),
        direction,
    )
    .map_err(movement_error)?;
    Ok(MovementLocalPosition {
        x,
        y,
        floor: i16::try_from(z).map_err(|_| MovementError::CoordinateOverflow)?,
    })
}

impl BundleStepProof<'_> {
    pub(crate) const fn destination(&self) -> MovementLocalPosition {
        self.destination
    }

    fn validate(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        let observed = destination(
            self.cells,
            self.table,
            self.expected.position(),
            self.direction,
        )?;
        if observed != self.destination {
            return Err(MovementError::SnapshotMismatch);
        }
        Ok(())
    }
}

impl crate::foundation::source_step_seal::Sealed for BundleStepProof<'_> {}
impl crate::foundation::SourceStepCommitProof for BundleStepProof<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.validate(runtime).map_err(|error| match error {
            MovementError::Actor(error) => error,
            MovementError::SnapshotMismatch => CarrierError::PositionSnapshotMismatch,
            MovementError::ContextMismatch => CarrierError::PositionContextMismatch,
            _ => CarrierError::PlanConflict,
        })
    }

    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        let facing = match self.direction {
            CardinalStep::North => MovementFacing::North,
            CardinalStep::East => MovementFacing::East,
            CardinalStep::South => MovementFacing::South,
            CardinalStep::West => MovementFacing::West,
        };
        (self.actor, self.expected, self.destination, facing)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::collections::BTreeSet;

    /// A flat world of walkable cells `0..=9` square on `z` in `zs`, minus `blocked`.
    fn world(zs: &[i32], blocked: &[Cell]) -> impl Fn(Cell) -> Ground {
        let zs = zs.to_vec();
        let blocked: BTreeSet<Cell> = blocked.iter().copied().collect();
        move |cell| {
            let (x, y, z) = cell;
            if !(0..10).contains(&x) || !(0..10).contains(&y) || !zs.contains(&z) {
                Ground::Absent
            } else if blocked.contains(&cell) {
                Ground::Blocked
            } else {
                Ground::Walkable
            }
        }
    }

    fn table(tiles: &[(Cell, u8)]) -> FloorTable {
        let mut table = FloorTable::default();
        for ((x, y, z), mask) in tiles {
            table.add(pos(*x, *y, *z).expect("in range"), *mask);
        }
        table
    }

    #[test]
    fn plain_step_stays_on_the_floor() {
        let t = FloorTable::default();
        assert_eq!(
            resolve(&t, world(&[7], &[]), (3, 3, 7), CardinalStep::East),
            Ok((4, 3, 7))
        );
    }

    #[test]
    fn stairs_up_move_one_step_and_one_floor_up() {
        // z 8 -> z 7; the north stair at (3, 3, 8) leads to (3, 2, 7).
        let t = table(&[((3, 3, 8), NORTH)]);
        assert_eq!(
            resolve(&t, world(&[7, 8], &[]), (3, 4, 8), CardinalStep::North),
            Ok((3, 2, 7))
        );
        // Each flag from a stair at (4, 4, 8), entered by the step named.
        for (flag, from, step, to) in [
            (SOUTH, (5, 4, 8), CardinalStep::West, (4, 5, 7)),
            (EAST, (4, 3, 8), CardinalStep::South, (5, 4, 7)),
            (WEST, (3, 4, 8), CardinalStep::East, (3, 4, 7)),
            (SOUTHALT, (5, 4, 8), CardinalStep::West, (4, 6, 7)),
            (EASTALT, (4, 5, 8), CardinalStep::North, (6, 4, 7)),
        ] {
            let t = table(&[((4, 4, 8), flag)]);
            assert_eq!(
                resolve(&t, world(&[7, 8], &[]), from, step),
                Ok(to),
                "flag {flag}"
            );
        }
    }

    #[test]
    fn hole_drops_one_floor_in_place() {
        let t = table(&[((3, 3, 7), DOWN)]);
        assert_eq!(
            resolve(&t, world(&[7, 8], &[]), (2, 3, 7), CardinalStep::East),
            Ok((3, 3, 8))
        );
    }

    #[test]
    fn hole_follows_the_flags_of_the_tile_below_and_the_alt_neighbours() {
        let w = world(&[7, 8], &[]);
        let t = table(&[((3, 3, 7), DOWN), ((3, 3, 8), NORTH)]);
        assert_eq!(
            resolve(&t, &w, (2, 3, 7), CardinalStep::East),
            Ok((3, 4, 8))
        );
        let t = table(&[((3, 3, 7), DOWN), ((3, 2, 8), SOUTHALT)]);
        assert_eq!(
            resolve(&t, &w, (2, 3, 7), CardinalStep::East),
            Ok((3, 1, 8))
        );
        let t = table(&[((3, 3, 7), DOWN), ((2, 3, 8), EASTALT)]);
        assert_eq!(
            resolve(&t, &w, (2, 3, 7), CardinalStep::East),
            Ok((1, 3, 8))
        );
    }

    #[test]
    fn missing_target_leaves_the_player_on_the_entered_tile() {
        let t = table(&[((3, 3, 7), DOWN)]);
        assert_eq!(
            resolve(&t, world(&[7], &[]), (2, 3, 7), CardinalStep::East),
            Ok((3, 3, 7))
        );
        let t = table(&[((3, 3, 7), NORTH)]);
        assert_eq!(
            resolve(&t, world(&[7], &[]), (3, 4, 7), CardinalStep::North),
            Ok((3, 3, 7))
        );
        // Up from z 0 and down from z 15 leave the map.
        let t = table(&[((3, 3, 0), NORTH), ((3, 3, 15), DOWN)]);
        assert_eq!(
            resolve(&t, world(&[0, 15], &[]), (3, 4, 0), CardinalStep::North),
            Ok((3, 3, 0))
        );
        assert_eq!(
            resolve(&t, world(&[0, 15], &[]), (2, 3, 15), CardinalStep::East),
            Ok((3, 3, 15))
        );
    }

    #[test]
    fn blocked_or_absent_step_target_and_blocked_landing_refuse() {
        let t = table(&[((3, 3, 8), NORTH)]);
        assert_eq!(
            resolve(&t, world(&[8], &[(4, 3, 8)]), (3, 3, 8), CardinalStep::East),
            Err(Refusal::Blocked)
        );
        assert_eq!(
            resolve(&t, world(&[8], &[]), (0, 0, 8), CardinalStep::North),
            Err(Refusal::Absent)
        );
        assert_eq!(
            resolve(
                &t,
                world(&[7, 8], &[(3, 2, 7)]),
                (3, 4, 8),
                CardinalStep::North
            ),
            Err(Refusal::Blocked)
        );
    }

    #[test]
    fn successive_floor_changes_follow_and_a_cycle_is_bounded() {
        // Two stacked stairs climb two floors.
        let t = table(&[((3, 3, 9), NORTH), ((3, 2, 8), NORTH)]);
        assert_eq!(
            resolve(&t, world(&[7, 8, 9], &[]), (3, 4, 9), CardinalStep::North),
            Ok((3, 1, 7))
        );
        // A stair up onto a hole whose floor below leads back to the stair never ends.
        let looped = table(&[((3, 3, 8), NORTH), ((3, 2, 7), DOWN), ((3, 2, 8), NORTH)]);
        let w = world(&[7, 8], &[]);
        assert_eq!(
            resolve(&looped, &w, (3, 4, 8), CardinalStep::North),
            Err(Refusal::Chain)
        );
        assert_eq!(looped.validate(&w), Err(pos(3, 2, 7).expect("pos")));
    }

    #[test]
    fn validate_accepts_ordinary_data() {
        let t = table(&[((3, 3, 7), DOWN), ((5, 5, 8), NORTH)]);
        assert_eq!(t.validate(world(&[7, 8], &[])), Ok(()));
        assert_eq!(FloorTable::default().validate(world(&[7], &[])), Ok(()));
    }

    #[test]
    fn palette_keys_resolve_against_the_catalogue() {
        assert_eq!(
            palette_mask(Family::Terrain, "oteryn:terrain.tibia.i293"),
            DOWN
        );
        assert_eq!(
            palette_mask(Family::Item, "oteryn:item.tibia.i855"),
            SOUTHALT
        );
        assert_eq!(palette_mask(Family::Item, "oteryn:item.tibia.i868"), DOWN);
        assert_eq!(
            palette_mask(Family::Item, "donor:crystalserver@r1:item/868"),
            DOWN
        );
        // The two families keep separate tables; a plain, UNKNOWN or foreign key is no change.
        assert_eq!(palette_mask(Family::Item, "oteryn:item.tibia.i100"), 0);
        assert_eq!(palette_mask(Family::Item, "item:box"), 0);
        assert_eq!(palette_mask(Family::Terrain, "terrain:grass"), 0);
    }

    /// The generated table equals the JSON catalogue it was generated from.
    #[test]
    fn catalogue_matches_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/world");
        for (dir, generated) in [("objects", OBJECT), ("terrain", TERRAIN)] {
            let mut found = Vec::new();
            let mut files: Vec<_> = std::fs::read_dir(root.join(dir))
                .expect("catalogue dir")
                .map(|entry| entry.expect("entry").path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(dir) && name.ends_with(".json"))
                })
                .collect();
            files.sort();
            assert!(!files.is_empty(), "{dir}");
            for file in files {
                let value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(file).expect("read")).expect("json");
                for record in value["records"].as_array().expect("records") {
                    let fact = &record["floor_change"];
                    if fact["state"] != "KNOWN" {
                        continue;
                    }
                    let key = record["identity"]["key"].as_str().expect("key");
                    let id: u16 = key
                        .rsplit_once(".i")
                        .expect("id")
                        .1
                        .parse()
                        .expect("numeric id");
                    let bit = match fact["value"].as_str().expect("value") {
                        "down" => DOWN,
                        "north" => NORTH,
                        "south" => SOUTH,
                        "east" => EAST,
                        "west" => WEST,
                        "southalt" => SOUTHALT,
                        "eastalt" => EASTALT,
                        other => panic!("floor_change {other}"),
                    };
                    found.push((id, bit));
                }
            }
            found.sort_unstable();
            assert_eq!(
                found.as_slice(),
                generated,
                "{dir} drifted from the JSON catalogue"
            );
        }
    }
}
