//! The native dialog and public PlayLink share this command queue caller.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::task_board::fights::{self, FightIntent};
use tokio::sync::mpsc::UnboundedSender;
pub(in crate::play) fn send(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: FightIntent,
) {
    if let Some(command) = fights::request(&mut m.task_board, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        m.task_board.fights.pending = None;
        m.task_board.fights.feedback = "The session has ended".into();
    }
}
impl PlayLink {
    pub fn soulseal_catalogue_request(&self, after: u16) {
        send(
            &mut lock(&self.mailbox),
            &self.commands,
            FightIntent::Catalogue { after },
        );
    }
    pub fn soulseal_select(&self, race: u16) -> bool {
        fights::select(&mut lock(&self.mailbox).task_board, race)
    }
    pub fn soulseal_fight_selected(&self) {
        send(
            &mut lock(&self.mailbox),
            &self.commands,
            FightIntent::EnterSelected,
        );
    }
}
