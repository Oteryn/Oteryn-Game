//! MAP-DOOR-1: production doors of a bundle World (decision
//! `OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION` §4, MAP track
//! packets §1.2).
//!
//! A placed door uses a closed/open pair or a key-door triple from the Game-owned door
//! registry ([`super::door_registry`]). Its state is scope-ephemeral and per Channel: a
//! restart restores the authored closed/locked predicate. [`DoorBook`] keeps the current
//! locked, closed or open appearance beside the immutable base, with a transition count
//! as its object revision and a matching solid predicate.
//!
//! Key doors also admit their source action binding. Direct USE refuses locked appearances
//! and the reserved bindings 101/1001; matching key USE-WITH can unlock or lock them.
//! Level and quest doors retain their own predicates; house doors and unqualified open
//! counterparts stay outside this book.

#[path = "door/key_use_observation.rs"]
mod key_use_observation;
pub use key_use_observation::KeyDoorOpening;

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
        key_forms: Option<[EntryFacts; 2]>,
        locked: bool,
    },
}

#[derive(Debug)]
struct Door {
    /// The bundle placement key of the closed door entry.
    key: u64,
    key_forms: Option<[EntryFacts; 2]>,
    /// Crystal action id carried by the placed door; key items must match it exactly.
    action: Option<u16>,
    required_level: Option<u32>,
    required_storage: Option<u16>,
    closed_solid: bool,
    open_solid: bool,
    open: EntryFacts,
    initial_state: u64,
    /// Upper bits count transitions; low bits: 0 unlocked closed, 1 open, 2 locked.
    /// One atomic word pins the visible pose and wire revision together.
    state: AtomicU64,
}

impl Door {
    fn solid(&self) -> bool {
        if self.state.load(Ordering::Acquire) & 3 == 1 {
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
    /// A private activity starts from authored door states, never the origin's
    /// mutable Channel door poses or revision counters.
    pub(super) fn activity_copy(&self, inside: impl Fn(TilePos) -> bool) -> Self {
        let mut result = Self::default();
        for door in &self.doors {
            if !super::view::placement_of(door.key).is_some_and(|(pos, _)| inside(pos)) {
                continue;
            }
            result.doors.push(Door {
                key: door.key,
                key_forms: door.key_forms,
                action: door.action,
                required_level: door.required_level,
                required_storage: door.required_storage,
                closed_solid: door.closed_solid,
                open_solid: door.open_solid,
                open: door.open,
                initial_state: door.initial_state,
                state: AtomicU64::new(door.initial_state),
            });
        }
        result
    }

    /// Private activities start from the authored pose, never another scope's toggles.
    pub(super) fn activity_region(&self, inside: impl Fn(TilePos) -> bool) -> Self {
        let doors = self
            .doors
            .iter()
            .filter(|door| super::view::placement_of(door.key).is_some_and(|(pos, _)| inside(pos)))
            .map(|door| Door {
                key: door.key,
                key_forms: door.key_forms,
                action: door.action,
                required_level: door.required_level,
                required_storage: door.required_storage,
                closed_solid: door.closed_solid,
                open_solid: door.open_solid,
                open: door.open,
                state: AtomicU64::new(door.initial_state),
                initial_state: door.initial_state,
            })
            .collect();
        Self {
            doors,
            ..Self::default()
        }
    }
    #[allow(clippy::too_many_arguments, reason = "one authored door record")]
    pub fn push(
        &mut self,
        key: u64,
        closed_solid: bool,
        open_solid: bool,
        open: EntryFacts,
        locked: bool,
        action: Option<u16>,
        required_level: Option<u32>,
        required_storage: Option<u16>,
    ) {
        self.doors.push(Door {
            key,
            key_forms: None,
            action,
            required_level,
            required_storage,
            closed_solid,
            open_solid,
            open,
            initial_state: if locked { 2 } else { 0 },
            state: AtomicU64::new(if locked { 2 } else { 0 }),
        });
    }

    /// Qualified locked and unlocked closed appearances for the last registered key door.
    pub(super) fn set_key_forms(&mut self, forms: Option<[EntryFacts; 2]>) {
        if let Some(door) = self.doors.last_mut() {
            door.key_forms = forms;
        }
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
        self.state(key, pinned).map_or(0, |(_, state)| state >> 2)
    }

    /// `entry`, the closed facts of placement `key`, or the open item's facts while the door is
    /// open; the door's state is read from `pinned` when given.
    pub fn facts(&self, key: u64, entry: EntryFacts, pinned: Option<&DoorStates>) -> EntryFacts {
        match self.state(key, pinned) {
            Some((door, state)) => {
                let face = if state & 3 == 1 {
                    door.open
                } else {
                    door.key_forms
                        .map_or(entry, |forms| forms[usize::from(state & 3 == 2)])
                };
                EntryFacts {
                    definition: face.definition,
                    terrain_kind: face.terrain_kind,
                    appearance_id: face.appearance_id,
                    blocks_projectile: face.blocks_projectile,
                    ..entry
                }
            }
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
    pub fn toggle(
        &self,
        key: u64,
        expected_revision: u64,
        occupied: bool,
        actor_level: u32,
        quest_storage_met: bool,
    ) -> Option<DoorUse> {
        let door = self.find(key)?;
        let current = door.state.load(Ordering::Acquire);
        if current >> 2 != expected_revision {
            return Some(DoorUse::Stale);
        }
        if current & 3 == 2
            || (current & 3 == 0
                && door.required_level.is_none()
                && door.required_storage.is_none()
                && matches!(door.action, Some(101 | 1001)))
        {
            return Some(DoorUse::Locked);
        }
        if door
            .required_level
            .is_some_and(|required| actor_level < required)
        {
            return Some(DoorUse::Locked);
        }
        if door.required_storage.is_some() && !quest_storage_met {
            return Some(DoorUse::Locked);
        }
        let closing = current & 3 == 1;
        if closing && occupied {
            return Some(DoorUse::Occupied);
        }
        let revision = (current >> 2)
            .checked_add(1)
            .filter(|r| *r <= u64::MAX >> 2)?;
        let next = (revision << 2) | u64::from(!closing);
        Some(
            match door
                .state
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    self.version.fetch_add(1, Ordering::AcqRel);
                    DoorUse::Toggled {
                        revision,
                        open: !closing,
                    }
                }
                Err(_) => DoorUse::Stale,
            },
        )
    }

    /// Applies a Crystal key action id to a key door. A matching key toggles the lock and the
    /// visible open state in one revision; a wrong key changes nothing.
    pub fn use_key(&self, key: u64, expected_revision: u64, action: u16) -> Option<DoorUse> {
        let door = self.find(key)?;
        let current = door.state.load(Ordering::Acquire);
        if current >> 2 != expected_revision {
            return Some(DoorUse::Stale);
        }
        if action == 0
            || door.action != Some(action)
            || door.required_level.is_some()
            || door.required_storage.is_some()
            || door.key_forms.is_none()
        {
            return Some(DoorUse::Locked);
        }
        let revision = (current >> 2)
            .checked_add(1)
            .filter(|r| *r <= u64::MAX >> 2)?;
        let opening = current & 3 == 2;
        let next = (revision << 2) | if opening { 1 } else { 2 };
        Some(
            match door
                .state
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    self.version.fetch_add(1, Ordering::AcqRel);
                    DoorUse::Toggled {
                        revision,
                        open: opening,
                    }
                }
                Err(_) => DoorUse::Stale,
            },
        )
    }
}

