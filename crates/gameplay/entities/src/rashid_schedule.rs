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

/// The authored placements in Monday-through-Sunday order.
pub const RASHID_DESTINATIONS: [RashidDestination; 7] = [
    destination("Monday", 32_207, 31_155, 7),
    destination("Tuesday", 32_300, 32_837, 7),
    destination("Wednesday", 32_577, 32_753, 7),
    destination("Thursday", 33_066, 32_879, 6),
    destination("Friday", 33_235, 32_483, 7),
    destination("Saturday", 33_166, 31_810, 6),
    destination("Sunday", 32_328, 31_782, 6),
];

/// Global uses the weekday of the most recent 10:00 Berlin Server Save.
/// Embed the named timezone so placement and trade reads need no runtime I/O.
pub fn server_save_weekday_index(unix_ms: i64) -> usize {
    static BERLIN: jiff::tz::TimeZone = jiff::tz::get!("Europe/Berlin");
    let timestamp = jiff::Timestamp::from_millisecond(unix_ms).unwrap_or(if unix_ms < 0 {
        jiff::Timestamp::MIN
    } else {
        jiff::Timestamp::MAX
    });
    let local = timestamp.to_zoned(BERLIN.clone());
    let weekday = if local.hour() < 10 {
        local.weekday().previous()
    } else {
        local.weekday()
    };
    weekday.to_monday_zero_offset() as usize
}

pub fn destination_at(unix_ms: i64) -> RashidDestination {
    RASHID_DESTINATIONS[server_save_weekday_index(unix_ms)]
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

/// Whether a visible Rashid actor is within his destination's wander area on the current Server Save day.
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

    // 1970-01-01 was winter time in Berlin.
    const THURSDAY_SAVE: i64 = 9 * 3_600_000;

    #[test]
    fn canary_week_rotates_all_seven_destinations() {
        let monday = THURSDAY_SAVE + 4 * 86_400_000;
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
    #[test]
    fn crystal_trade_follows_the_weekday_destination() {
        let thursday = THURSDAY_SAVE;
        let position = |x: i32, y: i32, floor: i16| MovementLocalPosition { x, y, floor };
        let ankrahmun = position(33_066, 32_879, 6);
        assert!(trade_available(RASHID_NPC_KEY, ankrahmun, thursday));
        assert!(trade_available(
            RASHID_NPC_KEY,
            ankrahmun,
            thursday + 86_400_000 - 1
        ));
        assert!(!trade_available(
            RASHID_NPC_KEY,
            ankrahmun,
            thursday + 86_400_000
        ));
        assert!(trade_available(
            RASHID_NPC_KEY,
            position(33_235, 32_483, 7),
            thursday + 86_400_000
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

    // Both pinned donors configure Rashid with walkRadius = 3 (npc/rashid.lua:11).
    #[test]
    fn weekday_trade_remains_available_after_wandering() {
        for day in 0..7 {
            let now = THURSDAY_SAVE + day * 86_400_000;
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
    fn rashid_rotates_at_ten_in_winter_summer_and_both_dst_weeks() -> Result<(), jiff::Error> {
        for (boundary, before, after) in [
            ("2026-01-05T09:00:00Z", "Sunday", "Monday"),
            ("2026-07-06T08:00:00Z", "Sunday", "Monday"),
            ("2026-03-29T08:00:00Z", "Saturday", "Sunday"),
            ("2026-03-30T08:00:00Z", "Sunday", "Monday"),
            ("2026-10-25T09:00:00Z", "Saturday", "Sunday"),
            ("2026-10-26T09:00:00Z", "Sunday", "Monday"),
        ] {
            let instant = boundary.parse::<jiff::Timestamp>()?.as_millisecond();
            assert_eq!(destination_at(instant - 60_000).city, before, "{boundary}");
            assert_eq!(destination_at(instant).city, after, "{boundary}");
        }
        Ok(())
    }

    #[test]
    fn rotation_selects_today_and_leaves_other_npcs_untouched() {
        let all = (0..7)
            .map(|day| destination_at(THURSDAY_SAVE + (4 + day) * 86_400_000).placement)
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
        rotate_placements(&mut table, THURSDAY_SAVE + 4 * 86_400_000);
        assert_eq!(table.npcs[0], alice);
        assert_eq!(
            table.npcs[1].placements,
            vec![destination_at(THURSDAY_SAVE + 4 * 86_400_000).placement]
        );
    }
}
