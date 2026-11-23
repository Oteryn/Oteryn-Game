//! Global loyalty skill-point projection. Loyalty never mutates trained progress.
use oteryn_gameplay_domain::domain::character_build::{
    BuildFormula, MAX_BUILD_LEVEL, MIN_SKILL_LEVEL,
};

/// Five percentage points per 360 loyalty points, capped at fifty percent.
#[must_use]
pub const fn bonus_percent(points: u32) -> u8 {
    let earned = (points / 360).saturating_mul(5);
    if earned > 50 { 50 } else { earned as u8 }
}

/// Highest account loyalty title; absent below 50 points.
///
/// Crystal `201189f`, `data/libs/functions/player.lua:819-844,857-865`.
#[must_use]
pub const fn title(points: u32) -> Option<&'static str> {
    match points {
        0..=49 => None,
        50..=99 => Some("Scout of Tibia"),
        100..=199 => Some("Sentinel of Tibia"),
        200..=399 => Some("Steward of Tibia"),
        400..=999 => Some("Warden of Tibia"),
        1_000..=1_999 => Some("Squire of Tibia"),
        2_000..=2_999 => Some("Warrior of Tibia"),
        3_000..=3_999 => Some("Keeper of Tibia"),
        4_000..=4_999 => Some("Guardian of Tibia"),
        5_000..=5_999 => Some("Sage of Tibia"),
        6_000..=6_999 => Some("Savant of Tibia"),
        _ => Some("Enlightened of Tibia"),
    }
}

/// Project a stored skill family by adding a share of its lifetime points to the current
/// level's remainder. `family` is zero for magic and 1..=7 for ordinary skills.
#[must_use]
pub fn projected_level(
    formula: &impl BuildFormula,
    vocation: &str,
    family: usize,
    level: u16,
    progress: u64,
    loyalty_points: u32,
) -> u16 {
    let percent = bonus_percent(loyalty_points);
    if percent == 0 || family > 7 || level >= MAX_BUILD_LEVEL {
        return level;
    }
    let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
    let required = |next| formula.required(vocation, family, next).map(u128::from);
    let progress = u128::from(progress);
    // The storage bound applies to one level's remainder. The lifetime total sums up to
    // MAX_BUILD_LEVEL individual requirements, safely within u128 even with the multiplier.
    let Some(total) = (floor + 1..=level).try_fold(progress, |total, completed| {
        Some(total + required(completed)?)
    }) else {
        return level;
    };
    let mut available = progress + total * u128::from(percent) / 100;
    let mut projected = level;
    for next in level + 1..=MAX_BUILD_LEVEL {
        let Some(cost) = required(next).filter(|&cost| cost <= available) else {
            break;
        };
        available -= cost;
        projected = next;
    }
    projected
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use oteryn_gameplay_content::content::training_formula::CompiledTrainingFormula;

    fn formula() -> CompiledTrainingFormula {
        CompiledTrainingFormula::from_profile(
            include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
            "actual-content-r1",
        )
        .expect("formula")
    }

    #[test]
    fn loyalty_titles_use_highest_met_donor_threshold() {
        let thresholds = [
            (50, "Scout of Tibia"),
            (100, "Sentinel of Tibia"),
            (200, "Steward of Tibia"),
            (400, "Warden of Tibia"),
            (1_000, "Squire of Tibia"),
            (2_000, "Warrior of Tibia"),
            (3_000, "Keeper of Tibia"),
            (4_000, "Guardian of Tibia"),
            (5_000, "Sage of Tibia"),
            (6_000, "Savant of Tibia"),
            (7_000, "Enlightened of Tibia"),
        ];
        let mut previous = None;
        for (points, name) in thresholds {
            assert_eq!(title(points - 1), previous);
            assert_eq!(title(points), Some(name));
            previous = Some(name);
        }
        assert_eq!(title(0), None);
        assert_eq!(title(u32::MAX), Some("Enlightened of Tibia"));
    }

    #[test]
    fn every_loyalty_bonus_threshold_is_inclusive_and_capped() {
        assert_eq!(bonus_percent(0), 0);
        for bracket in 1..=10 {
            let points = bracket * 360;
            assert_eq!(bonus_percent(points - 1), ((bracket - 1) * 5) as u8);
            assert_eq!(bonus_percent(points), (bracket * 5) as u8);
        }
        assert_eq!(bonus_percent(3_960), 50);
        assert_eq!(bonus_percent(u32::MAX), 50);
    }

    #[test]
    fn global_brackets_cap_and_preserve_base_progress() {
        assert_eq!(bonus_percent(359), 0);
        assert_eq!(bonus_percent(360), 5);
        assert_eq!(bonus_percent(3_600), 50);
        assert_eq!(bonus_percent(u32::MAX), 50);
        let formula = formula();
        let base = projected_level(&formula, "paladin", 6, 15, 0, 0);
        let loyal = projected_level(&formula, "paladin", 6, 15, 0, 2_160);
        assert_eq!(base, 15);
        assert!(loyal > base);
    }

    #[test]
    fn magic_and_skill_use_total_points_not_level_percentage() {
        let formula = formula();
        assert!(projected_level(&formula, "druid", 0, 89, 0, 360) >= 89);
        assert!(projected_level(&formula, "knight", 3, 100, 0, 1_800) < 125);
    }

    #[test]
    fn displayed_loyalty_levels_follow_the_native_training_formula() {
        let formula = formula();
        assert_eq!(projected_level(&formula, "paladin", 0, 30, 0, 3_600), 31);
        for (family, expected) in [
            (1, 102),
            (2, 102),
            (3, 102),
            (4, 102),
            (5, 104),
            (6, 104),
            (7, 104),
        ] {
            assert_eq!(
                projected_level(&formula, "paladin", family, 100, 0, 3_600),
                expected
            );
        }
        assert_eq!(projected_level(&formula, "paladin", 8, 100, 0, 3_600), 100);
        assert_eq!(projected_level(&formula, "paladin", 3, 100, 0, 0), 100);
    }
}
