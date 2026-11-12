use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn arena_initialize(&self, selected: bool) {
        lock(&self.0).arena.selected = selected;
    }
    pub(in crate::play) fn arena_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::arena::apply_result(&mut lock(&self.0).arena, result);
    }
}
