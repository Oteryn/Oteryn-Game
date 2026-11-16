use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::hireling::HirelingResponse;

impl Outbox {
    pub(in crate::play) fn hireling_initialize(&self, selected: bool) {
        lock(&self.0).hireling.selected = selected;
    }

    pub(in crate::play) fn hireling_result(&self, response: &HirelingResponse) {
        oteryn_client_ui::hireling::apply_result(&mut lock(&self.0).hireling, response);
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    /// Admits the banker hireling (165) without its management window (250).
    pub(crate) fn bevy_hireling_selected(&self) {
        self.hireling_initialize(true);
    }
}
