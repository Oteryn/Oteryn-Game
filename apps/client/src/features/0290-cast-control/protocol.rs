use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn cast_control_initialize(&self, selected: bool) {
        lock(&self.0).cast_control = oteryn_client_ui::cast_control::Panel {
            selected,
            ..Default::default()
        };
    }
    pub(in crate::play) fn cast_control_result(
        &self,
        result: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.cast_control;
        panel.pending = false;
        let value = oteryn_client_runtime::cast::Message::decode(&result.payload);
        if result.status != oteryn_session::CommandStatus::Accepted
            || value.ok().and_then(|v| panel.apply(v).ok()).is_none()
        {
            panel.feedback = "Cast request refused; refresh saved account settings".into();
        }
    }
}
