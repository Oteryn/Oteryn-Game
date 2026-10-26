//! MAP-CUTOVER-1b: the production [`MapFacts`] of a bundle World (MAP track packets §2.4).
//!
//! [`BundleFacts`] is built once at boot from the bundle and the served item definitions;
//! only its Channel-local door and furniture states change afterwards:
//!
//! - **References.** A Terrain entry is `terrain_definition_ref` 1 + its palette `id`. An Item
//!   entry is `item_definition_ref` 1 + its palette `id`, which must equal the §1.6 index entry
//!   of its key; a key the index lacks or places elsewhere refuses the boot.
//! - **Definitions.** Appearance comes from the palette key ([`view::appearance_id`]);
//!   `blocks_projectile` and pickup eligibility come from the served item definition.
//! - **Placements.** A top-level entry with an action, unique, door, depot, teleport, text or
//!   description attribute, or a dropped teleport, is bound; `count` is its stack count in
//!   `1..=100`, else 1, and `sub_type` its charges, else 0. A placement whose compact id names both
//!   an Item and a Terrain record keeps its palette index, because the base keeps the id alone.
//! - **Pickup.** The original attributes and exact pickup-wrapper decision are retained
//!   separately; the normalized view facts above do not change.
//! - **Overlay.** The Channel overlay stays empty until MAP-CUTOVER-1c, so no added entry has
//!   facts.
//! - **Doors (MAP-DOOR-1).** A plain door placement shows the open item's facts while open;
//!   its transition count is its object revision.
//! - **Rotation (PARITY-ITEM-TRANSFORMS-1).** Unbound, fixed furniture keeps its replacement
//!   facts and revision in the same per-Channel fact overlay. Snapshots pin both together.
//!   House tiles and durable/pickupable items are refused; restart restores base orientations.

use super::door::{self, DoorBook, DoorStates, PaletteDoor};
use super::floor::palette_mask;
use super::overlay::{AddedItem, TilePos, pickup};
use super::view::{self, EntryFacts, MapFacts, placement_key};
use super::{LoadError, palette_appearance};
use oteryn_protocol_oteryn::world_map::{MAX_BASE_ORDINAL, MapDefinition};
use oteryn_world_bundle::bundle::{Family, Manifest, PaletteEntry, Sector, TerrainKind};
use oteryn_world_bundle::sector::Attrs;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

mod quest_callbacks;
use quest_callbacks::CallbackStates;

type RotationStates = BTreeMap<u64, (u64, EntryFacts)>;

/// The served definition of the Item a palette key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDefinition {
    /// The §1.6 index entry of the key: 1 + its Item compact id in the active generation.
    pub reference: NonZeroU32,
    /// `block_solid`; `None` when the definition does not say, which blocks.
    pub solid: Option<bool>,
    pub blocks_projectile: bool,
    pub pickupable: bool,
    /// The content profile's `item_type` is `Door` (MAP-DOOR-1): the only thing that makes an
    /// Item a door, whatever its appearance id.
    pub door: bool,
}

/// Why the bundle's palette cannot be served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactsRefusal {
    Load(LoadError),
    /// A palette Item key that the §1.6 index lacks or places at another `id`.
    ItemReference,
}

impl From<LoadError> for FactsRefusal {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

/// The facts of one palette entry, and whether an Item entry is solid.
#[derive(Debug, Clone, Copy)]
struct PaletteFacts {
    family: Family,
    id: u32,
    facts: EntryFacts,
    solid: bool,
    door: PaletteDoor,
}

/// The per-placement facts that differ from the palette entry's.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placement {
    palette: u32,
    bound: bool,
    count: u8,
    sub_type: u16,
    // MAP-PICKUP-1: preserve the original attributes separately from the wire normalization.
    attrs: Attrs,
    pickup: Result<u32, pickup::Ineligible>,
}

/// The production map facts of one bundle World.
#[derive(Debug, Default)]
pub struct BundleFacts {
    palette: Vec<PaletteFacts>,
    /// `(compact id, palette index)` ascending by id, for the ids only one palette entry has.
    ids: Vec<(u32, u32)>,
    /// `(placement key, placement)` ascending by key.
    placements: Vec<(u64, Placement)>,
    /// Ascending.
    houses: Vec<TilePos>,
    house_doors: Vec<(TilePos, u32, u8)>,
    doors: DoorBook,
    /// Scope-ephemeral fact overrides, like doors; never an ItemInstance owner.
    rotations: Mutex<Arc<RotationStates>>,
    callbacks: Mutex<Arc<CallbackStates>>,
}

