//! Global loyalty skill-point projection. Loyalty never mutates trained progress.
use oteryn_gameplay_domain::domain::character_build::{
    BuildFormula, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, cumulative_progress, relevel,
};

/// Five percentage points per 360 loyalty points, capped at fifty percent.
#[must_use]
pub const fn bonus_percent(points: u32) -> u8 {
    let earned = (points / 360).saturating_mul(5);
    if earned > 50 { 50 } else { earned as u8 }
}

/// Project a stored skill family to its displayed/effective level by increasing its total
/// skill points. `family` is zero for magic and 1..=7 for ordinary skills.
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
    if percent == 0 || family > 7 {
        return level;
    }
    let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
    let total = cumulative_progress(level, progress, floor, |next| {
        formula.required(vocation, family, next)
    });
    let bonus = u128::from(total) * u128::from(percent) / 100;
    let projected = u128::from(total)
        .saturating_add(bonus)
        .min(u128::from(MAX_BUILD_PROGRESS)) as u64;
    relevel(projected, floor, |next| {
        formula.required(vocation, family, next)
    })
    .0
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
}
