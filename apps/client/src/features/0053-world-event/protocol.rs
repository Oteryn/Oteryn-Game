use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn world_event_initialize(&self, selected: bool) {
        lock(&self.0).world_event.selected = selected;
    }
    pub(in crate::play) fn world_event_result(&self, result: &GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        #[cfg(feature = "bevy-shell")]
        if mailbox.shared_frontend {
            // Closing cannot cancel transport. Drain its reply before allowing another query.
            if mailbox.world_event_session_closed() || !mailbox.world_event.pending {
                return;
            }
            if !mailbox.world_event.open {
                mailbox.world_event.pending = false;
                return;
            }
        }
        oteryn_client_ui::world_event::apply_result(&mut mailbox.world_event, result);
    }
}