const AMBIGUOUS: u32 = u32::MAX;

impl BundleFacts {
    /// The facts of `manifest`'s palette. `item` serves the definition of an Item key.
    pub fn palette(
        manifest: &Manifest,
        item: impl Fn(&str) -> Option<ItemDefinition>,
    ) -> Result<Self, FactsRefusal> {
        let palette = manifest
            .palette
            .iter()
            .map(|entry| palette_facts(entry, &item))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids: Vec<(u32, u32)> = palette
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.id, index as u32))
            .collect();
        ids.sort_unstable();
        let mut unique: Vec<(u32, u32)> = Vec::with_capacity(ids.len());
        for (id, index) in ids {
            match unique.last_mut() {
                Some(last) if last.0 == id => last.1 = AMBIGUOUS,
                _ => unique.push((id, index)),
            }
        }
        Ok(Self {
            palette,
            ids: unique,
            placements: Vec::new(),
            houses: Vec::new(),
            house_doors: Vec::new(),
            doors: DoorBook::default(),
            rotations: Mutex::default(),
            callbacks: Mutex::default(),
        })
    }

    /// Whether palette entry `index` blocks a tile: a `wall` Terrain or a solid Item. An index
    /// outside the palette blocks.
    pub fn blocks(&self, index: u32) -> bool {
        self.palette.get(index as usize).is_none_or(|entry| {
            entry.facts.terrain_kind == Some(TerrainKind::Wall)
                || (entry.family == Family::Item && entry.solid)
        })
    }

    /// Records the placements, house tiles and doors of one sector of `manifest`, and adds to
    /// `blocked` the tiles one of whose top-level entries is a `wall` Terrain or a solid Item. A
    /// door entry does not block here: [`DoorBook`] answers it by its current state.
    pub fn push(&mut self, manifest: &Manifest, sector: &Sector, blocked: &mut BTreeSet<TilePos>) {
        for tile in &sector.tiles {
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            if tile.house != 0 {
                self.houses.push(pos);
            }
            let top = tile.items.iter().filter(|item| item.depth == 0);
            let mut blocks = false;
            for (ordinal, item) in top.enumerate() {
                let door = self
                    .palette
                    .get(item.palette as usize)
                    .map_or(PaletteDoor::No, |entry| entry.door);
                // Preserve the source House and door numbers before ordinary Channel door
                // classification deliberately seals House doors.
                if tile.house != 0
                    && door != PaletteDoor::No
                    && let Some(number) = item.attrs.door.filter(|number| *number != 0)
                {
                    self.house_doors.push((pos, tile.house, number));
                }
                let key = u8::try_from(ordinal)
                    .ok()
                    .filter(|ordinal| *ordinal <= MAX_BASE_ORDINAL)
                    .and_then(|ordinal| placement_key(pos, ordinal));
                // A dropped teleport is a bound, non-materialized placement, never a door.
                let dropped =
                    key.is_some_and(|key| manifest.dropped_teleports.binary_search(&key).is_ok());
                // Only source key doors may use an action number without a quest binding.
                // All other attributes retain their existing owner and remain sealed here.
                let key_action = matches!(door, PaletteDoor::Pair { key_door: true, .. })
                    && !bound_without_action(&item.attrs);
                let door = if dropped || (bound(&item.attrs) && !key_action) {
                    PaletteDoor::No
                } else if tile.house != 0 && matches!(door, PaletteDoor::Pair { .. }) {
                    // A house door belongs to the house runtime, never to the channel DoorBook.
                    PaletteDoor::Sealed
                } else {
                    door
                };
                let closed_solid = self.blocks(item.palette);
                blocks |= closed_solid && door == PaletteDoor::No;
                let Some(key) = key else {
                    // A door beyond the ordinal reach cannot be named, so it blocks as a wall.
                    blocks |= closed_solid;
                    continue;
                };
                match door {
                    PaletteDoor::Pair {
                        open,
                        open_solid,
                        locked,
                        ..
                    } => {
                        let locked = locked || matches!(item.attrs.action, Some(101 | 1001));
                        self.doors.push(key, closed_solid, open_solid, open, locked);
                    }
                    PaletteDoor::Sealed => {
                        self.doors.push_sealed();
                        blocks |= closed_solid;
                    }
                    PaletteDoor::No => {}
                }
                let placement = Placement {
                    palette: item.palette,
                    bound: dropped || bound(&item.attrs),
                    count: item
                        .attrs
                        .count
                        .filter(|count| (1..=100).contains(count))
                        .unwrap_or(1),
                    sub_type: item.attrs.charges.unwrap_or(0),
                    attrs: item.attrs.clone(),
                    pickup: pickup::eligibility(
                        tile,
                        sector.floor,
                        ordinal as u8,
                        self.palette
                            .get(item.palette as usize)
                            .is_some_and(|p| p.facts.pickupable),
                        &manifest.dropped_teleports,
                    )
                    .map(|entry| entry.quantity),
                };
                let plain = Placement {
                    palette: item.palette,
                    bound: false,
                    count: 1,
                    sub_type: 0,
                    attrs: Attrs::default(),
                    pickup: self
                        .palette
                        .get(item.palette as usize)
                        .filter(|p| p.facts.pickupable)
                        .map_or(Err(pickup::Ineligible::NotPickupable), |_| Ok(1)),
                };
                let ambiguous = self
                    .palette
                    .get(item.palette as usize)
                    .is_none_or(|entry| self.unique_index(entry.id) != Some(item.palette));
                if placement != plain || ambiguous {
                    self.placements.push((key, placement));
                }
            }
            if blocks {
                blocked.insert(pos);
            }
        }
    }

    /// Orders the recorded placements and house tiles for lookup.
    pub fn finish(&mut self) {
        self.placements.sort_unstable_by_key(|(key, _)| *key);
        self.placements.shrink_to_fit();
        self.houses.sort_unstable();
        self.houses.dedup();
        self.houses.shrink_to_fit();
        self.house_doors.sort_unstable();
        self.house_doors.shrink_to_fit();
        self.doors.finish();
    }

    /// The one source door at this tile of this exact House. Ambiguous tiles grant nothing.
    pub fn house_door(&self, pos: TilePos, house: u32) -> Option<u32> {
        let at = self.house_doors.partition_point(|(tile, _, _)| *tile < pos);
        let (tile, found, number) = *self.house_doors.get(at)?;
        if tile != pos
            || found != house
            || self
                .house_doors
                .get(at + 1)
                .is_some_and(|(tile, _, _)| *tile == pos)
        {
            return None;
        }
        Some(u32::from(number))
    }

    /// The doors of the bundle and their current state.
    pub fn doors(&self) -> &DoorBook {
        &self.doors
    }

    /// These facts with every door's state read once (Codex 4239177726): a map composition
    /// reads them so that each door's appearance and its object revision come from one state,
    /// whatever `USE` commits while it runs.
    pub fn pinned(&self) -> PinnedFacts<'_> {
        PinnedFacts {
            facts: self,
            callbacks: self.callbacks.lock().ok().map(|s| Arc::clone(&s)),
            doors: self.doors.snapshot(),
            rotations: self.rotations.lock().ok().map(|states| Arc::clone(&states)),
        }
    }

    /// Commits a rotation resolved by the Channel owner from admitted Content. Fixed furniture
    /// only: pickup and house custody never acquire a changed definition through this overlay.
    /// The expected revision and facts are compared together, so a retry cannot rotate twice.
    pub fn rotate_base(
        &self,
        key: u64,
        expected_revision: u64,
        before: EntryFacts,
        after: EntryFacts,
    ) -> oteryn_protocol_oteryn::world_object::UseDisposition {
        use oteryn_protocol_oteryn::world_object::UseDisposition;
        let Some((pos, _)) = view::placement_of(key) else {
            return UseDisposition::NothingToUse;
        };
        if self.house_tile(pos)
            || self.doors.contains(key)
            || before.bound
            || before.pickupable
            || after.bound
            || after.pickupable
            || before.count != 1
            || after.count != 1
            || before.sub_type != 0
            || after.sub_type != 0
            || before.blocks_projectile != after.blocks_projectile
        {
            return UseDisposition::NothingToUse;
        }
        let Ok(mut states) = self.rotations.lock() else {
            return UseDisposition::Rejected;
        };
        let current = states.get(&key);
        if current.map_or(0, |(revision, _)| *revision) != expected_revision
            || current.is_some_and(|(_, facts)| *facts != before)
        {
            return UseDisposition::StaleState;
        }
        let Some(revision) = expected_revision.checked_add(1) else {
            return UseDisposition::Rejected;
        };
        // Conservative 512 bytes per changed placement, including B-tree slack and a pinned
        // snapshot retained across a write. Refuse new entries at the map overlay budget.
        if current.is_none() && states.len() >= super::overlay::OVERLAY_BUDGET_BYTES / 512 {
            return UseDisposition::Exhausted;
        }
        Arc::make_mut(&mut states).insert(key, (revision, after));
        UseDisposition::Committed
    }

    fn entry(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
        doors: Option<&DoorStates>,
        rotations: &RotationStates,
    ) -> Option<EntryFacts> {
        let key = placement_key(pos, ordinal)?;
        let plain;
        let placement = match self.placements.binary_search_by_key(&key, |(key, _)| *key) {
            Ok(found) => &self.placements[found].1,
            Err(_) => {
                plain = Placement {
                    palette: self.unique_index(id)?,
                    bound: false,
                    count: 1,
                    sub_type: 0,
                    attrs: Attrs::default(),
                    pickup: Ok(1),
                };
                &plain
            }
        };
        let entry = self.palette.get(placement.palette as usize)?;
        if entry.id != id {
            return None;
        }
        let facts = EntryFacts {
            bound: placement.bound,
            count: u32::from(placement.count),
            sub_type: u32::from(placement.sub_type),
            ..entry.facts
        };
        Some(rotations.get(&key).map_or_else(
            || self.doors.facts(key, facts, doors),
            |(_, rotated)| *rotated,
        ))
    }

    /// Authored town binding of this top-level placement in the admitted bundle.
    /// Reach, visibility and current session authority remain dispatch responsibilities.
    pub fn depot_at(&self, pos: TilePos, ordinal: u8) -> Option<u16> {
        let key = placement_key(pos, ordinal)?;
        let found = self
            .placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()?;
        self.placements.get(found)?.1.attrs.depot
    }

    /// MAP-PICKUP-1: the exact boot-time wrapper decision, before view normalization.
    /// The caller first verifies that the base tile holds this ordinal and compact id.
    pub fn pickup_entry(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
    ) -> Result<pickup::EligibleEntry, pickup::Ineligible> {
        let missing = pickup::Ineligible::NotTopLevel;
        let key = placement_key(pos, ordinal).ok_or(missing)?;
        let (palette, quantity) = match self.placements.binary_search_by_key(&key, |(key, _)| *key)
        {
            Ok(found) => {
                let placement = &self.placements[found].1;
                (placement.palette, placement.pickup?)
            }
            Err(_) => {
                let palette = self.unique_index(id).ok_or(missing)?;
                if !self
                    .palette
                    .get(palette as usize)
                    .is_some_and(|p| p.facts.pickupable)
                {
                    return Err(pickup::Ineligible::NotPickupable);
                }
                (palette, 1)
            }
        };
        if self
            .palette
            .get(palette as usize)
            .is_none_or(|entry| entry.id != id)
        {
            return Err(missing);
        }
        Ok(pickup::EligibleEntry {
            placement_key: key,
            origin: pos,
            ordinal,
            palette,
            quantity,
        })
    }

    fn unique_index(&self, id: u32) -> Option<u32> {
        let found = self.ids.binary_search_by_key(&id, |(id, _)| *id).ok()?;
        Some(self.ids[found].1).filter(|index| *index != AMBIGUOUS)
    }
}

