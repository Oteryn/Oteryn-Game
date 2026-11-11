//! Gem Vessel slots of the Winter Update 2026 Gem Atelier (test server 15.33, thread 559).
//!
//! Gems have no domain and there is no Vessel Resonance. Each Wheel node that used to be a
//! Vessel Resonance node is a Gem Vessel node: every fully allocated one opens the next slot of
//! a fixed quality order and grants flat damage and healing. A gem fits any open slot of its
//! own quality.

use super::{AtelierState, GemQuality};

/// One slot per Gem Vessel node of a vocation's Wheel (twelve, three per former domain).
pub const GEM_VESSEL_SLOTS: usize = 12;

/// The fixed quality of each slot, in opening order. The order is not published (spec open
/// question 1); "Gem slots come in a fixed order of quality" with lesser, regular, greater
/// repeated is the reading the evidence supports best. This table is the only place it lives.
pub const GEM_VESSEL_SLOT_QUALITIES: [GemQuality; GEM_VESSEL_SLOTS] = [
    GemQuality::Lesser,
    GemQuality::Regular,
    GemQuality::Greater,
    GemQuality::Lesser,
    GemQuality::Regular,
    GemQuality::Greater,
    GemQuality::Lesser,
    GemQuality::Regular,
    GemQuality::Greater,
    GemQuality::Lesser,
    GemQuality::Regular,
    GemQuality::Greater,
];

/// Flat damage and healing per unlocked Gem Vessel node. Client 15.33: "Each node grants +%1
/// Damage / Healing and unlocks a vessel in the Gem Atelier"; thread 559 #60906: +1.
pub const GEM_VESSEL_FLAT_DAMAGE_HEALING: u32 = 1;

/// Ordered slot contents: the owner-issued identity of the gem in each slot.
pub type GemVessels = [Option<[u8; 16]>; GEM_VESSEL_SLOTS];

/// The empty slot vector.
pub const NO_GEM_VESSELS: GemVessels = [None; GEM_VESSEL_SLOTS];

/// The Wheel conviction keys that used to be Vessel Resonance nodes are Gem Vessel nodes.
#[must_use]
pub fn is_gem_vessel_node(conviction_key: &str) -> bool {
    conviction_key.starts_with("vessel_resonance_")
}

/// The Wheel slots (1..=36) of a vocation's Gem Vessel nodes, in slot order.
#[must_use]
pub fn node_slots(ruleset: &crate::wheel::WheelRuleset, vocation: &str) -> Vec<u8> {
    crate::wheel::base_vocation(vocation)
        .and_then(|v| ruleset.vocation(v))
        .map_or_else(Vec::new, |rules| {
            rules
                .convictions
                .iter()
                .enumerate()
                .filter(|(_, key)| is_gem_vessel_node(key))
                .filter_map(|(index, _)| u8::try_from(index + 1).ok())
                .collect()
        })
}

/// Open slots for a number of fully allocated Gem Vessel nodes.
#[must_use]
pub fn open_slots(unlocked_nodes: u32) -> usize {
    usize::try_from(unlocked_nodes).map_or(GEM_VESSEL_SLOTS, |n| n.min(GEM_VESSEL_SLOTS))
}

/// The number of fully allocated Gem Vessel nodes in a current, eligible allocation.
#[must_use]
pub fn unlocked_nodes(
    ruleset: &crate::wheel::WheelRuleset,
    stages: &crate::wheel::WheelStages,
    vocation: &str,
) -> u32 {
    let Some(rules) = crate::wheel::base_vocation(vocation).and_then(|v| ruleset.vocation(v))
    else {
        return 0;
    };
    let mut keys: Vec<&str> = rules
        .convictions
        .iter()
        .map(String::as_str)
        .filter(|key| is_gem_vessel_node(key))
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys.iter()
        .map(|key| u32::from(stages.conviction_count(key)))
        .sum()
}

