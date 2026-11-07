use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cast_control_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::cast_control::open_key(events) {
            return false;
        }
        self.cast_control_open();
        true
    }
    pub fn cast_control_open(&self) {
        if lock(&self.mailbox).cast_control.consent_selected {
            self.cast_control_request(2);
        }

        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).cast_control.open = true;
        }
    }
    pub fn cast_control_request(&self, action: usize) {
        let mut shared = lock(&self.mailbox);
        submit(&mut shared, &self.commands, action);
    }
}
pub(in crate::play) fn submit(
    shared: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    action: usize,
) {
    let panel = &mut shared.cast_control;
    let message = if action >= 2 {
        oteryn_client_ui::cast_consent::protocol::request(
            panel,
            match action {
                2 => None,
                3 => Some(true),
                _ => Some(false),
            },
        )
    } else {
        oteryn_client_ui::cast_control::command(panel, action)
    };
    match message.and_then(|m| m.encode().map_err(|_| "Invalid settings")) {
        Ok(payload) => {
            let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
                capability: if action >= 2 { 292 } else { 290 },
                command_type: if action >= 2 { 503 } else { 502 },
                payload,
            };
            panel.pending = commands
                .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                .is_ok();
            if !panel.pending {
                panel.feedback = "The session has ended".into();
            }
        }
        Err(message) => panel.feedback = message.into(),
    }
}