/// A placement attribute that binds the entry or that a pickup cannot represent.
fn bound(attrs: &Attrs) -> bool {
    attrs.action.is_some() || bound_without_action(attrs)
}

fn bound_without_action(attrs: &Attrs) -> bool {
    attrs.unique.is_some()
        || attrs.door.is_some()
        || attrs.depot.is_some()
        || attrs.teleport.is_some()
        || attrs.text.is_some()
        || attrs.description.is_some()
}

/// The catalogue `floor_change` WorldObjects whose collision fact is `blocks_movement = true`
/// (ascending Tibia appearance ids; kept equal to the JSON by a test). Every other floor-change
/// record is walkable or has no collision fact (Terrain).
const SOLID_FLOOR_CHANGE: &[u16] = &[17230, 24806, 24810, 24812, 28655];

/// Whether the catalogue says the entry `key` names changes floors and can be walked onto: the
/// compiler routes stairs and holes to Item entries that production definitions leave without a
/// solidity fact, and a solid floor-change object (`SOLID_FLOOR_CHANGE`) is not one of them.
fn walkable_floor_change(key: &str) -> bool {
    palette_appearance(key).is_some_and(|id| {
        palette_mask(Family::Item, key) != 0 && SOLID_FLOOR_CHANGE.binary_search(&id).is_err()
    })
}

