use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn boss_difficulty_initialize(&self, selected: bool) {
        lock(&self.0).boss_difficulty.selected = selected;
    }
    pub(in crate::play) fn boss_difficulty_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::boss_difficulty::apply_result(&mut lock(&self.0).boss_difficulty, result);
    }
}
