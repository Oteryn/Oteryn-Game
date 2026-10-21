//! Pure skill and mana advancement from paid training progress.
use crate::hit_training::SkillFamily;
use oteryn_gameplay_content::content::training_formula::{CompiledTrainingFormula, TrainingError};
use oteryn_gameplay_domain::domain::character_build::{
    BuildFormula, MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS,
};

/// Bank `pending` onto `(level, progress)` of `family` (0 magic, 1..=7 the skills): every level
/// the formula reaches advances; progress saturates at the stored bound.
pub fn advance(
    formula: &CompiledTrainingFormula,
    vocation: &str,
    family: usize,
    (mut level, stored): (u16, u64),
    pending: u64,
) -> (u16, u64) {
    let mut progress = stored.saturating_add(pending).min(MAX_BUILD_PROGRESS);
    while level < MAX_BUILD_LEVEL {
        let Some(required) = formula.required(vocation, family, level + 1) else {
            break;
        };
        if progress < required {
            break;
        }
        progress -= required;
        level += 1;
    }
    (level, progress)
}

/// Whether the pending tries of `family` reach its next level.
pub fn skill_level_due(
    formula: &CompiledTrainingFormula,
    vocation: &str,
    family: SkillFamily,
    skill: (u16, u64),
    pending: u64,
) -> Result<bool, TrainingError> {
    if formula.required(vocation, 0, 1).is_none() {
        return Err(TrainingError::UnknownVocation);
    }
    Ok(advance(formula, vocation, family.index() + 1, skill, pending).0 != skill.0)
}

/// Whether pending mana or any pending skill tries reach a level now.
pub fn level_due(
    formula: &CompiledTrainingFormula,
    vocation: &str,
    magic: (u16, u64),
    pending_mana: u64,
    skills: [(u16, u64); 7],
    pending_skills: [u64; 7],
) -> bool {
    if formula.required(vocation, 0, 1).is_none() {
        return false;
    }
    advance(formula, vocation, 0, magic, pending_mana).0 != magic.0
        || SkillFamily::ALL.iter().any(|family| {
            skill_level_due(
                formula,
                vocation,
                *family,
                skills[family.index()],
                pending_skills[family.index()],
            )
            .unwrap_or(false)
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn formula() -> CompiledTrainingFormula {
        CompiledTrainingFormula::from_profile(
            include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
            "actual-content-r1",
        )
        .unwrap()
    }
    #[test]
    fn source_float_storage_and_all_vocations_are_preserved() {
        let f = formula();
        assert_eq!(f.required("druid", 0, 1), Some(1600));
        assert_eq!(f.required("druid", 0, 2), Some(1760));
        assert_eq!(f.required("knight", 0, 2), Some(4800));
        assert_eq!(f.required("none", 0, 2), Some(6400));
        assert_eq!(f.required("missing", 0, 1), None);
        assert_eq!(f.required("druid", 7, 11), Some(20));
        // SKILLS-0 §2: the elite knight trains skills on the knight row, not the swapped one.
        assert_eq!(f.required("elite_knight", 1, 12), Some(55));
        // 30 x 1.4f (binary32, 1.39999998) floors to 41, as the pinned source computes it.
        assert_eq!(f.required("elite_knight", 5, 12), Some(41));
        assert_eq!(
            f.required("elite_knight", 5, 12),
            f.required("knight", 5, 12)
        );
        for (promoted, base) in [
            ("elder_druid", "druid"),
            ("elite_knight", "knight"),
            ("exalted_monk", "monk"),
            ("master_sorcerer", "sorcerer"),
            ("royal_paladin", "paladin"),
        ] {
            for family in 0..=7 {
                assert_eq!(
                    f.required(promoted, family, 12),
                    f.required(base, family, 12)
                );
            }
        }
        let mut profile: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/build-training.json"
        ))
        .unwrap();
        profile["magic_base"] = 1700.into();
        assert!(
            CompiledTrainingFormula::from_profile(
                &serde_json::to_vec(&profile).unwrap(),
                "actual-content-r1"
            )
            .is_err()
        );
    }
}
