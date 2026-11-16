use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn cyclopedia_tracker_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        enqueue(&mut mailbox, &self.commands, intent);
    }
    /// Opens or closes the Bestiary (`boss` false) or Bosstiary tracker window; opening
    /// reads that tracker from the server.
    pub fn cyclopedia_tracker_widget_toggle(&self, boss: bool) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.cyclopedia_tracker;
        let open = if boss {
            panel.bosstiary_widget_open = !panel.bosstiary_widget_open;
            panel.bosstiary_widget_open
        } else {
            panel.bestiary_widget_open = !panel.bestiary_widget_open;
            panel.bestiary_widget_open
        };
        if !open || panel.pending {
            return;
        }
        panel.boss = boss;
        enqueue(&mut mailbox, &self.commands, Intent::Read);
    }
    /// Reads one tracker, or adds or removes `creature_key` on it.
    pub fn cyclopedia_tracker_change(&self, boss: bool, intent: Intent, creature_key: &str) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.cyclopedia_tracker;
        if panel.pending {
            panel.feedback = Some(tr!("Waiting for the server"));
            return;
        }
        panel.boss = boss;
        if intent != Intent::Read {
            creature_key.clone_into(&mut panel.creature_key);
        }
        enqueue(&mut mailbox, &self.commands, intent);
    }
    pub fn cyclopedia_tracker_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::cyclopedia_tracker::open_key(events) {
            return false;
        }
        lock(&self.mailbox).cyclopedia_tracker.open = true;
        true
    }
}

pub(super) fn enqueue(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.cyclopedia_tracker, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.cyclopedia_tracker.pending = false;
        mailbox.cyclopedia_tracker.feedback = Some(tr!("The session has ended"));
    }
}