/// The `border` Terrain records whose catalogue `walkable` fact is not KNOWN true (ascending
/// Tibia appearance ids; kept equal to `content/world/terrain` by a test): shorelines, cliffs and
/// other borders that must keep blocking when the bundle states no solidity for them.
const UNWALKABLE_BORDERS: &[u16] = &[
    439, 440, 441, 442, 443, 444, 445, 446, 447, 448, 449, 450, 568, 569, 570, 4633, 4634, 4635,
    4636, 4637, 4638, 4639, 4640, 4641, 4642, 4643, 4644, 4645, 4646, 4647, 4648, 4649, 4650, 4651,
    4652, 6562, 6563, 6627, 6628, 6629, 6630, 6631, 6632, 6633, 6634, 6635, 6636, 6637, 6638, 6639,
    6640, 6641, 6642, 6643, 6644, 6645, 6646, 6647, 6648, 6649, 6650, 6651, 6652, 6653, 6654, 6655,
    6656, 6657, 6658, 6659, 6660, 6661, 6662, 6663, 6664, 6665, 6666, 6667, 6668, 6669, 6670, 6671,
    6672, 6673, 6674, 6675, 6676, 6677, 6678, 6679, 6680, 6681, 6682, 6687, 6688, 6689, 6690, 6691,
    6692, 6693, 6694, 7186, 8317, 8318, 11337, 11349, 11350, 11351, 11516, 11518, 11519, 11520,
    11522, 11523, 11524, 11525, 11527, 11528, 11531, 11534, 11535, 11536, 11537, 11538, 12583,
    12584, 12585, 12586, 12587, 12588, 12589, 12590, 12591, 12592, 12593, 12594, 15597, 15598,
    15599, 15600, 15605, 15606, 15607, 15608, 15612, 15613, 15614, 15615, 15876, 15877, 15886,
    15887, 20168, 21159, 21265, 21266, 21267, 21268, 21269, 21270, 21271, 21272, 21273, 21274,
    21276, 21277, 21278, 21279, 21964, 24889, 24898, 24899, 25643, 26143, 28854, 28912, 30614,
    30615, 31171, 31172, 31173, 31174, 31175, 31176, 31177, 31178, 31179, 31180, 31181, 31182,
    31454, 31455, 31456, 31457, 31458, 31459, 31460, 31461, 32418, 32572, 32726, 38112, 38113,
    38115, 38116, 38117, 38118, 38119, 38120, 38121, 38122, 38123, 38124, 38125, 38126, 38127,
    38128, 38129, 38130, 38131, 38132, 38133, 38134, 38135, 38136, 38137, 38138, 38140, 38141,
    38142, 38143, 38144, 38724, 38725, 38726, 38727, 38728, 38729, 38730, 38731, 38732, 38733,
    38734, 38735,
];

