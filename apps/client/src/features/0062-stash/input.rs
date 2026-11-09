use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::stash::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn stash_stow_selected(
        &self,
        displayed: &[oteryn_client_runtime::depot::ItemEntry],
        row: usize,
    ) {
        Self::stash_enqueue_selected(&mut lock(&self.mailbox), &self.commands, displayed, row);
    }
    pub(in crate::play) fn stash_enqueue_selected(
        mailbox: &mut Mailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
        displayed: &[oteryn_client_runtime::depot::ItemEntry],
        row: usize,
    ) {
        if let Some(intent) =
            oteryn_client_ui::stash::carried_intent(&mailbox.stash, displayed, row)
            && let Some(command) = request(&mut mailbox.stash, intent)
            && commands.send(PlayCommand::Stash(command)).is_err()
        {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some(tr!("The session has ended").into());
        }
    }

    pub fn stash_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.stash, intent)
            && self.commands.send(PlayCommand::Stash(command)).is_err()
        {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn stash_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::stash::open_key(events) {
            return false;
        }
        self.open_stash();
        true
    }
    pub fn open_stash(&self) {
        Self::stash_open(&self.mailbox, &self.commands);
    }
    pub(in crate::play) fn stash_open(
        shared: &std::sync::Arc<std::sync::Mutex<Mailbox>>,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    ) {
        {
            let mut mailbox = lock(shared);
            if mailbox.depot_shutdown {
                return;
            }
            if let Some(command) = request(&mut mailbox.stash, Intent::Refresh)
                && commands.send(PlayCommand::Stash(command)).is_err()
            {
                mailbox.stash.pending = false;
                mailbox.stash.feedback = Some(tr!("The session has ended").into());
            }
        }
        #[cfg(windows)]
        super::view::open_window(shared, commands);
        #[cfg(not(windows))]
        {
            lock(shared).stash.open = true;
        }
    }
}
