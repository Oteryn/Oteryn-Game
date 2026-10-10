//! MAP-DOOR-1: production doors of a bundle World (decision
//! `OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION` §4, MAP track
//! packets §1.2).
//!
//! A placed door is a plain closed/open item pair of the Canary item table (closed id `C`, open
//! id `C + 1`, [`DOOR_PAIRS`]). Its state is scope-ephemeral and per Channel: a restart builds
//! every door closed again. It lives in [`DoorBook`], a layer beside the immutable base and the
//! (still empty) overlay: the book answers the entry facts of the door's placement (the open
//! item's appearance and definition while open), its object revision (the number of
//! transitions, so an open door has an odd revision) and whether the door's tile is solid.
//!
//! Only a placement with no action, unique, door, depot, teleport, text or description binding is
//! a door. Locked doors (a key `USE` needs KEY-1 and capability 15), quest, level and house doors,
//! gates without a consecutive open item and a pair whose open item the item index does not
//! serve stay sealed: the closed placement is solid and refuses `USE`.

use super::overlay::TilePos;
use super::view::EntryFacts;
use std::sync::atomic::{AtomicU64, Ordering};

/// `(closed id, open id)` of every plain door item of `imports/canary/items-xml/items.xml`: a
/// `type="door"` item named `closed door`, `closed fence gate` or `closed gate` with no
/// `editorsuffix` and no `levelDoor`, followed by the `type="door"` item of the matching `open`
/// name. Ascending by closed id; a test checks it against the item table.
pub(crate) const DOOR_PAIRS: &[(u32, u32)] = &[
    (1629, 1630),
    (1632, 1633),
    (1638, 1639),
    (1640, 1641),
    (1642, 1643),
    (1644, 1645),
    (1651, 1652),
    (1654, 1655),
    (1656, 1657),
    (1658, 1659),
    (1660, 1661),
    (1662, 1663),
    (1669, 1670),
    (1672, 1673),
    (1674, 1675),
    (1676, 1677),
    (1683, 1684),
    (1685, 1686),
    (1689, 1690),
    (1692, 1693),
    (1694, 1695),
    (1698, 1699),
    (2177, 2178),
    (2179, 2180),
    (5082, 5083),
    (5084, 5085),
    (5098, 5099),
    (5100, 5101),
    (5104, 5105),
    (5107, 5108),
    (5109, 5110),
    (5113, 5114),
    (5116, 5117),
    (5118, 5119),
    (5122, 5123),
    (5125, 5126),
    (5127, 5128),
    (5131, 5132),
    (5134, 5135),
    (5137, 5138),
    (5140, 5141),
    (5143, 5144),
    (5278, 5279),
    (5281, 5282),
    (5283, 5284),
    (5285, 5286),
    (5287, 5288),
    (5289, 5290),
    (5514, 5515),
    (5516, 5517),
    (5733, 5734),
    (5736, 5737),
    (5745, 5746),
    (6192, 6193),
    (6195, 6196),
    (6197, 6198),
    (6199, 6200),
    (6201, 6202),
    (6203, 6204),
    (6249, 6250),
    (6252, 6253),
    (6254, 6255),
    (6256, 6257),
    (6258, 6259),
    (6260, 6261),
    (6892, 6893),
    (6894, 6895),
    (6898, 6899),
    (6901, 6902),
    (6903, 6904),
    (6907, 6908),
    (7034, 7035),
    (7036, 7037),
    (7040, 7041),
    (7043, 7044),
    (7045, 7046),
    (7049, 7050),
    (7054, 7055),
    (7056, 7057),
    (7712, 7713),
    (7715, 7716),
    (7717, 7718),
    (7719, 7720),
    (7721, 7722),
    (7723, 7724),
    (7868, 7869),
    (8259, 8260),
    (8261, 8262),
    (8361, 8362),
    (8363, 8364),
    (9352, 9353),
    (9355, 9356),
    (9357, 9358),
    (9359, 9360),
    (9361, 9362),
    (9363, 9364),
    (9552, 9553),
    (9554, 9555),
    (9558, 9559),
    (9561, 9562),
    (9563, 9564),
    (9567, 9568),
    (9859, 9860),
    (9865, 9866),
    (9868, 9869),
    (9874, 9875),
    (11137, 11138),
    (11141, 11142),
    (11144, 11145),
    (11148, 11149),
    (11239, 11240),
    (11242, 11243),
    (11248, 11249),
    (12033, 12034),
    (12035, 12036),
    (12249, 12250),
    (13136, 13137),
    (13143, 13144),
    (15687, 15688),
    (15890, 15891),
    (15892, 15893),
    (17561, 17562),
    (17563, 17564),
    (17567, 17568),
    (17570, 17571),
    (17572, 17573),
    (17576, 17577),
    (17701, 17702),
    (17703, 17704),
    (17707, 17708),
    (17710, 17711),
    (17712, 17713),
    (17716, 17717),
    (17994, 17995),
    (17996, 17997),
    (18000, 18001),
    (18003, 18004),
    (18005, 18006),
    (18009, 18010),
    (20444, 20445),
    (20446, 20447),
    (20450, 20451),
    (20453, 20454),
    (20455, 20456),
    (20459, 20460),
    (22506, 22507),
    (22508, 22509),
    (24541, 24542),
    (24543, 24544),
    (28519, 28520),
    (30041, 30042),
    (30043, 30044),
    (30045, 30046),
    (30047, 30048),
    (31568, 31569),
    (31570, 31571),
    (31663, 31664),
    (31665, 31666),
    (33271, 33272),
    (33273, 33274),
    (34221, 34222),
    (34223, 34224),
    (42627, 42628),
    (42744, 42745),
    (43953, 43954),
    (44916, 44917),
    (48496, 48497),
    (48500, 48501),
    (48523, 48524),
    (48529, 48530),
    (49681, 49682),
    (49687, 49688),
    (50284, 50285),
];

