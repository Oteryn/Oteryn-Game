use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn training_panel(&self) -> oteryn_client_ui::training::Panel {
        lock(&self.mailbox).properties.training.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Training",
            labels: &[
                "Offline statue handle",
                "Exercise weapon handle",
                "Exercise dummy handle",
            ],
            buttons: &["Offline", "Start", "Status", "Stop"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.training;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::training::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.training.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.training;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::training::Intent::Offline,
                    1 => oteryn_client_ui::training::Intent::Start,
                    2 => oteryn_client_ui::training::Intent::Status,
                    3 => oteryn_client_ui::training::Intent::Stop,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn training_test_selected(&self, selected: bool) {
        lock(&self.0).properties.training.selected = selected;
    }
    pub(crate) fn training_test_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        disposition: u8,
        charges: u32,
        pool_seconds: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
        let payload = oteryn_session::property_services::loop_training::encode_training_result(
            oteryn_session::property_services::loop_training::TrainingResult {
                disposition,
                revision: 1,
                charges,
                pool_seconds,
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        self.properties_result(
            command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
        Ok(())
    }
}

/// A rendered training view belongs to exactly one retained session mailbox.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct TrainingBinding {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub(crate) panel: oteryn_client_ui::training::Panel,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn training_binding(&self) -> TrainingBinding {
        TrainingBinding {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: lock(&self.mailbox).properties.training.clone(),
        }
    }
    /// Re-prove the session, targets and modal gate before preparing one canonical command.
    pub(crate) fn training_dispatch_bound(
        &self,
        expected: &TrainingBinding,
        fields: [String; 3],
        intent: Option<oteryn_client_ui::training::Intent>,
        confirmed: bool,
    ) -> bool {
        use oteryn_client_ui::training::Intent;
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || mailbox.activity_reset_commands
            || mailbox.properties.training != expected.panel
            || !expected.panel.open
        {
            return false;
        }
        let Some(intent) = intent else {
            mailbox.properties.training.open = false;
            return true;
        };
        if !expected.panel.selected
            || expected.panel.pending
            || (intent != Intent::Status && !confirmed)
            || fields
                .iter()
                .any(|field| field.len() > 7996 || field.contains('\0'))
        {
            return false;
        }
        let mut next = expected.panel.clone();
        next.fields = fields;
        let Ok(command) = super::protocol::command(&next, intent) else {
            return false;
        };
        if !oteryn_client_ui::training::request(&mut next, command.kind)
            || self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
        {
            return false;
        }
        mailbox.properties.training = next;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
