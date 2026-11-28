//! Targeting state shared by inventory and world context actions.
use crate::world_interaction::menu::State;
impl State {
    pub fn begin_targeting(&mut self, source: oteryn_client_runtime::inventory::ItemHandle) {
        self.begin_shortcut_targeting(Some(source), None);
    }
    pub fn targeting_action(&self) -> Option<oteryn_client_runtime::world_interaction::Action> {
        use oteryn_client_runtime::world_interaction::Action;
        self.targeting
            .map(Action::UseOn)
            .or_else(|| self.targeting_definition.map(Action::UseDefinitionOn))
    }
    pub fn clear_targeting(&mut self) {
        self.targeting = None;
        self.targeting_definition = None;
    }
    pub fn begin_shortcut_targeting(
        &mut self,
        handle: Option<oteryn_client_runtime::inventory::ItemHandle>,
        definition: Option<std::num::NonZeroU32>,
    ) {
        self.targeting = handle;
        self.targeting_definition = definition;
        self.rune_targeting = false;
        self.set_feedback(
            tr!("Use with: choose a target (Escape cancels)").into(),
            std::time::Instant::now(),
        );
    }
    pub fn cancel_targeting(&mut self) -> bool {
        if self.targeting_action().is_none() {
            return false;
        }
        self.clear_targeting();
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