/// One top-level item remaining on a door tile after a player step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorTileItem<T> {
    pub item: T,
    pub movable: bool,
    pub corpse: bool,
}

/// Crystal/Canary closing_door.lua: relocate movable non-corpses before closing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoorStepOut<T> {
    Nothing,
    Close {
        item_moves: Vec<(T, TilePos)>,
        closed_id: u16,
    },
}

/// Pure player step-out rule. `east_enterable` is the donor's east queryAdd result;
/// otherwise it chooses south. No creature, including another player, may remain.
pub fn door_step_out<T: Copy>(
    open_id: u16,
    position: TilePos,
    creature_present: bool,
    east_enterable: bool,
    items: &[DoorTileItem<T>],
) -> DoorStepOut<T> {
    let Some(closed_id) = open_id.checked_sub(1) else {
        return DoorStepOut::Nothing;
    };
    let closed = u32::from(closed_id);
    if creature_present
        || super::door_registry::level_door_of(closed)
            .or_else(|| super::door_registry::quest_door_of(closed))
            != Some(u32::from(open_id))
    {
        return DoorStepOut::Nothing;
    }
    let destination = if east_enterable {
        position.x.checked_add(1).map(|x| TilePos { x, ..position })
    } else {
        position.y.checked_add(1).map(|y| TilePos { y, ..position })
    };
    let mut item_moves = Vec::new();
    for item in items.iter().filter(|item| item.movable && !item.corpse) {
        let Some(to) = destination else {
            return DoorStepOut::Nothing;
        };
        item_moves.push((item.item, to));
    }
    DoorStepOut::Close {
        item_moves,
        closed_id,
    }
}

#[cfg(test)]
mod step_out_tests {
    use super::*;

    const AT: TilePos = TilePos {
        x: 6,
        y: 7,
        floor: -7,
    };

    #[test]
    fn creature_on_door_prevents_closing_and_item_moves() {
        assert_eq!(
            door_step_out(
                5114,
                AT,
                true,
                true,
                &[DoorTileItem {
                    item: 1,
                    movable: true,
                    corpse: false
                }]
            ),
            DoorStepOut::Nothing
        );
    }

