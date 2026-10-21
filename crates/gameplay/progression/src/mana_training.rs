//! Pure skill and mana advancement from paid training progress.
use oteryn_gameplay_content::content::training_formula::CompiledTrainingFormula;
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
