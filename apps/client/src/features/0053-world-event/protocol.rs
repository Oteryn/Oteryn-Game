use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn world_event_initialize(&self, selected: bool, seasonal_selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.world_event.selected = selected;
        mailbox.world_event.seasonal_selected = selected && seasonal_selected;
    }
    pub(in crate::play) fn world_event_result(&self, result: &GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        if mailbox.world_event.seasonal_selected {
            oteryn_client_ui::seasonal_events::apply_result(&mut mailbox.world_event, result);
        } else {
            oteryn_client_ui::world_event::apply_result(&mut mailbox.world_event, result);
        }
    }
}
