//! Atomic presentation admission over the retained private-channel owner.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::chat_private_channel::{ChatPanel, Intent, request};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub(crate) struct PrivateChannelsBound {
    pub(crate) panel: ChatPanel,
    commands: UnboundedSender<PlayCommand>,
}
impl PrivateChannelsBound {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        self.commands.same_channel(&other.commands)
    }
}
impl PartialEq for PrivateChannelsBound {
    fn eq(&self, other: &Self) -> bool {
        self.same_session(other) && self.panel == other.panel
    }
}
impl PlayLink {
    pub(crate) fn chat_private_bound(&self) -> PrivateChannelsBound {
        PrivateChannelsBound {
            panel: self.chat_private_panel(),
            commands: self.commands.clone(),
        }
    }
    /// Close admission deliberately ignores capability, draft, pending and input fences.
    pub(crate) fn chat_private_admit_bound(
        &self,
        expected: &PrivateChannelsBound,
        closing: bool,
    ) -> bool {
        let mailbox = lock(&self.mailbox);
        self.commands.same_channel(&expected.commands)
            && !self.commands.is_closed()
            && !mailbox.chat_public_shutdown
            && !mailbox.custom_hotkeys_ended
            && (closing
                || (!mailbox.world_input_blocked()
                    && !mailbox.server_modal.is_active()
                    && !crate::play::feature_0450_server_modal::view::active()
                    && mailbox.chat_private == expected.panel))
    }
    pub(crate) fn chat_private_select_bound(
        &self,
        expected: &PrivateChannelsBound,
        channel: u16,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.same_channel(&expected.commands)
            || self.commands.is_closed()
            || mailbox.chat_public_shutdown
            || mailbox.custom_hotkeys_ended
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || crate::play::feature_0450_server_modal::view::active()
            || mailbox.chat_private != expected.panel
            || mailbox.chat_private.pending
            || !mailbox.chat_private.selected
            || !mailbox.chat_private.list.as_ref().is_some_and(|list| {
                list.own_channel == Some(channel)
                    || list.invited.iter().any(|row| row.channel_id == channel)
            })
        {
            return false;
        }
        mailbox.chat_private.channel = Some(channel);
        true
    }
    pub(crate) fn chat_private_dispatch_bound(
        &self,
        expected: &PrivateChannelsBound,
        intent: Intent,
        name: &str,
        text: &str,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.same_channel(&expected.commands)
            || self.commands.is_closed()
            || mailbox.chat_public_shutdown
            || mailbox.custom_hotkeys_ended
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || crate::play::feature_0450_server_modal::view::active()
            || mailbox.chat_private != expected.panel
            || !mailbox.chat_private.selected
            || mailbox.chat_private.pending
            || (matches!(intent, Intent::Exclude | Intent::Leave) && !confirmed)
        {
            return false;
        }
        // Prepare through the existing owner validator before attempting one dispatch.
        let mut prepared = mailbox.chat_private.clone();
        prepared.name = name.into();
        prepared.text = text.into();
        let Some(command) = request(&mut prepared, intent) else {
            // Match native draft validation feedback; server-backed data is untouched.
            mailbox.chat_private = prepared;
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ChatPublic(
                oteryn_session::ChatIntent::PrivateChannel(command),
            ))
            .is_err()
        {
            return false;
        }
        mailbox.chat_private = prepared;
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn private_channels_test_panel(&self, panel: ChatPanel) {
        lock(&self.0).chat_private = panel;
    }
    pub(crate) fn private_channels_test_block(&self, blocked: bool) {
        lock(&self.0).death.dead = blocked;
    }
    pub(crate) fn private_channels_test_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
