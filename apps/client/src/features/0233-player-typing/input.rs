use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::player_typing::request;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn player_typing_request(&self, typing: bool) {
        enqueue(&mut lock(&self.mailbox), &self.commands, typing);
    }
    pub fn player_typing_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::player_typing::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).player_typing.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    typing: bool,
) {
    if let Some(command) = request(&mut mailbox.player_typing, typing)
        && commands.send(PlayCommand::PlayerTyping(command)).is_err()
    {
        mailbox.player_typing.pending = false;
        mailbox.player_typing.feedback = Some(tr!("The session has ended"));
    }
}

impl PlayLink {
    /// Composer state is also exposed to synthetic clients through the production path.
    pub fn chat_composer(&self, focused: bool, text: String) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat.text = text;
        composer(&mut mailbox, &self.commands, focused);
    }
}

pub(in crate::play) fn composer(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    focused: bool,
) {
    if let Some(command) = oteryn_client_ui::player_typing::composer(
        &mut mailbox.player_typing,
        focused,
        &mailbox.chat.text,
        std::time::Instant::now(),
    ) {
        let _ = commands.send(PlayCommand::PlayerTyping(command));
    }
}
