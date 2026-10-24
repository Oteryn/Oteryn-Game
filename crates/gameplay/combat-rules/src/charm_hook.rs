//! Pure evaluation of auto-attack Charm hooks with no current assignments.

use oteryn_gameplay_progression::charm_effects::{
    CharmAssignment, CharmCatalogueRead, CharmEvaluationInput, CharmHookEvent, CharmStateRead,
    CharmStateUnavailable, evaluate_charm_hook,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

/// ATTACK-1b has no Charm state reader: every hook is evaluated against no assignments and its
/// outcomes are discarded. The Charm state wiring is a later stage.
struct NoCharmAssignments([u8; 16]);

impl CharmStateRead for NoCharmAssignments {
    fn character(&self) -> [u8; 16] {
        self.0
    }

    fn assignments_for_race(
        &self,
        _race_key: &str,
    ) -> Result<Vec<CharmAssignment>, CharmStateUnavailable> {
        Ok(Vec::new())
    }
}

/// Evaluate one Charm hook and discard its outcomes (see [`NoCharmAssignments`]).
pub fn run_charm_hook(
    catalogue: &dyn CharmCatalogueRead,
    character: [u8; 16],
    race_key: &str,
    decision_root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) {
    let _ = evaluate_charm_hook(
        &CharmEvaluationInput {
            character,
            race_key,
            decision_root,
            occurrence,
            event,
        },
        &NoCharmAssignments(character),
        catalogue,
    );
}
