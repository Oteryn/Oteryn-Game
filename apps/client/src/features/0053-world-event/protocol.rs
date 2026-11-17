use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn world_event_initialize(&self, selected: bool) {
        lock(&self.0).world_event.selected = selected;
    }
    pub(in crate::play) fn world_event_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::world_event::apply_result(&mut lock(&self.0).world_event, result);
    }
}
