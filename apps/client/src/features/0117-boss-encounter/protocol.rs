use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn boss_encounter_initialize(&self, selected: bool) {
        lock(&self.0).boss_encounter.selected = selected;
    }
    pub(in crate::play) fn boss_encounter_result(&self, response: &GatedCommandResult) {
        oteryn_client_ui::boss_encounter::apply_result(&mut lock(&self.0).boss_encounter, response);
    }
}
