use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_rotation_panel(&self) -> oteryn_client_ui::house_furniture_rotation::Panel {
        lock(&self.mailbox)
            .properties
            .house_furniture_rotation
            .clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    use oteryn_client_ui::house_furniture_rotation::Intent;
    open(
        link,
        Form {
            title: "Rotate house furniture",
            labels: &["Furniture number from the list"],
            buttons: &["Refresh", "Previous", "Next", "Rotate"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_furniture_rotation;
                (
                    panel.open,
                    panel.pending,
                    vec![panel.choice.clone()],
                    oteryn_client_ui::house_furniture_rotation::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_furniture_rotation.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_furniture_rotation;
                if panel.pending {
                    return;
                }
                if let Some(value) = values
                    .first()
                    .filter(|value| value.len() <= 3 && value.chars().all(|ch| ch.is_ascii_digit()))
                {
                    panel.choice = value.clone();
                } else if action == 3 {
                    panel.feedback = tr!("Choose a furniture number from the list").into();
                    return;
                }
                let intent = match action {
                    0 => Intent::Refresh,
                    1 => Intent::Previous,
                    2 => Intent::Next,
                    3 => Intent::Rotate,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
#[cfg(feature = "bevy-shell")]
mod bound {
    use super::*;
    use crate::play::mailbox::SharedMailbox;
    use oteryn_client_ui::house_furniture_rotation::{Intent, Panel};
    use std::sync::Arc;

    /// A displayed snapshot retains the actual session, not just matching catalogue bytes.
    #[derive(Clone)]
    pub(crate) struct RotationBinding {
        session: SharedMailbox,
        pub(crate) panel: Panel,
    }
    impl RotationBinding {
        pub(crate) fn can_rotate(&self) -> bool {
            super::super::protocol::command(&self.panel, Intent::Rotate).is_ok()
        }
        pub(crate) fn same_session(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.session, &other.session)
        }
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(crate) enum BoundAction {
        Toggle,
        Close,
        Select(usize),
        Request(Intent),
    }
    impl PlayLink {
        pub(crate) fn house_rotation_binding(&self) -> RotationBinding {
            RotationBinding {
                session: Arc::clone(&self.mailbox),
                panel: self.house_rotation_panel(),
            }
        }
        /// Validate and enqueue under the owner lock; the existing reducer owns replies.
        pub(crate) fn house_rotation_dispatch_bound(
            &self,
            expected: &RotationBinding,
            action: BoundAction,
        ) -> bool {
            if !Arc::ptr_eq(&self.mailbox, &expected.session) {
                return false;
            }
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.shared_frontend
                || self.commands.is_closed()
                || mailbox.server_modal.is_active()
                || mailbox.world_input_blocked()
            {
                return false;
            }
            let panel = &mut mailbox.properties.house_furniture_rotation;
            if panel != &expected.panel || !panel.selected {
                return false;
            }
            if action == BoundAction::Close || action == BoundAction::Toggle && panel.open {
                // Closing presentation does not cancel the retained owner's in-flight request.
                panel.open = false;
                return true;
            }
            if panel.pending || !panel.open && action != BoundAction::Toggle {
                return false;
            }
            let intent = match action {
                BoundAction::Select(index) => {
                    if panel
                        .catalogue
                        .as_ref()
                        .and_then(|catalogue| catalogue.entries.get(index))
                        .is_none()
                    {
                        return false;
                    }
                    panel.choice = (index + 1).to_string();
                    return true;
                }
                BoundAction::Toggle => Intent::Refresh,
                BoundAction::Request(intent) => intent,
                BoundAction::Close => return false,
            };
            let Ok(command) = super::super::protocol::command(panel, intent) else {
                return false;
            };
            // Sending and marking pending share the lock with the reply reducer.
            if self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command.clone()))
                .is_err()
            {
                return false;
            }
            panel.open = true;
            oteryn_client_ui::house_furniture_rotation::request(panel, command.payload)
        }
        pub(crate) fn house_rotation_toggle_input_bound(
            &self,
            events: &[oteryn_input_actions::NormalizedInputEvent],
        ) -> bool {
            if !oteryn_client_ui::house_furniture_rotation::open_key(events) {
                return false;
            }
            self.house_rotation_dispatch_bound(&self.house_rotation_binding(), BoundAction::Toggle);
            true
        }
    }
}
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{BoundAction, RotationBinding};

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn furniture_rotation_selected(&self) {
        lock(&self.0).properties.house_furniture_rotation.selected = true;
    }
    pub(crate) fn furniture_rotation_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        super::protocol::apply(
            &mut lock(&self.0).properties.house_furniture_rotation,
            command,
            result,
        );
    }
    pub(crate) fn furniture_rotation_change_target(&self) {
        use oteryn_client_runtime::raw_feature::house_rotation::Target;
        if let Some(catalogue) = &mut lock(&self.0).properties.house_furniture_rotation.catalogue
            && let Some(entry) = catalogue.entries.first_mut()
            && let Target::Item {
                expected_revision, ..
            } = &mut entry.target
        {
            *expected_revision += 1;
        }
    }
    pub(crate) fn furniture_rotation_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