/// The slots a Wheel allocation opens for a Character, read at this use.
#[must_use]
pub fn open_slots_for(
    ruleset: &crate::wheel::WheelRuleset,
    slots: &crate::wheel::WheelSlots,
    vocation: &str,
    level: u32,
    promoted: bool,
) -> usize {
    let allocation = crate::wheel::WheelAllocation {
        wheel_ruleset_revision: Some(ruleset.revision().to_owned()),
        wheel_revision: 0,
        slots: *slots,
        current: true,
    };
    let stages = crate::wheel::WheelStages::derive(ruleset, &allocation, vocation, level, promoted);
    open_slots(unlocked_nodes(ruleset, &stages, vocation))
}

/// Projects the slot vector onto the state's derived `vessel` / `slot` fields.
pub fn project(state: &mut AtelierState, vessels: &GemVessels) {
    for gem in state.gems.values_mut() {
        let id = gem.id.to_be_bytes();
        gem.slot = vessels
            .iter()
            .position(|cell| *cell == Some(id))
            .and_then(|slot| u8::try_from(slot).ok());
        gem.vessel = gem.slot.is_some();
    }
}

/// The slot vector a projected state carries (the inverse of [`project`]).
#[must_use]
pub fn from_state(state: &AtelierState) -> GemVessels {
    let mut vessels = NO_GEM_VESSELS;
    for gem in state.gems.values().filter(|gem| gem.vessel) {
        if let Some(cell) = gem.slot.and_then(|slot| vessels.get_mut(usize::from(slot))) {
            *cell = Some(gem.id.to_be_bytes());
        }
    }
    vessels
}

/// Validates a replacement slot vector: every filled slot holds a distinct current gem of the
/// slot's quality, and every slot whose content changed to a gem is open.
#[must_use]
pub fn valid_change(
    before: &GemVessels,
    after: &GemVessels,
    state: Option<&AtelierState>,
    revision: &str,
    open: usize,
) -> bool {
    let mut seen = Vec::with_capacity(GEM_VESSEL_SLOTS);
    after.iter().enumerate().all(|(slot, cell)| {
        let Some(id) = cell else {
            return true;
        };
        if seen.contains(id) {
            return false;
        }
        seen.push(*id);
        let fits = state
            .and_then(|state| state.gems.get(&u128::from_be_bytes(*id)))
            .is_some_and(|gem| {
                gem.quality == GEM_VESSEL_SLOT_QUALITIES[slot] && gem.ruleset_revision == revision
            });
        fits && (before[slot] == *cell || slot < open)
    })
}

/// Replace Vessel: puts `gem` into `slot` (moving it out of any other slot) or empties the
/// slot. An occupant of `slot` returns to the gem inventory.
#[must_use]
pub fn place(vessels: &GemVessels, slot: usize, gem: Option<[u8; 16]>) -> Option<GemVessels> {
    let mut next = *vessels;
    if let Some(gem) = gem {
        for cell in &mut next {
            if *cell == Some(gem) {
                *cell = None;
            }
        }
    }
    *next.get_mut(slot)? = gem;
    Some(next)
}

/// Converts a set of socketed gems from the legacy domain-indexed vector (green, red,
/// purple, blue) into slots. A gem that is still socketed keeps its slot; a gem the client
/// saw in its four cells ([`legacy_cells`]) and no longer lists leaves its slot; a gem
/// socketed beyond those four cells, which the client never saw, keeps its slot; a newly
/// listed gem takes the first free slot of its quality. `quality_of` returns `None` for an
/// unknown gem, which refuses the whole conversion.
pub fn from_legacy_cells(
    current: &GemVessels,
    cells: &[Option<[u8; 16]>; 4],
    quality_of: impl Fn([u8; 16]) -> Option<GemQuality>,
) -> Option<GemVessels> {
    let listed: Vec<&[u8; 16]> = cells.iter().flatten().collect();
    if listed
        .iter()
        .enumerate()
        .any(|(index, id)| listed[..index].contains(id))
    {
        return None;
    }
    let visible = legacy_cells(current);
    let mut next = NO_GEM_VESSELS;
    for (slot, cell) in current.iter().enumerate() {
        if cell.is_some_and(|id| cells.contains(&Some(id)) || !visible.contains(&Some(id))) {
            next[slot] = *cell;
        }
    }
    for id in cells.iter().flatten() {
        if next.contains(&Some(*id)) {
            continue;
        }
        let quality = quality_of(*id)?;
        let slot = (0..GEM_VESSEL_SLOTS)
            .find(|slot| next[*slot].is_none() && GEM_VESSEL_SLOT_QUALITIES[*slot] == quality)?;
        next[slot] = Some(*id);
    }
    Some(next)
}

