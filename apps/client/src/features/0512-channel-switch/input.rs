//! chsw (capability 512): the Change Channel window's actions.
use super::protocol::now_ms;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::channel_switch;

impl PlayLink {
    /// Opens the window and asks the server for the locks and the cooldown.
    pub fn channel_switch_open(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.channel_switch;
        panel.open = true;
        panel.confirming = None;
        if let Some(command) = channel_switch::request_status(panel)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            panel.pending = false;
            panel.error = Some(tr!("The session has ended").to_owned());
        }
        // The online column; a lost query leaves it at "—".
        if let Some(command) = channel_switch::request_load(panel) {
            let _ = self.commands.send(PlayCommand::Feature(command));
        }
    }

    /// A click on a Channel row: the confirmation, or the error of the lock in the way.
    pub fn channel_switch_select(&self, channel: [u8; 16]) {
        channel_switch::select(&mut lock(&self.mailbox).channel_switch, channel, now_ms());
    }

    /// The confirmed switch request.
    pub fn channel_switch_confirm(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.channel_switch;
        let Some(command) = channel_switch::confirm(panel) else {
            return false;
        };
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            panel.pending = false;
            panel.switching = None;
            panel.error = Some(tr!("The session has ended").to_owned());
            return false;
        }
        true
    }

    /// Back from a confirmation or an error dialog to the Channel list.
    pub fn channel_switch_back(&self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.channel_switch.confirming = None;
        mailbox.channel_switch.error = None;
    }

    pub fn channel_switch_close(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.channel_switch;
        panel.open = false;
        panel.confirming = None;
        panel.error = None;
    }
}
