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

const MILLIS_PER_DAY: i64 = 86_400_000;
const MILLIS_PER_HOUR: i64 = 3_600_000;

// Gregorian calendar conversions adapted from the public-domain civil-calendar algorithms by
// Howard Hinnant. Keeping this tiny conversion local avoids making timezone behavior depend on
// the host configuration or adding a dependency.
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

const fn year_from_days(days: i64) -> i64 {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    year
}

const fn last_sunday(year: i64, month: i64) -> i64 {
    let last = days_from_civil(year, month, 31);
    // 1970-01-01 was Thursday (Sunday = 0).
    last - (last + 4).rem_euclid(7)
}

fn berlin_utc_offset_ms(unix_ms: i64) -> i64 {
    let year = year_from_days(unix_ms.div_euclid(MILLIS_PER_DAY));
    // Europe/Berlin changes at 01:00 UTC on the last Sundays in March and October.
    let summer_start = last_sunday(year, 3) * MILLIS_PER_DAY + MILLIS_PER_HOUR;
    let summer_end = last_sunday(year, 10) * MILLIS_PER_DAY + MILLIS_PER_HOUR;
    if (summer_start..summer_end).contains(&unix_ms) {
        2 * MILLIS_PER_HOUR
    } else {
        MILLIS_PER_HOUR
    }
}

/// Destination at a UTC Unix timestamp. Tibia's weekday changes at the Europe/Berlin 10:00
/// server-save boundary, including the region's daylight-saving transitions.
pub fn destination_at(unix_ms: i64) -> RashidDestination {
    let server_day =
        (unix_ms + berlin_utc_offset_ms(unix_ms) - 10 * MILLIS_PER_HOUR).div_euclid(MILLIS_PER_DAY);
    // 1970-01-01 was Thursday. Subtracting the server-save boundary makes the interval before
    // that day's save retain the previous weekday destination.
    match (server_day + 3).rem_euclid(7) {
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

/// Whether a visible Rashid actor is at his destination for this server-save weekday.
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
        // 1970-01-05 09:00 UTC = Monday's 10:00 server save in Berlin.
        let monday = 4 * MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR;
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
            let actual = destination_at(monday + day * MILLIS_PER_DAY);
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
    // Global rotates at Europe/Berlin's 10:00 server save.
    #[test]
    fn crystal_trade_follows_the_weekday_destination() {
        let thursday = 9 * MILLIS_PER_HOUR;
        let position = |x: i32, y: i32, floor: i16| MovementLocalPosition { x, y, floor };
        let ankrahmun = position(33_066, 32_879, 6);
        assert!(trade_available(RASHID_NPC_KEY, ankrahmun, thursday));
        assert!(trade_available(
            RASHID_NPC_KEY,
            ankrahmun,
            MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR - 1
        ));
        assert!(!trade_available(
            RASHID_NPC_KEY,
            ankrahmun,
            MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR
        ));
        assert!(trade_available(
            RASHID_NPC_KEY,
            position(33_235, 32_483, 7),
            MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR
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
    fn global_server_save_boundary_tracks_standard_and_summer_time() {
        let cases = [
            // Winter Monday: 10:00 CET is 09:00 UTC.
            (1_704_704_399_000, "Sunday"),
            (1_704_704_400_000, "Monday"),
            // DST-change Sunday: by server save, 10:00 CEST is 08:00 UTC.
            (1_711_871_999_000, "Saturday"),
            (1_711_872_000_000, "Sunday"),
            // The following summer Monday retains the 08:00 UTC boundary.
            (1_711_958_399_000, "Sunday"),
            (1_711_958_400_000, "Monday"),
            // Standard time has resumed before October's Sunday server save.
            (1_730_019_599_000, "Saturday"),
            (1_730_019_600_000, "Sunday"),
            (1_730_105_999_000, "Sunday"),
            (1_730_106_000_000, "Monday"),
        ];
        for (unix_ms, expected) in cases {
            assert_eq!(destination_at(unix_ms).city, expected, "at {unix_ms}");
        }
    }

    #[test]
    fn rotation_selects_today_and_leaves_other_npcs_untouched() {
        let all = (0..7)
            .map(|day| destination_at((4 + day) * MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR).placement)
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
        rotate_placements(&mut table, 4 * MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR);
        assert_eq!(table.npcs[0], alice);
        assert_eq!(
            table.npcs[1].placements,
            vec![destination_at(4 * MILLIS_PER_DAY + 9 * MILLIS_PER_HOUR).placement]
        );
    }
}
