//! Native movement spell controls submit the existing CHAT_V1 SAY command.
#![cfg(any(windows, test))]
use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_ui::spell_action_bar::movement::{MovementSpell, request};
use oteryn_session::{ChatIntent, ChatSpeechMode};
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    spell: MovementSpell,
) {
    if let Some(words) = request(&mut mailbox.chat_public, spell)
        && commands
            .send(PlayCommand::MovementSpeech(ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: words.into(),
            }))
            .is_err()
    {
        mailbox.chat_public.pending = false;
        mailbox.chat_public.feedback = Some(tr!("The session has ended").into());
    }
}
