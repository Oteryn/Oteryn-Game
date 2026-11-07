//! Canary-compatible weekday placement and availability for the travelling trader Rashid.

use crate::{MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable};

/// The catalogue identity whose placement rotates each weekday.
pub const RASHID_NPC_KEY: &str = "oteryn:npc.rashid";

/// Rashid's Canary weekday destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RashidDestination {
    pub city: &'static str,
    pub placement: NpcPlacement,
}

const fn destination(city: &'static str, x: u16, y: u16, z: i8) -> RashidDestination {
    RashidDestination {
        city,
        placement: NpcPlacement {
            floor: -z,
            x,
            y,
            direction: MovementFacing::South,
        },
    }
}

/// Destination at a UTC Unix timestamp. Canary uses the host calendar day; Oteryn deliberately
/// fixes that otherwise deployment-dependent clock to UTC.
pub fn destination_at(unix_ms: i64) -> RashidDestination {
    // 1970-01-01 was Thursday. `rem_euclid` also gives a defined answer before the epoch.
    match (unix_ms.div_euclid(86_400_000) + 3).rem_euclid(7) {
        0 => destination("Monday", 32_207, 31_155, 7),
        1 => destination("Tuesday", 32_300, 32_837, 7),
        2 => destination("Wednesday", 32_577, 32_753, 7),
        3 => destination("Thursday", 33_066, 32_879, 6),
        4 => destination("Friday", 33_235, 32_483, 7),
        5 => destination("Saturday", 33_166, 31_810, 6),
        _ => destination("Sunday", 32_328, 31_782, 6),
    }
}

/// Retain only today's Canary placement for Rashid. Other NPCs and their ordering are unchanged.
pub fn rotate_placements(table: &mut NpcPlacementTable, unix_ms: i64) {
    let expected = destination_at(unix_ms).placement;
    if let Some(rashid) = table.npcs.iter_mut().find(|npc| npc.key == RASHID_NPC_KEY) {
        rashid.placements.retain(|placement| {
            (placement.floor, placement.x, placement.y) == (expected.floor, expected.x, expected.y)
        });
    }
    // An absent destination means Rashid is unavailable, not an invalid empty placement record.
    table
        .npcs
        .retain(|npc| npc.key != RASHID_NPC_KEY || !npc.placements.is_empty());
}

/// Whether a visible Rashid actor is at his destination for this UTC weekday.
pub fn trade_available(npc_key: &str, position: MovementLocalPosition, unix_ms: i64) -> bool {
    if npc_key != RASHID_NPC_KEY {
        return true;
    }
    let expected = destination_at(unix_ms).placement;
    position.x == i32::from(expected.x)
        && position.y == i32::from(expected.y)
        && position.floor == -i16::from(expected.floor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NpcPlacements;

    #[test]
    fn canary_week_rotates_all_seven_destinations() {
        let monday = 4 * 86_400_000;
        let expected = [
            ("Monday", 32_207, 31_155, -7),
            ("Tuesday", 32_300, 32_837, -7),
            ("Wednesday", 32_577, 32_753, -7),
            ("Thursday", 33_066, 32_879, -6),
            ("Friday", 33_235, 32_483, -7),
            ("Saturday", 33_166, 31_810, -6),
            ("Sunday", 32_328, 31_782, -6),
        ];
        for (day, (city, x, y, floor)) in (0_i64..).zip(expected) {
            let actual = destination_at(monday + day * 86_400_000);
            assert_eq!(
                (
                    actual.city,
                    actual.placement.x,
                    actual.placement.y,
                    actual.placement.floor
                ),
                (city, x, y, floor)
            );
        }
    }

    // Crystal 201189f8 data-global/scripts/globalevents/spawn/rashid.lua:2-8 (Canary 47dfd51f
    // identical) teleports Rashid to the weekday destination, so trading follows his position.
    // Crystal rotates at host-local 00:01; Oteryn rotates at UTC 00:00.
    #[test]
    fn crystal_trade_follows_the_weekday_destination() {
        let thursday = 0;
        let position = |x: i32, y: i32, floor: i16| MovementLocalPosition { x, y, floor };
        let ankrahmun = position(33_066, 32_879, 6);
        assert!(trade_available(RASHID_NPC_KEY, ankrahmun, thursday));
        assert!(trade_available(RASHID_NPC_KEY, ankrahmun, 86_400_000 - 1));
        assert!(!trade_available(RASHID_NPC_KEY, ankrahmun, 86_400_000));
        assert!(trade_available(
            RASHID_NPC_KEY,
            position(33_235, 32_483, 7),
            86_400_000
        ));
        assert!(!trade_available(
            RASHID_NPC_KEY,
            position(33_066, 32_879, 7),
            thursday
        ));
        assert!(trade_available(
            "oteryn:npc.alice",
            position(0, 0, 0),
            thursday
        ));
    }

    #[test]
    fn rotation_selects_today_and_leaves_other_npcs_untouched() {
        let all = (0..7)
            .map(|day| destination_at((4 + day) * 86_400_000).placement)
            .collect();
        let alice = NpcPlacements {
            key: "oteryn:npc.alice".into(),
            placements: vec![destination("fixture", 1, 2, 7).placement],
        };
        let mut table = NpcPlacementTable {
            npcs: vec![
                alice.clone(),
                NpcPlacements {
                    key: RASHID_NPC_KEY.into(),
                    placements: all,
                },
            ],
        };
        rotate_placements(&mut table, 4 * 86_400_000);
        assert_eq!(table.npcs[0], alice);
        assert_eq!(
            table.npcs[1].placements,
            vec![destination_at(4 * 86_400_000).placement]
        );
    }
}