/// The open id of closed door item `closed`, if it is a plain door.
pub(crate) fn open_of(closed: u32) -> Option<u32> {
    let found = DOOR_PAIRS
        .binary_search_by_key(&closed, |(closed, _)| *closed)
        .ok()?;
    Some(DOOR_PAIRS[found].1)
}

/// The palette key of the open item that pairs with the closed door `key` (the key names Item id
/// `closed` as its trailing digits): the same key with `open` in place of `closed`.
pub(crate) fn open_key(key: &str, closed: u32, open: u32) -> Option<String> {
    let prefix = key.strip_suffix(closed.to_string().as_str())?;
    Some(format!("{prefix}{open}"))
}

/// What a bundle palette Item entry is as a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteDoor {
    /// Not a plain closed door.
    No,
    /// A plain closed door whose open item the item index does not serve: it stays sealed.
    Sealed,
    /// A plain closed door and the facts of its open item.
    Pair { open: EntryFacts, open_solid: bool },
}

#[derive(Debug)]
struct Door {
    /// The bundle placement key of the closed door entry.
    key: u64,
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

/// The outcome of one `USE` on a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoorUse {
    /// The door changed state; `revision` is its new object revision and `open` its new state.
    Toggled { revision: u64, open: bool },
    /// `expected_revision` is not the door's current one.
    Stale,
    /// The door is open and an actor stands on its tile.
    Occupied,
}

/// The doors of one Channel's bundle World, ascending by placement key.
#[derive(Debug, Default)]
pub struct DoorBook {
    doors: Vec<Door>,
    sealed: usize,
}

impl DoorBook {
    pub(crate) fn push(
        &mut self,
        key: u64,
        closed_solid: bool,
        open_solid: bool,
        open: EntryFacts,
    ) {
        self.doors.push(Door {
            key,
            closed_solid,
            open_solid,
            open,
            state: AtomicU64::new(0),
        });
    }

