use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::chat::{Mode, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn chat_text(&self, text: String) {
        self.chat_composer(true, text);
    }
    pub fn chat_request(&self, mode: Mode, recipient: &str) {
        enqueue(&mut lock(&self.mailbox), &self.commands, mode, recipient);
    }
    pub fn chat_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::chat::default_channel_key(events) {
            self.chat_console_tab(oteryn_client_ui::chat::ConsoleTab::All);
            self.open_chat();
            return true;
        }
        if oteryn_client_ui::chat::temporary_mode_key(events) {
            lock(&self.mailbox).chat.console.mode = oteryn_client_ui::chat::ComposerMode::Temporary;
            self.open_chat();
            return true;
        }
        if !oteryn_client_ui::chat::open_key(events) {
            return false;
        }
        self.open_chat();
        true
    }
    pub fn open_chat(&self) {
        // Reopening the console refreshes opt-in current levels immediately.
        let display = {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.chat.open {
                mailbox.chat.console.close();
            }
            mailbox.chat.console.display
        };
        if let Err(message) = self.console_display(display) {
            lock(&self.mailbox).chat.feedback = Some(message.into());
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).chat.open = true;
        }
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    mode: Mode,
    recipient: &str,
) {
    // A selected channel tab receives the composer text; the fixed tabs use the speech modes.
    let intent = if mailbox.chat.console.current.is_some() {
        oteryn_client_ui::chat::channel_request(&mut mailbox.chat)
    } else {
        request(&mut mailbox.chat, mode, recipient)
    };
    if intent.is_none()
        && mailbox.chat.console.current == Some(oteryn_client_ui::chat::ChannelTab::Npc)
        && mailbox.chat.feedback.is_none()
    {
        let target = mailbox
            .dialogue
            .target
            .filter(|_| mailbox.dialogue.open && !mailbox.npc_dialog_dismissed);
        let talk = target.and_then(|npc| {
            oteryn_client_ui::npc_dialogue::request(&mut mailbox.dialogue, npc, &mailbox.chat.text)
        });
        if let Some(talk) = talk {
            super::super::feature_0233_player_typing::input::composer(mailbox, commands, false);
            if commands.send(PlayCommand::NpcTalk(talk)).is_err() {
                mailbox.dialogue.pending = false;
                mailbox.chat.feedback = Some(tr!("The session has ended").into());
            } else {
                mailbox.chat.console.sent();
            }
        } else {
            mailbox.chat.feedback = Some(tr!("Chat unavailable or waiting for the server").into());
        }
        return;
    }
    if let Some(intent) = intent {
        super::super::feature_0233_player_typing::input::composer(mailbox, commands, false);
        if commands.send(PlayCommand::ChatPublic(intent)).is_err() {
            mailbox.chat.pending = false;
            mailbox.chat.feedback = Some(tr!("The session has ended").into());
        } else {
            mailbox.chat.console.sent();
        }
    }
}

/// The native composer uses the same destination precedence as `enqueue`.
#[cfg(any(windows, test))]
pub(in crate::play) fn composer_enabled(
    panel: &oteryn_client_ui::chat::ChatPanel,
    mode: Mode,
) -> bool {
    panel.console.current.is_some()
        || mode != Mode::Guild
        || oteryn_client_ui::chat_guild_room::available(panel)
}
