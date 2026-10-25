//! MAP-DOOR-1: production doors of a bundle World (decision
//! `OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION` §4, MAP track
//! packets §1.2).
//!
//! A placed door uses a closed/open pair or a key-door triple from the Game-owned door
//! registry ([`super::door_registry`]). Its state is scope-ephemeral and per Channel: a
//! restart restores the authored closed/locked predicate. It lives in [`DoorBook`], a layer beside the immutable base and the
//! (still empty) overlay: the book answers the entry facts of the door's placement (the open
//! item's appearance and definition while open), its object revision (the number of
//! transitions, so an open door has an odd revision) and whether the door's tile is solid.
//!
//! Key doors also admit their source action binding. Direct USE refuses locked appearances
//! and the reserved bindings 101/1001; key USE-WITH is a separate follow-up. Other bindings,
//! quest, level and house doors stay sealed, as do unqualified open counterparts.

pub use super::door_registry::open_of;
use super::overlay::TilePos;
use super::view::EntryFacts;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// The palette key of the open item that pairs with the closed door `key` (the key names Item id
/// `closed` as its trailing digits): the same key with `open` in place of `closed`.
pub fn open_key(key: &str, closed: u32, open: u32) -> Option<String> {
    let prefix = key.strip_suffix(closed.to_string().as_str())?;
    Some(format!("{prefix}{open}"))
}

/// What a bundle palette Item entry is as a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteDoor {
    /// Not a plain closed door.
    No,
    /// A plain closed door whose open item the item index does not serve: it stays sealed.
    Sealed,
    /// A qualified closed or locked door and the facts of its open item.
    Pair {
        open: EntryFacts,
        open_solid: bool,
        key_door: bool,
        locked: bool,
    },
}

#[derive(Debug)]
struct Door {
    /// The bundle placement key of the closed door entry.
    key: u64,
    locked: bool,
    closed_solid: bool,
    open_solid: bool,
    open: EntryFacts,
    /// The number of transitions: the door is open when odd. It is the object revision.
    state: AtomicU64,
}

impl Door {
    fn solid(&self) -> bool {
        if self.state.load(Ordering::Acquire) % 2 == 1 {
            self.open_solid
        } else {
            self.closed_solid
        }
    }
}

/// The state of every door of a [`DoorBook`] at one read ([`DoorBook::snapshot`]), in the book's
/// order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoorStates(Arc<[u64]>);

/// The outcome of one `USE` on a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorUse {
    /// The door changed state; `revision` is its new object revision and `open` its new state.
    Toggled { revision: u64, open: bool },
    /// `expected_revision` is not the door's current one.
    Stale,
    /// Direct use cannot open a locked key door.
    Locked,
    /// The door is open and an actor stands on its tile.
    Occupied,
}

/// The doors of one Channel's bundle World, ascending by placement key.
#[derive(Debug, Default)]
pub struct DoorBook {
    doors: Vec<Door>,
    sealed: usize,
    /// Bumped after every committed transition; keys the shared snapshot.
    version: AtomicU64,
    /// The last snapshot and the version it was read at, shared by every session's read.
    cached: Mutex<Option<(u64, DoorStates)>>,
}

impl DoorBook {
    pub fn push(
        &mut self,
        key: u64,
        closed_solid: bool,
        open_solid: bool,
        open: EntryFacts,
        locked: bool,
    ) {
        self.doors.push(Door {
            key,
            locked,
            closed_solid,
            open_solid,
            open,
            state: AtomicU64::new(0),
        });
    }

    pub fn push_sealed(&mut self) {
        self.sealed += 1;
    }

    pub fn finish(&mut self) {
        self.doors.sort_unstable_by_key(|door| door.key);
        self.doors.shrink_to_fit();
    }

    /// The doors a player can use.
    pub fn len(&self) -> usize {
        self.doors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doors.is_empty()
    }

    /// The plain closed doors that stay sealed because their open item is not served.
    pub fn sealed(&self) -> usize {
        self.sealed
    }

    fn find(&self, key: u64) -> Option<&Door> {
        self.position(key).map(|found| &self.doors[found])
    }

    fn position(&self, key: u64) -> Option<usize> {
        self.doors.binary_search_by_key(&key, |door| door.key).ok()
    }

    /// Every door's state, each read once, for one map composition (Codex 4239177726): the
    /// composition picks a door's facts and later reads its object revision, after the runtime
    /// lock is released, so both must come from the same value or a concurrent `USE` could pair
    /// a closed appearance with an open revision. One `u64` per door.
    pub fn snapshot(&self) -> DoorStates {
        // One shared snapshot per version: sessions reading between transitions clone an Arc
        // instead of each walking every door. A transition racing the build leaves the cached
        // version stale, so the next read rebuilds.
        let version = self.version.load(Ordering::Acquire);
        let Ok(mut cached) = self.cached.lock() else {
            return self.read_states();
        };
        match cached.as_ref() {
            Some((at, states)) if *at == version => states.clone(),
            _ => {
                let states = self.read_states();
                *cached = Some((version, states.clone()));
                states
            }
        }
    }