    #[test]
    fn empty_quest_and_level_doors_close() {
        for open in [5114, 5112] {
            assert_eq!(
                door_step_out::<u8>(open, AT, false, true, &[]),
                DoorStepOut::Close {
                    item_moves: vec![],
                    closed_id: open - 1
                }
            );
        }
    }

    #[test]
    fn items_move_east_or_south_before_close() {
        let items = [
            DoorTileItem {
                item: 1,
                movable: true,
                corpse: false,
            },
            DoorTileItem {
                item: 2,
                movable: true,
                corpse: false,
            },
        ];
        for (east, destination) in [
            (true, TilePos { x: 7, ..AT }),
            (false, TilePos { y: 8, ..AT }),
        ] {
            assert_eq!(
                door_step_out(5114, AT, false, east, &items),
                DoorStepOut::Close {
                    item_moves: vec![(1, destination), (2, destination)],
                    closed_id: 5113
                }
            );
        }
    }

    #[test]
    fn corpse_and_immovable_items_stay() {
        assert_eq!(
            door_step_out(
                5114,
                AT,
                false,
                true,
                &[
                    DoorTileItem {
                        item: 1,
                        movable: true,
                        corpse: true
                    },
                    DoorTileItem {
                        item: 2,
                        movable: false,
                        corpse: false
                    }
                ]
            ),
            DoorStepOut::Close {
                item_moves: vec![],
                closed_id: 5113
            }
        );
    }

    #[test]
    fn ordinary_door_does_not_self_close() {
        assert_eq!(
            door_step_out::<u8>(1630, AT, false, true, &[]),
            DoorStepOut::Nothing
        );
    }

    #[test]
    fn destination_overflow_never_wraps() {
        assert_eq!(
            door_step_out(
                5114,
                TilePos { x: u16::MAX, ..AT },
                false,
                true,
                &[DoorTileItem {
                    item: 1,
                    movable: true,
                    corpse: false
                }]
            ),
            DoorStepOut::Nothing
        );
    }
}

#[cfg(test)]
mod key_tests {
    use super::*;
    use oteryn_protocol_oteryn::world_map::MapDefinition;

    fn open_facts() -> EntryFacts {
        EntryFacts {
            definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 1630,
            blocks_projectile: false,
            pickupable: false,
            bound: false,
            count: 1,
            sub_type: 0,
        }
    }

    #[test]
    fn matching_key_toggles_and_wrong_or_reserved_keys_do_not() {
        for door_action in [None, Some(101), Some(1001), Some(4322)] {
            let mut doors = DoorBook::default();
            doors.push(7, true, false, open_facts(), true, door_action, None, None);
            doors.set_key_forms(Some([open_facts(); 2]));
            doors.finish();
            assert_eq!(doors.use_key(7, 0, 4321), Some(DoorUse::Locked));
            assert_eq!(doors.revision(7), 0);
        }

        let mut doors = DoorBook::default();
        doors.push(7, true, false, open_facts(), true, Some(4321), None, None);
        doors.set_key_forms(Some([open_facts(); 2]));
        doors.finish();
        assert_eq!(
            doors.use_key(7, 0, 4321),
            Some(DoorUse::Toggled {
                revision: 1,
                open: true,
            })
        );
        assert!(!doors.blocks(TilePos {
            x: 0,
            y: 0,
            floor: 0
        }));
        assert_eq!(doors.use_key(7, 0, 4321), Some(DoorUse::Stale));
    }

    #[test]
    fn key_opened_door_can_close_then_open_without_the_key() {
        let mut doors = DoorBook::default();
        doors.push(7, true, false, open_facts(), true, Some(4321), None, None);
        doors.set_key_forms(Some([open_facts(); 2]));
        doors.finish();
        assert!(matches!(
            doors.use_key(7, 0, 4321),
            Some(DoorUse::Toggled { open: true, .. })
        ));
        assert_eq!(
            doors.toggle(7, 1, false, 1, false),
            Some(DoorUse::Toggled {
                revision: 2,
                open: false
            })
        );
        assert_eq!(
            doors.toggle(7, 2, false, 1, false),
            Some(DoorUse::Toggled {
                revision: 3,
                open: true
            })
        );
        assert_eq!(
            doors.use_key(7, 3, 4321),
            Some(DoorUse::Toggled {
                revision: 4,
                open: false
            })
        );
        assert_eq!(doors.toggle(7, 4, false, 1, false), Some(DoorUse::Locked));
    }