    pub(crate) fn push_sealed(&mut self) {
        self.sealed += 1;
    }

    pub(crate) fn finish(&mut self) {
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
        let found = self
            .doors
            .binary_search_by_key(&key, |door| door.key)
            .ok()?;
        Some(&self.doors[found])
    }

    /// Whether placement `key` is a door a player can use.
    pub(crate) fn contains(&self, key: u64) -> bool {
        self.find(key).is_some()
    }

    /// The object revision of placement `key`: 0 for every entry that is no door.
    pub(crate) fn revision(&self, key: u64) -> u64 {
        self.find(key)
            .map_or(0, |door| door.state.load(Ordering::Acquire))
    }

    /// `entry`, the closed facts of placement `key`, or the open item's facts while the door is
    /// open.
    pub(crate) fn facts(&self, key: u64, entry: EntryFacts) -> EntryFacts {
        match self.find(key) {
            Some(door) if door.state.load(Ordering::Acquire) % 2 == 1 => EntryFacts {
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
    pub(crate) fn blocks(&self, pos: TilePos) -> bool {
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

    /// Opens or closes the door of placement `key` when `expected_revision` is its current
    /// revision. `occupied` is whether an actor stands on the door's tile: it keeps an open door
    /// from closing. The change is one compare-and-swap, so concurrent `USE`s of a revision
    /// commit once. `None` when `key` is no door.
    pub(crate) fn toggle(
        &self,
        key: u64,
        expected_revision: u64,
        occupied: bool,
    ) -> Option<DoorUse> {
        let door = self.find(key)?;
        let current = door.state.load(Ordering::Acquire);
        if current != expected_revision {
            return Some(DoorUse::Stale);
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
                Ok(_) => DoorUse::Toggled {
                    revision: next,
                    open: !closing,
                },
                Err(_) => DoorUse::Stale,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_are_ascending_and_consecutive() {
        assert!(DOOR_PAIRS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(DOOR_PAIRS.iter().all(|(closed, open)| *open == closed + 1));
        assert!(DOOR_PAIRS.len() > 100);
        assert_eq!(open_of(1629), Some(1630));
        assert_eq!(open_of(1628), None);
    }

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

    /// The table is exactly the plain door pairs of the vendored Canary item table.
    #[test]
    fn pairs_match_the_canary_item_table() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../imports/canary/items-xml/items.xml");
        let text = std::fs::read_to_string(path)?;
        let mut doors = std::collections::BTreeMap::new();
        for chunk in text.split("<item id=\"").skip(1) {
            let Some((id, rest)) = chunk.split_once('"') else {
                continue;
            };
            let Ok(id) = id.parse::<u32>() else {
                continue;
            };
            let (head, body) = rest.split_once('>').unwrap_or((rest, ""));
            let body = if head.ends_with('/') {
                ""
            } else {
                body.split("</item>").next().unwrap_or("")
            };
            if body.contains("key=\"type\" value=\"door\"") {
                let name = head
                    .split("name=\"")
                    .nth(1)
                    .and_then(|name| name.split('"').next())
                    .unwrap_or("")
                    .to_owned();
                let plain = !head.contains("editorsuffix") && !body.contains("levelDoor");
                doors.insert(id, (name, plain));
            }
        }
        let expected: Vec<(u32, u32)> = doors
            .iter()
            .filter(|(_, (name, plain))| {
                *plain
                    && ["closed door", "closed fence gate", "closed gate"].contains(&name.as_str())
            })
            .filter_map(|(id, (name, _))| {
                let (open, plain) = doors.get(&(id + 1))?;
                (*plain && *open == name.replace("closed", "open")).then_some((*id, id + 1))
            })
            .collect();
        assert_eq!(expected, DOOR_PAIRS);
        Ok(())
    }
}