/// The legacy four-cell projection for clients of capability 148: the first four socketed
/// gems in slot order. Cell positions no longer carry a domain.
#[must_use]
pub fn legacy_cells(vessels: &GemVessels) -> [Option<[u8; 16]>; 4] {
    let mut cells = [None; 4];
    for (cell, id) in cells.iter_mut().zip(vessels.iter().flatten()) {
        *cell = Some(*id);
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atelier::Gem;

    fn gem(id: u8, quality: GemQuality) -> Gem {
        Gem {
            id: u128::from_be_bytes([id; 16]),
            quality,
            mods: Vec::new(),
            locked: false,
            vessel: false,
            slot: None,
            ruleset_revision: "r".into(),
        }
    }

    fn state(gems: &[(u8, GemQuality)]) -> AtelierState {
        AtelierState {
            gems: gems
                .iter()
                .map(|(id, quality)| {
                    let gem = gem(*id, *quality);
                    (gem.id, gem)
                })
                .collect(),
            ..AtelierState::default()
        }
    }

    #[test]
    fn legacy_domain_vessels_convert_to_first_free_slot_of_each_quality() {
        let atelier = state(&[
            (1, GemQuality::Regular),
            (2, GemQuality::Lesser),
            (3, GemQuality::Regular),
            (4, GemQuality::Greater),
        ]);
        let quality = |id: [u8; 16]| {
            atelier
                .gems
                .get(&u128::from_be_bytes(id))
                .map(|gem| gem.quality)
        };
        let converted = from_legacy_cells(
            &NO_GEM_VESSELS,
            &[Some([1; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])],
            quality,
        );
        let mut expected = NO_GEM_VESSELS;
        expected[0] = Some([2; 16]);
        expected[1] = Some([1; 16]);
        expected[2] = Some([4; 16]);
        expected[4] = Some([3; 16]);
        assert_eq!(converted, Some(expected));
        // Every legacy gem keeps a slot: no socket is lost.
        assert_eq!(converted.map(|v| v.iter().flatten().count()), Some(4));
        // An unknown gem refuses the conversion instead of dropping it silently.
        assert_eq!(
            from_legacy_cells(&NO_GEM_VESSELS, &[Some([9; 16]), None, None, None], quality),
            None
        );
        // A gem listed twice refuses the conversion.
        assert_eq!(
            from_legacy_cells(
                &NO_GEM_VESSELS,
                &[Some([2; 16]), Some([2; 16]), None, None],
                quality
            ),
            None
        );
        // A gem still listed keeps its slot; an unlisted one leaves.
        let kept = from_legacy_cells(&expected, &[None, Some([3; 16]), None, None], quality);
        let mut only = NO_GEM_VESSELS;
        only[4] = Some([3; 16]);
        assert_eq!(kept, Some(only));
        assert_eq!(
            legacy_cells(&expected),
            [Some([2; 16]), Some([1; 16]), Some([4; 16]), Some([3; 16])]
        );
    }

    #[test]
    fn legacy_save_keeps_gems_socketed_beyond_the_four_cells() {
        let atelier = state(&[
            (1, GemQuality::Lesser),
            (2, GemQuality::Regular),
            (3, GemQuality::Greater),
            (4, GemQuality::Lesser),
            (5, GemQuality::Regular),
            (6, GemQuality::Greater),
        ]);
        let quality = |id: [u8; 16]| {
            atelier
                .gems
                .get(&u128::from_be_bytes(id))
                .map(|gem| gem.quality)
        };
        let mut current = NO_GEM_VESSELS;
        for (slot, id) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)] {
            current[slot] = Some([id; 16]);
        }
        let shown = legacy_cells(&current);
        assert_eq!(
            shown,
            [Some([1; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])]
        );
        // The client unsockets gem 1 and sockets gem 6: gem 5, socketed beyond the four
        // cells it was shown, keeps its slot.
        let saved = from_legacy_cells(
            &current,
            &[Some([6; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])],
            quality,
        );
        let mut expected = current;
        expected[0] = None;
        expected[5] = Some([6; 16]);
        assert_eq!(saved, Some(expected));
        // Saving exactly what was shown changes nothing.
        assert_eq!(from_legacy_cells(&current, &shown, quality), Some(current));
    }

    #[test]
    fn gem_fits_any_open_slot_of_its_quality() {
        let atelier = state(&[(1, GemQuality::Lesser), (2, GemQuality::Greater)]);
        for slot in 0..GEM_VESSEL_SLOTS {
            let after = place(&NO_GEM_VESSELS, slot, Some([1; 16])).unwrap_or(NO_GEM_VESSELS);
            let lesser = GEM_VESSEL_SLOT_QUALITIES[slot] == GemQuality::Lesser;
            assert_eq!(
                valid_change(
                    &NO_GEM_VESSELS,
                    &after,
                    Some(&atelier),
                    "r",
                    GEM_VESSEL_SLOTS
                ),
                lesser,
                "slot {slot}"
            );
        }
        let greater = place(&NO_GEM_VESSELS, 2, Some([2; 16])).unwrap_or(NO_GEM_VESSELS);
        // Slot 2 opens with the third unlocked node.
        assert!(!valid_change(
            &NO_GEM_VESSELS,
            &greater,
            Some(&atelier),
            "r",
            2
        ));
        assert!(valid_change(
            &NO_GEM_VESSELS,
            &greater,
            Some(&atelier),
            "r",
            3
        ));
        // A slot that closed keeps its gem until it is changed.
        assert!(valid_change(&greater, &greater, Some(&atelier), "r", 0));
        // Stale ruleset, unknown gem and duplicate placement are refused.
        assert!(!valid_change(
            &NO_GEM_VESSELS,
            &greater,
            Some(&atelier),
            "s",
            3
        ));
        assert!(!valid_change(&NO_GEM_VESSELS, &greater, None, "r", 3));
        let mut twice = NO_GEM_VESSELS;
        twice[0] = Some([1; 16]);
        twice[3] = Some([1; 16]);
        assert!(!valid_change(
            &NO_GEM_VESSELS,
            &twice,
            Some(&atelier),
            "r",
            12
        ));
    }

    #[test]
    fn every_vocation_has_twelve_gem_vessel_nodes() -> Result<(), Box<dyn std::error::Error>> {
        let ruleset = crate::wheel_gem_data::WheelGemData::embedded()?.wheel_ruleset()?;
        for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
            let nodes = node_slots(&ruleset, vocation);
            assert_eq!(nodes.len(), GEM_VESSEL_SLOTS, "{vocation}");
            assert!(nodes.windows(2).all(|pair| pair[0] < pair[1]));
        }
        assert_eq!(
            node_slots(&ruleset, "knight"),
            [3, 5, 7, 10, 15, 18, 19, 22, 27, 30, 32, 34]
        );
        Ok(())
    }

    #[test]
    fn replace_vessel_moves_a_gem_and_returns_the_occupant() {
        let mut vessels = NO_GEM_VESSELS;
        vessels[0] = Some([1; 16]);
        vessels[3] = Some([2; 16]);
        let moved = place(&vessels, 3, Some([1; 16]));
        let mut expected = NO_GEM_VESSELS;
        expected[3] = Some([1; 16]);
        assert_eq!(moved, Some(expected));
        assert_eq!(place(&vessels, 0, None).map(|v| v[0]), Some(None));
        assert_eq!(place(&vessels, GEM_VESSEL_SLOTS, None), None);
        let mut atelier = state(&[(1, GemQuality::Lesser), (2, GemQuality::Lesser)]);
        project(&mut atelier, &expected);
        let one = atelier.gems.get(&u128::from_be_bytes([1; 16]));
        assert_eq!(one.map(|gem| (gem.vessel, gem.slot)), Some((true, Some(3))));
        let two = atelier.gems.get(&u128::from_be_bytes([2; 16]));
        assert_eq!(two.map(|gem| (gem.vessel, gem.slot)), Some((false, None)));
        assert_eq!(from_state(&atelier), expected);
    }
}
