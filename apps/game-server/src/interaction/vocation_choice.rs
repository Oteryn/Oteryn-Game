//! DAWNPORT-1: adapt a qualified owner choice into the existing A13 build-writer request.
//!
//! This prepares an intent, not authority or a committed choice. The owner must qualify the
//! selected vocation and current Dawnport interaction facts from active native content, flush
//! pending training, and read the current build/stance under its existing authority boundary.
//! Stance fit remains UNKNOWN: only the owner may supply an admitted prune decision. This
//! adapter infers no island, level, departure, stance-fit or magic-level-cap rule.
//! The request goes through RevisionSlot::commit_build; the existing writer owns current
//! session/recovery fences, revision/before-state checks, cause validation, binding and replay.

use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::character_progression::CharacterProgressionError;

/// Always derive `after` using the same cumulative-progress conversion the writer verifies
/// (SKILLS-0 §3.3), never by copying raw levels/tries or resetting progress. A formula's `None`
/// keeps its existing meaning, unreachable; it is not a vocation-admission decision.
///
/// `pruned_stance` is the owner's already-qualified decision: `None` leaves the stored stance
/// alone, `Some` names the stored key to prune in this same build receipt. No extra stance
/// receipt is created. The occurrence is preserved for the writer's existing replay contract.
pub(crate) fn vocation_choice_request(
    occurrence: BuildOccurrence,
    before: DurableBuildState,
    selected_vocation: &str,
    formula: &dyn BuildFormula,
    pruned_stance: Option<String>,
) -> Result<BuildChangeRequest, CharacterProgressionError> {
    let after = convert_vocation(formula, &before, selected_vocation)?;
    Ok(BuildChangeRequest {
        occurrence,
        cause: BuildCause::VocationChoice,
        before,
        after,
        pruned_stance,
    })
}