/// Whether the bundle routes the Item entry to a Terrain record that does not block by itself:
/// the compiler routes the World's grounds and borders to Item keys whose production definitions
/// leave them without a solidity fact. A `ground` never blocks by itself (the base tile's ground
/// already decides whether the cell can be walked on); a `border` only when the terrain
/// catalogue proves it walkable (`UNWALKABLE_BORDERS`), so an unwalkable or unknown border, a
/// wall, any other Terrain kind and every other Item without the fact still block.
fn terrain_floor(entry: &PaletteEntry) -> bool {
    let Some(terrain) = entry.terrain.as_ref() else {
        return false;
    };
    match terrain.kind {
        TerrainKind::Ground => true,
        TerrainKind::Border => {
            terrain.walkable != Some(false)
                && palette_appearance(&entry.key)
                    .is_some_and(|id| UNWALKABLE_BORDERS.binary_search(&id).is_err())
        }
        _ => false,
    }
}

fn palette_facts(
    entry: &PaletteEntry,
    item: &impl Fn(&str) -> Option<ItemDefinition>,
) -> Result<PaletteFacts, FactsRefusal> {
    let reference = entry
        .id
        .checked_add(1)
        .and_then(NonZeroU32::new)
        .ok_or(FactsRefusal::ItemReference)?;
    let mut facts = EntryFacts {
        definition: MapDefinition::Terrain(reference),
        terrain_kind: entry.terrain.as_ref().map(|terrain| terrain.kind),
        appearance_id: view::appearance_id(
            &entry.key,
            palette_appearance(&entry.key).map(u32::from),
        ),
        blocks_projectile: false,
        pickupable: false,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let mut solid = false;
    let mut door = PaletteDoor::No;
    if entry.family == Family::Item {
        let definition = item(&entry.key)
            .filter(|definition| definition.reference == reference)
            .ok_or(FactsRefusal::ItemReference)?;
        facts.definition = MapDefinition::Item(reference);
        facts.blocks_projectile = definition.blocks_projectile;
        facts.pickupable = definition.pickupable;
        solid = definition
            .solid
            .unwrap_or_else(|| !walkable_floor_change(&entry.key) && !terrain_floor(entry));
        door = palette_door(&entry.key, item);
    }
    Ok(PaletteFacts {
        family: entry.family,
        id: entry.id,
        facts,
        solid,
        door,
    })
}

/// Whether the Item palette key `key` is a qualified closed/locked door, and the facts of its
/// open counterpart, which `item` must serve. The content profile decides doorhood
/// (`item_type` Door); the Game-owned registry only pairs a door with its open item.
fn palette_door(key: &str, item: &impl Fn(&str) -> Option<ItemDefinition>) -> PaletteDoor {
    if !item(key).is_some_and(|definition| definition.door) {
        return PaletteDoor::No;
    }
    let Some(closed) = palette_appearance(key).map(u32::from) else {
        return PaletteDoor::Sealed;
    };
    let key_door = super::door_registry::key_door_of(closed);
    let Some(open) = key_door
        .map(|(open, _)| open)
        .or_else(|| door::open_of(closed))
    else {
        return PaletteDoor::Sealed;
    };
    let Some(open_key) = door::open_key(key, closed, open) else {
        return PaletteDoor::Sealed;
    };
    // The closed state must itself block movement and projectiles.
    match item(key) {
        Some(closed_def) if closed_def.solid != Some(false) && closed_def.blocks_projectile => {}
        _ => return PaletteDoor::Sealed,
    }
    let Some(definition) = item(&open_key) else {
        return PaletteDoor::Sealed;
    };
    // Qualified facts only: an item the profile does not describe reads `solid: None` and
    // blocks projectiles, and must not pass as a walkable open doorway.
    if definition.solid != Some(false) || definition.blocks_projectile {
        return PaletteDoor::Sealed;
    }
    PaletteDoor::Pair {
        key_door: key_door.is_some(),
        locked: key_door.is_some_and(|(_, locked)| locked),
        open: EntryFacts {
            definition: MapDefinition::Item(definition.reference),
            terrain_kind: None,
            appearance_id: view::appearance_id(
                &open_key,
                palette_appearance(&open_key).map(u32::from),
            ),
            blocks_projectile: definition.blocks_projectile,
            pickupable: false,
            bound: false,
            count: 1,
            sub_type: 0,
        },
        open_solid: false,
    }
}

impl MapFacts for BundleFacts {
    fn base_text(&self, key: u64) -> Option<&str> {
        let found = self
            .placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()?;
        self.placements[found].1.attrs.text.as_deref()
    }
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts> {
        if let Some((_, facts)) = self
            .callbacks
            .lock()
            .ok()?
            .changed
            .get(&placement_key(pos, ordinal)?)
        {
            return Some(*facts);
        }
        let rotations = self.rotations.lock().ok()?;
        self.entry(pos, ordinal, id, None, &rotations)
    }

    fn added_entry(&self, _item: &AddedItem) -> Option<EntryFacts> {
        None
    }

    fn house_tile(&self, pos: TilePos) -> bool {
        self.houses.binary_search(&pos).is_ok()
    }

    fn base_hidden(&self, key: u64) -> bool {
        self.callbacks
            .lock()
            .map_or(true, |s| s.hidden.contains(&key))
    }

    fn object_revision(&self, placement_key: u64) -> u64 {
        if let Ok(s) = self.callbacks.lock()
            && let Some((r, _)) = s.changed.get(&placement_key)
        {
            return *r;
        }
        self.rotations.lock().map_or(u64::MAX, |states| {
            states.get(&placement_key).map_or_else(
                || self.doors.revision(placement_key),
                |(revision, _)| *revision,
            )
        })
    }
}

/// [`BundleFacts`] with the door states of one read ([`BundleFacts::pinned`]).
pub struct PinnedFacts<'a> {
    callbacks: Option<Arc<CallbackStates>>,
    facts: &'a BundleFacts,
    doors: DoorStates,
    rotations: Option<Arc<RotationStates>>,
}

