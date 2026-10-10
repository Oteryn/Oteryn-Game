//! MAP-CUTOVER-1b: the production [`MapFacts`] of a bundle World (MAP track packets §2.4).
//!
//! [`BundleFacts`] is built once at boot from the bundle and the served item definitions and is
//! never mutated afterwards:
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
//! - **Overlay.** The Channel overlay stays empty until MAP-CUTOVER-1c, so no added entry has
//!   facts.
//! - **Doors (MAP-DOOR-1).** The only state is the [`DoorBook`]: a plain door placement shows the
//!   open item's facts while open and its transition count is its object revision; every other
//!   object revision is 0.

use super::door::{self, DoorBook, PaletteDoor};
use super::overlay::{AddedItem, TilePos};
use super::view::{self, EntryFacts, MapFacts, placement_key};
use super::{LoadError, palette_appearance};
use oteryn_protocol_oteryn::world_map::{MAX_BASE_ORDINAL, MapDefinition};
use oteryn_world_bundle::bundle::{Family, Manifest, PaletteEntry, Sector, TerrainKind};
use oteryn_world_bundle::sector::Attrs;
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// The served definition of the Item a palette key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDefinition {
    /// The §1.6 index entry of the key: 1 + its Item compact id in the active generation.
    pub reference: NonZeroU32,
    /// `block_solid`; `None` when the definition does not say, which blocks.
    pub solid: Option<bool>,
    pub blocks_projectile: bool,
    pub pickupable: bool,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placement {
    palette: u32,
    bound: bool,
    count: u8,
    sub_type: u16,
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
    doors: DoorBook,
}

const AMBIGUOUS: u32 = u32::MAX;

impl BundleFacts {
    /// The facts of `manifest`'s palette. `item` serves the definition of an Item key.
    pub(crate) fn palette(
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
            doors: DoorBook::default(),
        })
    }

    /// Whether palette entry `index` blocks a tile: a `wall` Terrain or a solid Item. An index
    /// outside the palette blocks.
    pub(crate) fn blocks(&self, index: u32) -> bool {
        self.palette.get(index as usize).is_none_or(|entry| {
            entry.facts.terrain_kind == Some(TerrainKind::Wall)
                || (entry.family == Family::Item && entry.solid)
        })
    }

    /// Records the placements, house tiles and doors of one sector of `manifest`, and adds to
    /// `blocked` the tiles one of whose top-level entries is a `wall` Terrain or a solid Item. A
    /// door entry does not block here: [`DoorBook`] answers it by its current state.
    pub(crate) fn push(
        &mut self,
        manifest: &Manifest,
        sector: &Sector,
        blocked: &mut BTreeSet<TilePos>,
    ) {
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
                let door = if bound(&item.attrs) {
                    PaletteDoor::No
                } else {
                    door
                };
                let closed_solid = self.blocks(item.palette);
                blocks |= closed_solid && door == PaletteDoor::No;
                let Some(key) = u8::try_from(ordinal)
                    .ok()
                    .filter(|ordinal| *ordinal <= MAX_BASE_ORDINAL)
                    .and_then(|ordinal| placement_key(pos, ordinal))
                else {
                    // A door beyond the ordinal reach cannot be named, so it blocks as a wall.
                    blocks |= closed_solid;
                    continue;
                };
                match door {
                    PaletteDoor::Pair { open, open_solid } => {
                        self.doors.push(key, closed_solid, open_solid, open);
                    }
                    PaletteDoor::Sealed => {
                        self.doors.push_sealed();
                        blocks |= closed_solid;
                    }
                    PaletteDoor::No => {}
                }
                let dropped = manifest.dropped_teleports.binary_search(&key).is_ok();
                let placement = Placement {
                    palette: item.palette,
                    bound: dropped || bound(&item.attrs),
                    count: item
                        .attrs
                        .count
                        .filter(|count| (1..=100).contains(count))
                        .unwrap_or(1),
                    sub_type: item.attrs.charges.unwrap_or(0),
                };
                let plain = Placement {
                    palette: item.palette,
                    bound: false,
                    count: 1,
                    sub_type: 0,
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
    pub(crate) fn finish(&mut self) {
        self.placements.sort_unstable_by_key(|(key, _)| *key);
        self.placements.shrink_to_fit();
        self.houses.sort_unstable();
        self.houses.dedup();
        self.houses.shrink_to_fit();
        self.doors.finish();
    }

    /// The doors of the bundle and their current state.
    pub fn doors(&self) -> &DoorBook {
        &self.doors
    }

    fn unique_index(&self, id: u32) -> Option<u32> {
        let found = self.ids.binary_search_by_key(&id, |(id, _)| *id).ok()?;
        Some(self.ids[found].1).filter(|index| *index != AMBIGUOUS)
    }
}

/// A placement attribute that binds the entry or that a pickup cannot represent.
fn bound(attrs: &Attrs) -> bool {
    attrs.action.is_some()
        || attrs.unique.is_some()
        || attrs.door.is_some()
        || attrs.depot.is_some()
        || attrs.teleport.is_some()
        || attrs.text.is_some()
        || attrs.description.is_some()
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
        solid = definition.solid != Some(false);
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

/// Whether the Item palette key `key` is a plain closed door (MAP-DOOR-1), and the facts of its
/// open counterpart, which `item` must serve.
fn palette_door(key: &str, item: &impl Fn(&str) -> Option<ItemDefinition>) -> PaletteDoor {
    let Some(closed) = palette_appearance(key).map(u32::from) else {
        return PaletteDoor::No;
    };
    let Some(open) = door::open_of(closed) else {
        return PaletteDoor::No;
    };
    let Some(open_key) = door::open_key(key, closed, open) else {
        return PaletteDoor::No;
    };
    let Some(definition) = item(&open_key) else {
        return PaletteDoor::Sealed;
    };
    PaletteDoor::Pair {
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
        open_solid: definition.solid != Some(false),
    }
}

impl MapFacts for BundleFacts {
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts> {
        let key = placement_key(pos, ordinal)?;
        let placement = match self.placements.binary_search_by_key(&key, |(key, _)| *key) {
            Ok(found) => self.placements[found].1,
            Err(_) => Placement {
                palette: self.unique_index(id)?,
                bound: false,
                count: 1,
                sub_type: 0,
            },
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
        Some(self.doors.facts(key, facts))
    }

    fn added_entry(&self, _item: &AddedItem) -> Option<EntryFacts> {
        None
    }

    fn house_tile(&self, pos: TilePos) -> bool {
        self.houses.binary_search(&pos).is_ok()
    }

    fn object_revision(&self, placement_key: u64) -> u64 {
        self.doors.revision(placement_key)
    }
}
