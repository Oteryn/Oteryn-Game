//! PKG-P4/P02: category rules from the single Bosstiary content table.
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BosstiaryBoss {
    pub key: String,
    pub category: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BosstiaryProgress {
    /// 0 = below prowess, 1 = prowess, 2 = expertise, 3 = mastery.
    pub stage: u8,
    pub points: u16,
}

#[derive(Deserialize)]
struct Category {
    category: String,
    kills: [u64; 3],
    points: [u16; 3],
}

fn category(category: &str) -> Option<Category> {
    let rows: Vec<Category> = serde_json::from_str(include_str!(
        "../../../../../rulesets/progression/bosstiary/categories.json"
    ))
    .ok()?;
    rows.into_iter().find(|row| row.category == category)
}

/// The category's prowess, expertise and mastery kill thresholds.
pub fn kill_thresholds(category: &str) -> Option<[u64; 3]> {
    Some(self::category(category)?.kills)
}

pub fn progress(category: &str, kills: u64) -> Option<BosstiaryProgress> {
    let row = self::category(category)?;
    let mut result = BosstiaryProgress {
        stage: 0,
        points: 0,
    };
    for (threshold, points) in row.kills.into_iter().zip(row.points) {
        if kills >= threshold {
            result.stage += 1;
            result.points += points;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_threshold_and_cumulative_points() {
        for (category, thresholds, awards) in [
            ("bane", [25, 100, 300], [5, 15, 30]),
            ("archfoe", [5, 20, 60], [10, 30, 60]),
            ("nemesis", [1, 3, 5], [10, 30, 60]),
        ] {
            assert_eq!(kill_thresholds(category), Some(thresholds));
            let mut points = 0;
            for (index, threshold) in thresholds.into_iter().enumerate() {
                assert_eq!(
                    progress(category, threshold - 1),
                    Some(BosstiaryProgress {
                        stage: index as u8,
                        points
                    })
                );
                points += awards[index];
                for count in [threshold, threshold + 1] {
                    assert_eq!(
                        progress(category, count),
                        Some(BosstiaryProgress {
                            stage: index as u8 + 1,
                            points
                        })
                    );
                }
            }
        }
        assert_eq!(progress("unknown", 10), None);
        assert_eq!(kill_thresholds("unknown"), None);
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    #[test]
    fn crystal_boss_slot_unlock_bonus_remove_and_boosted_credit() {
        assert!(!slot_unlocked(1, 0, false));
        assert!(slot_unlocked(1, 5, true));
        assert!(!slot_unlocked(2, 1499, true));
        assert!(slot_unlocked(2, 1500, true));
        for (points, bonus) in [(0, 25), (250, 50), (1250, 100), (1500, 106)] {
            assert_eq!(loot_bonus_percent(points, false), bonus);
            assert_eq!(loot_bonus_percent(points, true), bonus + 25);
        }
        assert_eq!(
            [remove_fee(1), remove_fee(2), remove_fee(3)],
            [0, 100000, 400000]
        );
        assert_eq!(kill_credit(false), 1);
        assert_eq!(kill_credit(true), 3);
    }
}

/// Crystal protocolgame.cpp:10615-10632: first prowess unlocks slot one; 1500 points slot two.
pub const fn slot_unlocked(slot: u8, points: u32, prowess: bool) -> bool {
    match slot {
        1 => prowess,
        2 => points >= 1500,
        _ => false,
    }
}
/// Crystal io_bosstiary.cpp:225-232 and protocolgame.cpp:10621: mastery adds 25%.
pub fn loot_bonus_percent(points: u32, mastered: bool) -> u16 {
    let base = if points <= 250 {
        25 + points / 10
    } else if points < 1250 {
        37 + points / 20
    } else {
        100 + (((8 * u64::from((points - 1250) / 5) + 81) as f64).sqrt() - 9.0) as u32 / 2
    };
    base.min(u32::from(u16::MAX - 25)) as u16 + if mastered { 25 } else { 0 }
}
/// Crystal io_bosstiary.cpp:311-315, player.hpp:1881: initial removal counter is one.
pub const fn remove_fee(removals: u8) -> u64 {
    if removals < 2 {
        0
    } else {
        300000 * removals as u64 - 500000
    }
}
/// Crystal player.cpp:6567-6578, config.lua.dist:240-244: base one, boosted multiplier three.
pub const fn kill_credit(boosted: bool) -> u8 {
    if boosted { 3 } else { 1 }
}