impl MapFacts for PinnedFacts<'_> {
    fn base_text(&self, key: u64) -> Option<&str> {
        self.facts.base_text(key)
    }
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts> {
        if let Some((_, facts)) = self
            .callbacks
            .as_ref()?
            .changed
            .get(&placement_key(pos, ordinal)?)
        {
            return Some(*facts);
        }
        self.facts.entry(
            pos,
            ordinal,
            id,
            Some(&self.doors),
            self.rotations.as_deref()?,
        )
    }

    fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts> {
        self.facts.added_entry(item)
    }

    fn house_tile(&self, pos: TilePos) -> bool {
        self.facts.house_tile(pos)
    }

    fn base_hidden(&self, key: u64) -> bool {
        self.callbacks
            .as_ref()
            .is_none_or(|s| s.hidden.contains(&key))
    }

    fn object_revision(&self, placement_key: u64) -> u64 {
        if let Some(s) = &self.callbacks
            && let Some((r, _)) = s.changed.get(&placement_key)
        {
            return *r;
        }
        self.rotations.as_ref().map_or(u64::MAX, |states| {
            states.get(&placement_key).map_or_else(
                || {
                    self.facts
                        .doors
                        .revision_in(placement_key, Some(&self.doors))
                },
                |(revision, _)| *revision,
            )
        })
    }
}

