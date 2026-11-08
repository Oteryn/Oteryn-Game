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
    pub fn chat_console(
        &self,
        tab: oteryn_client_ui::chat::ConsoleTab,
        mode: oteryn_client_ui::chat::ComposerMode,
        filter: oteryn_client_ui::chat::ConsoleFilter,
    ) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat.console.tab = tab;
        mailbox.chat.console.mode = mode;
        *mailbox.chat.console.filter_mut() = filter;
    }
    pub fn chat_request(&self, mode: Mode, recipient: &str) {
        enqueue(&mut lock(&self.mailbox), &self.commands, mode, recipient);
    }
    pub fn chat_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::chat::open_key(events) {
            return false;
        }
        self.open_chat();
        true
    }
    pub fn open_chat(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
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
    if let Some(intent) = request(&mut mailbox.chat, mode, recipient) {
        super::super::feature_0233_player_typing::input::composer(mailbox, commands, false);
        if commands.send(PlayCommand::ChatPublic(intent)).is_err() {
            mailbox.chat.pending = false;
            mailbox.chat.feedback = Some("The session has ended".into());
        }
    }
}
