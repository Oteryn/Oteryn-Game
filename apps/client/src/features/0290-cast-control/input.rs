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
        let control = lock(&self.mailbox).cast_control.clone();
        if control.selected {
            self.cast_control_request(5);
        } else if control.consent_selected {
            self.cast_control_request(2);
        }

        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
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
    match command(panel, action) {
        Ok(command) => {
            panel.pending = commands
                .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                .is_ok();
            if !panel.pending {
                panel.feedback = tr!("The session has ended").into();
            }
        }
        Err(message) => panel.feedback = message.into(),
    }
}

/// The shared action mapping for native and bound cast submissions.
pub(in crate::play) fn command(
    panel: &oteryn_client_ui::cast_control::Panel,
    action: usize,
) -> Result<oteryn_client_runtime::server_mechanic::ServerMechanicCommand, &'static str> {
    let message = if (2..=4).contains(&action) {
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
    let payload = message?.encode().map_err(|_| tr!("Invalid settings"))?;
    Ok(
        oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
            capability: if (2..=4).contains(&action) { 292 } else { 290 },
            command_type: if (2..=4).contains(&action) { 503 } else { 502 },
            payload,
        },
    )
}