    #[test]
    fn reserved_binding_does_not_turn_an_authored_closed_door_into_a_locked_door() {
        for action in [101, 1001] {
            let mut doors = DoorBook::default();
            doors.push(
                7,
                true,
                false,
                open_facts(),
                false,
                Some(action),
                None,
                None,
            );
            doors.set_key_forms(Some([
                EntryFacts {
                    appearance_id: 1629,
                    ..open_facts()
                },
                EntryFacts {
                    appearance_id: 1628,
                    ..open_facts()
                },
            ]));
            doors.finish();
            assert_eq!(doors.facts(7, open_facts(), None).appearance_id, 1629);
            assert_eq!(doors.toggle(7, 0, false, 1, false), Some(DoorUse::Locked));
            assert_eq!(
                doors.use_key(7, 0, action),
                Some(DoorUse::Toggled {
                    revision: 1,
                    open: false
                })
            );
            assert_eq!(doors.facts(7, open_facts(), None).appearance_id, 1628);
            assert_eq!(
                doors.use_key(7, 1, action),
                Some(DoorUse::Toggled {
                    revision: 2,
                    open: true
                })
            );
            assert_eq!(
                doors.toggle(7, 2, false, 1, false),
                Some(DoorUse::Toggled {
                    revision: 3,
                    open: false
                })
            );
            assert_eq!(doors.toggle(7, 3, false, 1, false), Some(DoorUse::Locked));
        }
    }

    #[test]
    fn a_key_cannot_bypass_a_quest_or_level_door() {
        for (level, storage) in [(Some(50), None), (None, Some(4321))] {
            let mut doors = DoorBook::default();
            doors.push(
                7,
                true,
                false,
                open_facts(),
                false,
                Some(4321),
                level,
                storage,
            );
            doors.set_key_forms(Some([open_facts(); 2]));
            doors.finish();
            assert_eq!(doors.use_key(7, 0, 4321), Some(DoorUse::Locked));
            assert_eq!(doors.revision(7), 0);
        }
    }

    #[test]
    fn authored_level_and_quest_actions_keep_their_own_predicates()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::map::{
            boot::tests::bundle_with_items,
            facts::{BundleFacts, ItemDefinition},
        };
        for (appearance, action, level, storage_met, opens) in [
            (1646, None, 1, false, false),
            (1646, Some(0), 1, false, false),
            (1646, Some(1001), 0, false, false),
            (1646, Some(1001), 1, false, true),
            (1646, Some(999), 0, false, true),
            (1642, None, 1, true, false),
            (1642, Some(0), 1, true, false),
            (1642, Some(1001), 1, false, false),
            (1642, Some(1001), 1, true, true),
            (5745, Some(10089), 1, false, false),
            (5745, Some(10089), 1, true, true),
            (51285, Some(10089), 1, false, false),
            (51285, Some(10089), 1, true, true),
            (51287, Some(10089), 1, false, false),
            (51287, Some(10089), 1, true, true),
        ] {
            let key = format!("oteryn:item.tibia.i{appearance}");
            let (bytes, _) = bundle_with_items(&key, "oteryn:item.tibia.i1000");
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            for tile in data.sectors.iter_mut().flat_map(|sector| &mut sector.tiles) {
                if tile.x == 6 {
                    tile.items.last_mut().ok_or("door")?.attrs.action = action;
                }
            }
            let mut facts = BundleFacts::palette(&data.manifest, |name| {
                let is_closed = name == key;
                Some(ItemDefinition {
                    reference: std::num::NonZeroU32::new(if is_closed { 1 } else { 2 })?,
                    solid: Some(is_closed),
                    blocks_projectile: is_closed,
                    pickupable: false,
                    door: is_closed,
                })
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut blocked = Default::default();
            for sector in &data.sectors {
                facts.push(&data.manifest, sector, &mut blocked);
            }
            let result = facts.doors().toggle(
                crate::map::boot::tests::door_key(),
                0,
                false,
                level,
                storage_met,
            );
            assert_eq!(
                matches!(result, Some(DoorUse::Toggled { .. })),
                opens,
                "{appearance} {action:?} level={level} storage={storage_met}: {result:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn quest_door_requires_its_durable_storage_predicate() {
        let mut doors = DoorBook::default();
        doors.push(
            7,
            true,
            false,
            open_facts(),
            false,
            Some(4500),
            None,
            Some(4500),
        );
        doors.set_key_forms(Some([open_facts(); 2]));
        doors.finish();
        assert_eq!(
            doors.toggle(7, 0, false, u32::MAX, false),
            Some(DoorUse::Locked)
        );
        assert_eq!(doors.revision(7), 0);
        assert_eq!(
            doors.toggle(7, 0, false, u32::MAX, true),
            Some(DoorUse::Toggled {
                revision: 1,
                open: true,
            })
        );
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

#[cfg(test)]
#[path = "door/key_use_observation_tests.rs"]
mod key_use_observation_tests;
