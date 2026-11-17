//! Targeting state shared by inventory and world context actions.
use crate::world_interaction::menu::State;
impl State {
    pub fn begin_targeting(&mut self, source: oteryn_client_runtime::inventory::ItemHandle) {
        self.targeting = Some(source);
        self.rune_targeting = false;
        self.set_feedback(
            tr!("Use with: choose a target (Escape cancels)").into(),
            std::time::Instant::now(),
        );
    }
    pub fn cancel_targeting(&mut self) -> bool {
        if self.targeting.take().is_none() {
            return false;
        }
        self.set_feedback(tr!("Use with cancelled").into(), std::time::Instant::now());
        true
    }
}

/// Apply authoritative source validity after the session has reduced its pushes.
pub fn source_current(state: &mut State, current: bool) {
    if !current && state.targeting.take().is_some() {
        state.set_feedback(
            tr!("Use with: source item disappeared").into(),
            std::time::Instant::now(),
        );
    }
}
