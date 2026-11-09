use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn cast_consent_initialize(&self, selected: bool) {
        lock(&self.0).cast_control.consent_selected = selected;
    }

    pub(in crate::play) fn cast_control_initialize(&self, selected: bool) {
        lock(&self.0).cast_control = oteryn_client_ui::cast_control::Panel {
            selected,
            ..Default::default()
        };
    }
    pub(in crate::play) fn cast_consent_result(
        &self,
        result: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.cast_control;
        panel.pending = false;
        if result.status != oteryn_session::CommandStatus::Accepted
            || oteryn_client_ui::cast_consent::protocol::apply(panel, &result.payload).is_err()
        {
            panel.feedback = tr!("Character consent unavailable; refresh and try again").into();
        }
    }
    pub(in crate::play) fn cast_control_result(
        &self,
        result: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.cast_control;
        panel.pending = false;
        let value = oteryn_client_runtime::cast::Message::decode(&result.payload);
        let stopped = matches!(value, Ok(oteryn_client_runtime::cast::Message::End));
        if result.status != oteryn_session::CommandStatus::Accepted
            || value
                .ok()
                .filter(|v| {
                    matches!(
                        v,
                        oteryn_client_runtime::cast::Message::CasterState { .. }
                            | oteryn_client_runtime::cast::Message::End
                    )
                })
                .and_then(|v| panel.apply(v).ok())
                .is_none()
        {
            panel.feedback =
                tr!("Cast unavailable; refresh consent or saved account settings").into();
        }
        if stopped && result.status == oteryn_session::CommandStatus::Accepted {
            shared.cast_chat.lines.clear();
            shared.cast_chat.subjects.clear();
            shared.cast_chat.roster_revision = 0;
        }
    }
}
