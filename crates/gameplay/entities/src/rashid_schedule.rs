//! Server-save weekday placement and availability for the travelling trader Rashid.

use jiff::{Timestamp, tz::TimeZone};
use std::sync::LazyLock;

use crate::{MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable};

/// The catalogue identity whose placement rotates each weekday.
pub const RASHID_NPC_KEY: &str = "oteryn:npc.rashid";

/// Rashid's destination for a Tibia weekday.
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

/// Rashid switches at 10:00 CET/CEST, the start of the next Tibia day.
pub fn destination_at(unix_ms: i64) -> RashidDestination {
    // The Fandom Rashid schedule uses CET with DST from the last Sunday in March
    // to the last Sunday in October. An explicit zone keeps host settings out of it.
    static TIBIA_ZONE: LazyLock<TimeZone> =
        LazyLock::new(|| TimeZone::posix("CET-1CEST,M3.5.0,M10.5.0/3").unwrap_or(TimeZone::UTC));
    let utc = Timestamp::from_millisecond(unix_ms).unwrap_or(if unix_ms < 0 {
        Timestamp::MIN
    } else {
        Timestamp::MAX
    });
    let local = utc.to_zoned(TIBIA_ZONE.clone());
    let weekday = local.weekday().to_monday_zero_offset() - i8::from(local.hour() < 10);
    match weekday.rem_euclid(7) {
        0 => destination("Monday", 32_207, 31_155, 7),
        1 => destination("Tuesday", 32_300, 32_837, 7),
        2 => destination("Wednesday", 32_577, 32_753, 7),
        3 => destination("Thursday", 33_066, 32_879, 6),
        4 => destination("Friday", 33_235, 32_483, 7),
        5 => destination("Saturday", 33_166, 31_810, 6),
        _ => destination("Sunday", 32_328, 31_782, 6),
    }
}

/// Retain only today's placement for Rashid. Other NPCs and their ordering are unchanged.
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

/// Whether a visible Rashid actor is within his destination's wander area this Tibia weekday.
/// Canary 47dfd51f and Crystal 201189f8 npc/rashid.lua:10-11 set a 2000 ms
/// interval and radius 3. Canary src/creatures/npcs/npc.cpp:1190-1196 bounds
/// walking around masterPos; trading must remain available after those steps.
pub fn trade_available(npc_key: &str, position: MovementLocalPosition, unix_ms: i64) -> bool {
    if npc_key != RASHID_NPC_KEY {
        return true;
    }
    let expected = destination_at(unix_ms).placement;
    position.x.abs_diff(i32::from(expected.x)) <= 3
        && position.y.abs_diff(i32::from(expected.y)) <= 3
        && position.floor == -i16::from(expected.floor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NpcPlacements;

    #[test]
    fn canary_week_rotates_all_seven_destinations() {
        let monday = 4 * 86_400_000 + 9 * 3_600_000;
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

    #[test]
    fn rashid_placement_and_trade_switch_at_server_save_in_winter_and_summer() {
        let friday = destination("Friday", 33_235, 32_483, 7);
        let saturday = destination("Saturday", 33_166, 31_810, 6);
        let position = |day: RashidDestination| MovementLocalPosition {
            x: i32::from(day.placement.x),
            y: i32::from(day.placement.y),
            floor: -i16::from(day.placement.floor),
        };
        const WINTER_SAVE: i64 = 1_795_856_400_000; // Saturday 2026-11-28, 09:00 UTC
        const SUMMER_SAVE: i64 = 1_785_571_200_000; // Saturday 2026-08-01, 08:00 UTC
        for save in [WINTER_SAVE, SUMMER_SAVE] {
            for (now, active, inactive) in [(save - 1, friday, saturday), (save, saturday, friday)]
            {
                let mut table = NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: RASHID_NPC_KEY.into(),
                        placements: vec![friday.placement, saturday.placement],
                    }],
                };
                rotate_placements(&mut table, now);
                assert_eq!(
                    table.npcs[0].placements,
                    [active.placement],
                    "placement at {now}"
                );
                assert!(
                    trade_available(RASHID_NPC_KEY, position(active), now),
                    "trade at {now}"
                );
                assert!(!trade_available(RASHID_NPC_KEY, position(inactive), now));
            }
        }
        assert!(!trade_available(
            RASHID_NPC_KEY,
            MovementLocalPosition {
                floor: 6,
                ..position(friday)
            },
            WINTER_SAVE - 1
        ));
        assert!(trade_available("oteryn:npc.alice", position(friday), 0));
    }

    // Both pinned donors configure Rashid with walkRadius = 3 (npc/rashid.lua:11).
    #[test]
    fn weekday_trade_remains_available_after_wandering() {
        for day in 0..7 {
            let now = day * 86_400_000;
            let spawn = destination_at(now).placement;
            for dx in -3..=3 {
                for dy in -3..=3 {
                    assert!(trade_available(
                        RASHID_NPC_KEY,
                        MovementLocalPosition {
                            x: i32::from(spawn.x) + dx,
                            y: i32::from(spawn.y) + dy,
                            floor: -i16::from(spawn.floor),
                        },
                        now,
                    ));
                }
            }
            assert!(!trade_available(
                RASHID_NPC_KEY,
                MovementLocalPosition {
                    x: i32::from(spawn.x) + 4,
                    y: i32::from(spawn.y),
                    floor: -i16::from(spawn.floor),
                },
                now,
            ));
        }
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
