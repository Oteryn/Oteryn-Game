//! Source Item::hasProperty/canBeMoved, Canary 47dfd51f:
//! src/items/item.cpp:1600-1641; src/items/items.cpp:236-247;
//! src/utils/const.hpp:73. Tile aggregation: src/items/tile.cpp:1844-1889.
//! OTS_HYPOTHESIS_ONLY; the catalogue contains loader facts, not movement authority.

use oteryn_world_bundle::sector::Attrs;

mod catalogue {
    include!("immovable_catalogue.rs");
}

/// Unknown source identities remain blocked. A placement's unique id or action 100
/// overrides source movability, just as Item::canBeMoved does in the donor.
pub(super) fn flags(source_id: u32, attrs: &Attrs) -> [bool; 3] {
    let Some(mask) = usize::try_from(source_id)
        .ok()
        .and_then(|id| catalogue::FLAGS.as_bytes().get(id))
        .filter(|value| (b'A'..=b'P').contains(value))
        .map(|value| value - b'A')
    else {
        return [true; 3];
    };
    let movable = mask & 8 != 0 && attrs.unique.is_none() && attrs.action != Some(100);
    [
        mask & 1 != 0 && !movable,
        mask & 2 != 0 && !movable,
        mask & 2 != 0 && mask & 4 == 0 && !movable,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_solid_path_and_magic_field_facts_remain_distinct() {
        assert_eq!(flags(103, &Attrs::default()), [false; 3]);
        assert_eq!(flags(100, &Attrs::default()), [true, false, false]);
        assert_eq!(flags(1950, &Attrs::default()), [false, true, true]);
        assert_eq!(flags(2124, &Attrs::default()), [false, true, false]);
        assert_eq!(flags(2128, &Attrs::default()), [true, true, false]);
    }

    #[test]
    fn placement_unique_and_immovable_action_override_source_movability() {
        assert_eq!(flags(2025, &Attrs::default()), [false; 3]);
        for attrs in [
            Attrs {
                unique: Some(7),
                ..Attrs::default()
            },
            Attrs {
                action: Some(100),
                ..Attrs::default()
            },
        ] {
            assert_eq!(flags(2025, &attrs), [true, false, false]);
        }
        assert_eq!(
            flags(
                2025,
                &Attrs {
                    action: Some(101),
                    ..Attrs::default()
                }
            ),
            [false; 3]
        );
    }

    #[test]
    fn unknown_source_facts_never_open_a_destination() {
        for id in [0, 65535, u32::MAX] {
            assert_eq!(flags(id, &Attrs::default()), [true; 3]);
        }
    }
}
