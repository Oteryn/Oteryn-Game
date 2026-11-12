//! Refusal-safe positions shared by all clones of a scoped owner.
use super::ComposedFreshAdmission;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn background_start(&self, lane: &'static str, len: usize) -> usize {
        self.states
            .background_cursors
            .lock()
            .ok()
            .and_then(|cursors| cursors.get(lane).copied())
            .unwrap_or(0)
            % len.max(1)
    }

    /// Rotate a branch containing several units without sharing their grants.
    pub(super) async fn background_turn<F: Future>(
        &self,
        lane: &'static str,
        next: usize,
        future: F,
    ) -> F::Output {
        let (output, acquired) = self.root.with_background_sweep_progress(future).await;
        if acquired && let Ok(mut cursors) = self.states.background_cursors.lock() {
            cursors.insert(lane, next);
        }
        output
    }

    pub(super) async fn background_unit<F: Future>(
        &self,
        lane: &'static str,
        next: usize,
        future: F,
    ) -> F::Output {
        let (output, acquired) = self.root.with_background_operation_progress(future).await;
        if acquired && let Ok(mut cursors) = self.states.background_cursors.lock() {
            cursors.insert(lane, next);
        }
        output
    }
}