    fn read_states(&self) -> DoorStates {
        DoorStates(
            self.doors
                .iter()
                .map(|door| door.state.load(Ordering::Acquire))
                .collect(),
        )
    }

    /// The state of placement `key`: from `pinned` when given, else the live state.
    fn state(&self, key: u64, pinned: Option<&DoorStates>) -> Option<(&Door, u64)> {
        let found = self.position(key)?;
        let door = &self.doors[found];
        let state = match pinned {
            Some(states) => *states.0.get(found)?,
            None => door.state.load(Ordering::Acquire),
        };
        Some((door, state))
    }

    /// Whether placement `key` is a door a player can use.
    pub fn contains(&self, key: u64) -> bool {
        self.find(key).is_some()
    }

    /// The object revision of placement `key`: 0 for every entry that is no door.
    pub fn revision(&self, key: u64) -> u64 {
        self.revision_in(key, None)
    }

    /// [`Self::revision`], read from `pinned` when given.
    pub fn revision_in(&self, key: u64, pinned: Option<&DoorStates>) -> u64 {
        self.state(key, pinned).map_or(0, |(_, state)| state)
    }

    /// `entry`, the closed facts of placement `key`, or the open item's facts while the door is
    /// open; the door's state is read from `pinned` when given.
    pub fn facts(&self, key: u64, entry: EntryFacts, pinned: Option<&DoorStates>) -> EntryFacts {
        match self.state(key, pinned) {
            Some((door, state)) if state % 2 == 1 => EntryFacts {
                definition: door.open.definition,
                terrain_kind: door.open.terrain_kind,
                appearance_id: door.open.appearance_id,
                blocks_projectile: door.open.blocks_projectile,
                ..entry
            },
            _ => entry,
        }
    }

    /// Whether a door entry of `pos` is solid in its current state.
    pub fn blocks(&self, pos: TilePos) -> bool {
        let Some(tile) = super::view::placement_key(pos, 0) else {
            return false;
        };
        let start = self.doors.partition_point(|door| door.key < tile);
        self.doors[start..]
            .iter()
            .take_while(|door| door.key <= tile | 0xFF)
            .any(Door::solid)
    }

    /// Whether any door entry of `pos` exists.
    pub fn at(&self, pos: TilePos) -> bool {
        let Some(tile) = super::view::placement_key(pos, 0) else {
            return false;
        };
        let start = self.doors.partition_point(|door| door.key < tile);
        self.doors
            .get(start)
            .is_some_and(|door| door.key <= tile | 0xFF)
    }

    /// The placement keys of every door, ascending.
    pub fn keys(&self) -> impl Iterator<Item = u64> + '_ {
        self.doors.iter().map(|door| door.key)
    }

    /// Refuses a locked door, or opens/closes placement `key` when `expected_revision` is its current
    /// revision. `occupied` is whether an actor stands on the door's tile: it keeps an open door
    /// from closing. The change is one compare-and-swap, so concurrent `USE`s of a revision
    /// commit once. `None` when `key` is no door.
    pub fn toggle(&self, key: u64, expected_revision: u64, occupied: bool) -> Option<DoorUse> {
        let door = self.find(key)?;
        let current = door.state.load(Ordering::Acquire);
        if current != expected_revision {
            return Some(DoorUse::Stale);
        }
        if door.locked {
            return Some(DoorUse::Locked);
        }
        let closing = current % 2 == 1;
        if closing && occupied {
            return Some(DoorUse::Occupied);
        }
        let next = current.checked_add(1)?;
        Some(
            match door
                .state
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    self.version.fetch_add(1, Ordering::AcqRel);
                    DoorUse::Toggled {
                        revision: next,
                        open: !closing,
                    }
                }
                Err(_) => DoorUse::Stale,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_key_replaces_the_trailing_item_id() {
        assert_eq!(
            open_key("oteryn:item.tibia.i1629", 1629, 1630).as_deref(),
            Some("oteryn:item.tibia.i1630")
        );
        assert_eq!(
            open_key("donor:crystalserver@x:item/1629", 1629, 1630).as_deref(),
            Some("donor:crystalserver@x:item/1630")
        );
        assert_eq!(open_key("oteryn:item.tibia.i1628", 1629, 1630), None);
    }
}
