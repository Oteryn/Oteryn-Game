use crate::play::{Outbox, Stepper, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn targeting_refresh(&self, session: &impl Stepper) {
        let mut mailbox = lock(&self.0);
        if let Some(source) = mailbox.world_context.targeting {
            oteryn_client_ui::item_use::source_current(
                &mut mailbox.world_context,
                session.targeting_source_current(source),
            );
        }
    }
}
