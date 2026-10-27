//! Production-facing Charm hook evaluation.

use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmEvaluationError, CharmEvaluationInput, CharmHookEvent, CharmOutcome,
    CharmStateRead, evaluate_charm_hook,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

/// Evaluate one Charm hook against the durable assignment view supplied by the dispatch owner.
/// The result cannot be silently discarded by this boundary: dispatch must apply it or fail the
/// event closed.
pub fn run_charm_hook(
    catalogue: &dyn CharmCatalogueRead,
    state: &dyn CharmStateRead,
    character: [u8; 16],
    race_key: &str,
    decision_root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) -> Result<Vec<CharmOutcome>, CharmEvaluationError> {
    evaluate_charm_hook(
        &CharmEvaluationInput {
            character,
            race_key,
            decision_root,
            occurrence,
            event,
        },
        state,
        catalogue,
    )
}
