use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::account_achievements::Response;
impl Outbox {
    pub(in crate::play) fn achievements_initialize(&self) {
        lock(&self.0).achievements.selected = true;
    }
    pub(in crate::play) fn achievements_result(&self, response: &Response) {
        oteryn_client_ui::account_achievements::apply_result(
            &mut lock(&self.0).achievements,
            response,
        );
    }
}
