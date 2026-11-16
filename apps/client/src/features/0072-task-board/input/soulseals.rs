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

impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn soulseal_fight_bound(&self, expected: &oteryn_client_ui::task_board::TaskBoardPanel, race: u16) -> bool {
        let mut m = lock(&self.mailbox);
        if m.world_input_blocked() || !m.task_board.open || &m.task_board != expected || !fights::select(&mut m.task_board, race) { return false; }
        send(&mut m, &self.commands, FightIntent::EnterSelected);
        true
    }
}
