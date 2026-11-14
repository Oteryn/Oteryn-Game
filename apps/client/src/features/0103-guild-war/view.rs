use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guild_war_panel(&self) -> oteryn_client_ui::guild_war::Panel {
        lock(&self.mailbox).properties.guild_war.clone()
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
            title: "Guild wars",
            labels: &[
                "Opponent guild UUID (declare) or war UUID (accept)",
                "Frag limit (declare)",
                "Duration in days (declare)",
            ],
            buttons: &["Declare", "Accept"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.guild_war;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::guild_war::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.guild_war.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.guild_war;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::guild_war::Intent::Declare,
                    1 => oteryn_client_ui::guild_war::Intent::Accept,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

/// A presentation snapshot tied to the actual connection command channel.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub struct GuildWarsBinding {
    pub panel: oteryn_client_ui::guild_war::Panel,
    commands: tokio::sync::mpsc::WeakUnboundedSender<crate::combat_input::PlayCommand>,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn guild_wars_binding(&self) -> GuildWarsBinding {
        GuildWarsBinding {
            panel: lock(&self.mailbox).properties.guild_war.clone(),
            commands: self.commands.downgrade(),
        }
    }

    /// Validate the displayed target and session and queue through the existing owner atomically.
    pub fn guild_wars_submit_bound(
        &self,
        expected: &GuildWarsBinding,
        fields: &[String; 3],
        intent: oteryn_client_ui::guild_war::Intent,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mailbox.properties.guild_war;
        if expected
            .commands
            .upgrade()
            .is_none_or(|commands| !self.commands.same_channel(&commands))
            || self.commands.is_closed()
            || *panel != expected.panel
            || !panel.open
            || !panel.selected
            || panel.pending
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || mailbox.server_modal.is_active()
            || fields
                .iter()
                .any(|value| value.len() > 7996 || value.contains('\0'))
        {
            return false;
        }
        mailbox.properties.guild_war.fields = fields.clone();
        super::input::submit(&mut mailbox, &self.commands, intent);
        mailbox.properties.guild_war.pending
    }

    pub fn guild_wars_close_bound(&self, expected: &GuildWarsBinding) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if expected
            .commands
            .upgrade()
            .is_none_or(|commands| !self.commands.same_channel(&commands))
            || mailbox.properties.guild_war != expected.panel
            || !expected.panel.open
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.properties.guild_war.open = false;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn guild_wars_test_selected(&self) {
        lock(&self.0).properties.guild_war.selected = true;
    }
    pub(crate) fn guild_wars_test_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) {
        let mut war = [1; 16];
        war[6] = 0x70;
        war[8] = 0x80;
        let payload = oteryn_client_runtime::guild_war::encode_guild_war_result(
            &oteryn_client_runtime::guild_war::GuildWarResult { code: 1, war },
        );
        if let Ok(payload) = payload {
            super::protocol::apply(
                &mut lock(&self.0).properties.guild_war,
                command,
                &oteryn_client_runtime::raw_feature::GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    payload,
                },
            );
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
