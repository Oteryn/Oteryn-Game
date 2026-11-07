//! Browse-field rules from Crystal `201189f`, independent of custody and session authority.

use oteryn_protocol_oteryn::browse_field::BROWSE_FIELD_CAPACITY;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

/// Facts read from the current item definition and its actual tile placement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct ItemFacts {
    pub unique: bool,
    pub movable: bool,
    pub container: bool,
    pub wrappable: bool,
    pub blocks_path: bool,
}

/// Crystal `container.cpp:68-78`: immovable containers and nonblocking wrappable furniture
/// remain visible; a unique placement is never included, even when it is a container.
pub(in crate::gameplay_transport) const fn eligible(facts: ItemFacts) -> bool {
    !facts.unique && (facts.container || facts.movable || (facts.wrappable && !facts.blocks_path))
}

/// Crystal `container.cpp:63-78` pushes eligible items to the front of the browse list.
/// The input is the tile owner's item order, before any map-view truncation.
pub(in crate::gameplay_transport) fn display_order<T>(
    items: &[T],
    facts: impl Fn(&T) -> ItemFacts,
) -> impl Iterator<Item = &T> {
    items.iter().rev().filter(move |item| eligible(facts(item)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) enum Reach {
    Adjacent,
    Walk,
    WrongFloor,
}

/// Crystal `game.cpp:5027-5047`: floor refusal precedes same-floor adjacent reach/walk.
pub(in crate::gameplay_transport) const fn reach(
    actor: WorldTilePosition,
    target: WorldTilePosition,
) -> Reach {
    if actor.floor != target.floor {
        Reach::WrongFloor
    } else if actor.x.abs_diff(target.x) <= 1 && actor.y.abs_diff(target.y) <= 1 {
        Reach::Adjacent
    } else {
        Reach::Walk
    }
}

/// Crystal `game.cpp:5074`: adjacent tiles use distinct slots in the upper nine view ids.
pub(in crate::gameplay_transport) const fn dummy_view_id(x: u16, y: u16) -> u8 {
    15 - ((x % 3) * 3 + y % 3) as u8
}

/// Crystal `game.cpp:5192`: opening an empty field is allowed, seeking it is not.
pub(in crate::gameplay_transport) const fn valid_seek(index: u16, total: u16) -> bool {
    index.is_multiple_of(BROWSE_FIELD_CAPACITY as u16) && index < total
}

/// Keep a viewed page while it exists; move back when removals empty the current page.
/// Crystal `player.cpp:9077-9080` receives the size before `container.cpp:838` erases the item.
/// Here `total` is the authoritative size after the removal, including coalesced changes.
pub(in crate::gameplay_transport) const fn refresh_index(index: u16, total: u16) -> u16 {
    let capacity = BROWSE_FIELD_CAPACITY as u16;
    let last = total.saturating_sub(1) / capacity * capacity;
    let current = index / capacity * capacity;
    if current > last { last } else { current }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32, y: i32, floor: i16) -> WorldTilePosition {
        WorldTilePosition { x, y, floor }
    }

    #[test]
    fn browse_filter_preserves_each_donor_eligibility_branch() {
        assert!(!eligible(ItemFacts::default()));
        assert!(eligible(ItemFacts {
            movable: true,
            ..ItemFacts::default()
        }));
        assert!(eligible(ItemFacts {
            container: true,
            ..ItemFacts::default()
        }));
        assert!(eligible(ItemFacts {
            wrappable: true,
            ..ItemFacts::default()
        }));
        assert!(!eligible(ItemFacts {
            wrappable: true,
            blocks_path: true,
            ..ItemFacts::default()
        }));
        // Blocking only excludes the furniture branch, not movable items or containers.
        assert!(eligible(ItemFacts {
            movable: true,
            wrappable: true,
            blocks_path: true,
            ..ItemFacts::default()
        }));
        assert!(eligible(ItemFacts {
            container: true,
            blocks_path: true,
            ..ItemFacts::default()
        }));
    }

    #[test]
    fn browse_unique_placement_excludes_every_eligible_item_kind() {
        for facts in [
            ItemFacts {
                movable: true,
                ..ItemFacts::default()
            },
            ItemFacts {
                container: true,
                ..ItemFacts::default()
            },
            ItemFacts {
                wrappable: true,
                ..ItemFacts::default()
            },
        ] {
            assert!(!eligible(ItemFacts {
                unique: true,
                ..facts
            }));
        }
    }

    #[test]
    fn browse_items_reverse_full_tile_order_after_filtering() {
        let items = [
            (
                1,
                ItemFacts {
                    movable: true,
                    ..ItemFacts::default()
                },
            ),
            (2, ItemFacts::default()),
            (
                3,
                ItemFacts {
                    container: true,
                    ..ItemFacts::default()
                },
            ),
            (
                4,
                ItemFacts {
                    unique: true,
                    movable: true,
                    ..ItemFacts::default()
                },
            ),
            (
                5,
                ItemFacts {
                    wrappable: true,
                    ..ItemFacts::default()
                },
            ),
        ];
        assert_eq!(
            display_order(&items, |item| item.1)
                .map(|item| item.0)
                .collect::<Vec<_>>(),
            [5, 3, 1]
        );
    }

    #[test]
    fn browse_reach_includes_diagonals_and_checks_floor_before_walking() {
        let actor = at(100, 100, -7);
        for x in 99..=101 {
            for y in 99..=101 {
                assert_eq!(reach(actor, at(x, y, -7)), Reach::Adjacent);
            }
        }
        for target in [at(98, 100, -7), at(100, 102, -7), at(102, 102, -7)] {
            assert_eq!(reach(actor, target), Reach::Walk);
        }
        assert_eq!(reach(actor, at(100, 100, -6)), Reach::WrongFloor);
        assert_eq!(reach(actor, at(500, 500, -8)), Reach::WrongFloor);
        assert_eq!(reach(at(i32::MIN, 0, 0), at(i32::MAX, 0, 0)), Reach::Walk);
    }

    #[test]
    fn browse_dummy_slots_cover_the_adjacent_nine_and_repeat_after_three() {
        let mut seen = std::collections::BTreeSet::new();
        for x in 0..3 {
            for y in 0..3 {
                assert!(seen.insert(dummy_view_id(x, y)));
                assert_eq!(dummy_view_id(x, y), dummy_view_id(x + 3, y + 3));
            }
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            (7..=15).collect::<Vec<_>>()
        );
        assert_eq!(dummy_view_id(0, 0), 15);
        assert_eq!(dummy_view_id(u16::MAX, u16::MAX), 15);
    }

    #[test]
    fn browse_seek_requires_a_full_page_boundary_inside_the_field() {
        assert!(!valid_seek(0, 0));
        assert!(valid_seek(0, 1));
        assert!(!valid_seek(1, 31));
        assert!(!valid_seek(29, 31));
        assert!(!valid_seek(30, 30));
        assert!(valid_seek(30, 31));
        assert!(valid_seek(60, 61));
        assert!(!valid_seek(60, 60));
        assert!(valid_seek(65_520, u16::MAX));
        assert!(!valid_seek(u16::MAX, u16::MAX));
    }

    #[test]
    fn browse_refresh_retains_valid_page_and_rewinds_removed_pages() {
        assert_eq!(refresh_index(0, 0), 0);
        assert_eq!(refresh_index(30, 31), 30);
        assert_eq!(refresh_index(30, 30), 0);
        assert_eq!(refresh_index(60, 60), 30);
        assert_eq!(refresh_index(90, 31), 30);
        assert_eq!(refresh_index(90, 0), 0);
        assert_eq!(refresh_index(65_520, u16::MAX), 65_520);
    }
}