#[cfg(test)]
mod pickup_tests;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::{ChannelId, WorldId};
    use crate::map::boot::{boot, tests as fixture};

    fn item_solid(key: &str) -> bool {
        let entry = PaletteEntry {
            key: key.into(),
            family: Family::Item,
            id: 0,
            terrain: None,
        };
        let definition = |_: &str| {
            Some(ItemDefinition {
                reference: NonZeroU32::MIN,
                solid: None,
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        };
        palette_facts(&entry, &definition).expect("facts").solid
    }

    #[test]
    fn item_transform_rotation_refuses_house_custody_bound_and_pickupable_items() {
        use oteryn_protocol_oteryn::world_object::UseDisposition;
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let key = placement_key(pos, 1).expect("key");
        let before = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 2358,
            blocks_projectile: false,
            pickupable: false,
            bound: false,
            count: 1,
            sub_type: 0,
        };
        let after = EntryFacts {
            appearance_id: 2359,
            ..before
        };
        for (house, before, after) in [
            (true, before, after),
            (
                false,
                EntryFacts {
                    bound: true,
                    ..before
                },
                after,
            ),
            (
                false,
                EntryFacts {
                    pickupable: true,
                    ..before
                },
                after,
            ),
            (
                false,
                before,
                EntryFacts {
                    pickupable: true,
                    ..after
                },
            ),
            (
                false,
                before,
                EntryFacts {
                    blocks_projectile: true,
                    ..after
                },
            ),
            (false, EntryFacts { count: 2, ..before }, after),
            (
                false,
                EntryFacts {
                    sub_type: 7,
                    ..before
                },
                after,
            ),
        ] {
            let mut facts = BundleFacts::default();
            if house {
                facts.houses.push(pos);
            }
            assert_eq!(
                facts.rotate_base(key, 0, before, after),
                UseDisposition::NothingToUse
            );
            assert_eq!(facts.object_revision(key), 0);
            assert!(facts.rotations.lock().expect("states").is_empty());
        }
    }

    /// A solid floor-change object keeps blocking; a plain item without the fact blocks too.
    #[test]
    fn solid_floor_change_objects_without_a_solidity_fact_block() {
        for key in SOLID_FLOOR_CHANGE {
            assert!(item_solid(&format!("oteryn:item.tibia.i{key}")), "{key}");
        }
        assert!(item_solid("oteryn:item.tibia.i100"));
        assert!(!item_solid("oteryn:item.tibia.i1950"));
        assert!(!item_solid("oteryn:item.tibia.i293"));
    }

    /// An Item entry routed to a `ground` Terrain record, or to a `border` the terrain catalogue
    /// proves walkable, does not block without a solidity fact (BOOT_WORLD_BUNDLE_REFUSED on the
    /// real World, where every ground is an Item key); an unwalkable or unknown border, a wall,
    /// any other Terrain kind and a stated solidity keep blocking.
    #[test]
    fn terrain_routed_grounds_and_walkable_borders_without_a_solidity_fact_do_not_block() {
        let facts = |key: &str, kind: Option<TerrainKind>, walkable: Option<bool>| {
            let solid = |solid: Option<bool>| {
                let entry = PaletteEntry {
                    key: key.into(),
                    family: Family::Item,
                    id: 0,
                    terrain: kind.map(|kind| oteryn_world_bundle::bundle::Terrain {
                        kind,
                        walkable,
                        ground_speed: (kind == TerrainKind::Ground).then_some(150),
                    }),
                };
                let definition = move |_: &str| {
                    Some(ItemDefinition {
                        reference: NonZeroU32::MIN,
                        solid,
                        blocks_projectile: false,
                        pickupable: false,
                        door: false,
                    })
                };
                palette_facts(&entry, &definition).expect("facts").solid
            };
            assert!(solid(Some(true)), "stated solid {key} {kind:?}");
            solid(None)
        };
        let ground = Some(TerrainKind::Ground);
        let border = Some(TerrainKind::Border);
        assert!(
            !facts("oteryn:item.tibia.i4408", ground, Some(true)),
            "ground"
        );
        assert!(
            !facts("oteryn:item.tibia.i1109", border, None),
            "walkable border"
        );
        // The catalogue says these borders are not walkable: they keep blocking.
        for id in [30614, 4633, 439] {
            let key = format!("oteryn:item.tibia.i{id}");
            assert!(facts(&key, border, None), "unwalkable border {id}");
        }
        assert!(
            facts("oteryn:item.tibia.i1109", border, Some(false)),
            "stated unwalkable"
        );
        assert!(
            facts("item:border", border, None),
            "border without a catalogue id"
        );
        for kind in [
            TerrainKind::Wall,
            TerrainKind::Roof,
            TerrainKind::Field,
            TerrainKind::Common,
        ] {
            assert!(
                facts("oteryn:item.tibia.i4408", Some(kind), None),
                "{kind:?}"
            );
        }
        assert!(facts("oteryn:item.tibia.i4408", None, None), "plain item");
    }

    /// The unwalkable-border list equals the terrain catalogue's `border` records whose
    /// `walkable` fact is not KNOWN true.
    #[test]
    fn unwalkable_border_list_matches_content() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|root| root.join("workspace-boundaries.toml").is_file())
            .expect("workspace root")
            .join("content/world/terrain");
        let (mut found, mut borders) = (Vec::new(), 0);
        for entry in std::fs::read_dir(dir).expect("dir") {
            let path = entry.expect("entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !(name.starts_with("terrain-") && name.ends_with(".json")) {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
            for record in value["records"].as_array().expect("records") {
                if record["kind"]["value"] != "border" {
                    continue;
                }
                borders += 1;
                let walkable = &record["walkable"];
                if walkable["state"] == "KNOWN" && walkable["value"] == true {
                    continue;
                }
                let key = record["identity"]["key"].as_str().expect("key");
                found.push(
                    key.rsplit_once(".i")
                        .expect("id")
                        .1
                        .parse::<u16>()
                        .expect("n"),
                );
            }
        }
        found.sort_unstable();
        assert!(borders > found.len(), "borders {borders}");
        assert_eq!(found.as_slice(), UNWALKABLE_BORDERS);
    }

    /// The solid list equals the JSON catalogue's `blocks_movement` floor-change objects.
    #[test]
    fn solid_floor_change_list_matches_content() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|root| root.join("workspace-boundaries.toml").is_file())
            .expect("workspace root")
            .join("content/world/objects");
        let mut found = Vec::new();
        for entry in std::fs::read_dir(dir).expect("dir") {
            let path = entry.expect("entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !(name.starts_with("objects-") && name.ends_with(".json")) {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
            for record in value["records"].as_array().expect("records") {
                let blocks = &record["collision"]["blocks_movement"];
                if record["floor_change"]["state"] == "KNOWN"
                    && blocks["state"] == "KNOWN"
                    && blocks["value"] == true
                {
                    let key = record["identity"]["key"].as_str().expect("key");
                    found.push(
                        key.rsplit_once(".i")
                            .expect("id")
                            .1
                            .parse::<u16>()
                            .expect("n"),
                    );
                }
            }
        }
        found.sort_unstable();
        assert_eq!(found.as_slice(), SOLID_FLOOR_CHANGE);
    }

    /// Production item definitions carry no solidity fact: a stair routed to an Item entry is
    /// still walkable through its catalogue floor change, a box without the fact still blocks.
    #[test]
    fn floor_change_items_without_a_solidity_fact_are_walkable() {
        let (bytes, load) = fixture::floor_bundle();
        let unknown = |key: &str| {
            fixture::floor_items(key).map(|definition| ItemDefinition {
                solid: None,
                ..definition
            })
        };
        let world = boot(
            &bytes,
            &fixture::floor_pins(load),
            WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("id"),
            ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2]).expect("id"),
            unknown,
        )
        .expect("boot");
        let at = |x, floor| TilePos { x, y: 0, floor };
        assert!(world.enterable(at(1, -8)), "east stair");
        assert!(!world.enterable(at(1, -7)), "box");
    }
}
