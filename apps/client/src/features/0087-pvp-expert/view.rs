use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn pvp_expert_panel(&self) -> oteryn_client_ui::pvp_expert::Panel {
        lock(&self.mailbox).properties.pvp_expert.clone()
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
            title: "PvP expert",
            labels: &[
                "Fight: 1 offensive, 2 balanced, 3 defensive",
                "Chase: 1 stand, 2 chase",
                "Secure: 0 off, 1 on",
                "Expert: 0 dove, 1 white hand, 2 yellow hand, 3 red fist",
            ],
            buttons: &["Apply"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.pvp_expert;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::pvp_expert::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.pvp_expert.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.pvp_expert;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::pvp_expert::Intent::Apply,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bevy_bound.rs"]
mod bevy_bound_tests;

/// A view bound to the connection that supplied it, never just a copied tuple of modes.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct ExpertPvpModesSnapshot {
    pub(crate) panel: oteryn_client_ui::pvp_expert::Panel,
    commands: tokio::sync::mpsc::WeakUnboundedSender<crate::combat_input::PlayCommand>,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for ExpertPvpModesSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.commands
            .upgrade()
            .zip(other.commands.upgrade())
            .is_some_and(|(left, right)| left.same_channel(&right))
            && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy)]
pub(crate) enum ExpertPvpModesAction {
    Open,
    Close,
    Field(usize, u8),
    Apply,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn expert_pvp_modes_snapshot(&self) -> ExpertPvpModesSnapshot {
        ExpertPvpModesSnapshot {
            panel: lock(&self.mailbox).properties.pvp_expert.clone(),
            commands: self.commands.downgrade(),
        }
    }
    /// Compare and enqueue under the existing owner lock. Rejection does not edit feedback.
    pub(crate) fn expert_pvp_modes_dispatch_bound(
        &self,
        expected: &ExpertPvpModesSnapshot,
        action: ExpertPvpModesAction,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .commands
            .upgrade()
            .is_some_and(|commands| commands.same_channel(&self.commands))
            || self.commands.is_closed()
            || mailbox.properties.pvp_expert != expected.panel
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        let panel = &mut mailbox.properties.pvp_expert;
        if matches!(action, ExpertPvpModesAction::Open) {
            if panel.open {
                return false;
            }
            panel.open = true;
            return true;
        }
        if !panel.open {
            return false;
        }
        if matches!(action, ExpertPvpModesAction::Close) {
            panel.open = false;
            return true;
        }
        if !panel.selected || panel.pending {
            return false;
        }
        match action {
            ExpertPvpModesAction::Field(field, value) => {
                let valid = match field {
                    0 => (1..=3).contains(&value),
                    1 => (1..=2).contains(&value),
                    2 => value <= 1,
                    3 => value <= 3,
                    _ => false,
                };
                if !valid {
                    return false;
                }
                super::input::edit(panel, field, value.to_string());
                true
            }
            ExpertPvpModesAction::Apply => {
                // Validate before calling the canonical submitter so refusal is read-only.
                if super::protocol::command(panel, oteryn_client_ui::pvp_expert::Intent::Apply)
                    .is_err()
                {
                    return false;
                }
                super::input::submit(
                    &mut mailbox,
                    &self.commands,
                    oteryn_client_ui::pvp_expert::Intent::Apply,
                );
                if mailbox.properties.pvp_expert.pending {
                    true
                } else {
                    // The receiver may close after is_closed() but before enqueueing.
                    mailbox.properties.pvp_expert.clone_from(&expected.panel);
                    false
                }
            }
            ExpertPvpModesAction::Open | ExpertPvpModesAction::Close => false,
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn expert_pvp_modes_test_state(&self, panel: oteryn_client_ui::pvp_expert::Panel) {
        lock(&self.0).properties.pvp_expert = panel;
    }
    pub(crate) fn expert_pvp_modes_test_push(
        &self,
        modes: oteryn_client_runtime::pvp_expert::ExpertFightModes,
    ) {
        self.push(vec![oteryn_session::SessionEvent::ExpertFightModes(
            oteryn_session::AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: modes,
            },
        )]);
    }
    pub(crate) fn expert_pvp_modes_test_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        status: oteryn_session::CommandStatus,
        payload: Vec<u8>,
    ) {
        super::protocol::apply(
            &mut lock(&self.0).properties.pvp_expert,
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status,
                payload,
            },
        );
    }
}
