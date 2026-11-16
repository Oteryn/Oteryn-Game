use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::{Intent, TrackerSort, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn cyclopedia_tracker_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        enqueue(&mut mailbox, &self.commands, intent);
    }
    /// Opens or closes the Bestiary (`boss` false) or Bosstiary tracker window; opening
    /// reads that tracker from the server, after the other tracker's request in flight.
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
        if open {
            send_or_queue(&mut mailbox, &self.commands, boss, Intent::Read, "");
        }
    }
    /// Reads one tracker, or adds or removes `creature_key` on it. A request on one tracker
    /// waits for the other tracker's request in flight; each tracker keeps its own feedback.
    pub fn cyclopedia_tracker_change(&self, boss: bool, intent: Intent, creature_key: &str) {
        let mut mailbox = lock(&self.mailbox);
        send_or_queue(&mut mailbox, &self.commands, boss, intent, creature_key);
    }
    /// Sends the request a tracker queued behind the other tracker's, once that one answered.
    pub fn cyclopedia_tracker_flush(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.cyclopedia_tracker;
        if panel.pending {
            return;
        }
        // The window that did not ask last goes first, so neither waits forever.
        let first = usize::from(!panel.boss);
        let Some((slot, (intent, key))) = [first, 1 - first]
            .into_iter()
            .find_map(|slot| panel.queued[slot].take().map(|queued| (slot, queued)))
        else {
            return;
        };
        send(&mut mailbox, &self.commands, slot == 1, intent, &key);
    }
    /// The row order and whether completed rows show, for one tracker window.
    pub fn cyclopedia_tracker_configure(&self, boss: bool, sort: TrackerSort, hide_completed: bool) {
        let panel = &mut lock(&self.mailbox).cyclopedia_tracker;
        if boss {
            panel.boss_sort = sort;
            panel.boss_hide_completed = hide_completed;
        } else {
            panel.sort = sort;
            panel.hide_completed = hide_completed;
        }
    }
    pub fn cyclopedia_tracker_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::cyclopedia_tracker::open_key(events) {
            return false;
        }
        lock(&self.mailbox).cyclopedia_tracker.open = true;
        true
    }
}

fn send_or_queue(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    boss: bool,
    intent: Intent,
    creature_key: &str,
) {
    let panel = &mut mailbox.cyclopedia_tracker;
    let slot = usize::from(boss);
    if panel.pending {
        if panel.boss == boss || panel.queued[slot].is_some() {
            // An open while this tracker is already being read needs no second read.
            if intent != Intent::Read {
                panel.widget_feedback[slot] = Some(tr!("Waiting for the server"));
            }
            return;
        }
        panel.queued[slot] = Some((intent, creature_key.to_owned()));
        return;
    }
    send(mailbox, commands, boss, intent, creature_key);
}

fn send(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    boss: bool,
    intent: Intent,
    creature_key: &str,
) {
    let panel = &mut mailbox.cyclopedia_tracker;
    panel.boss = boss;
    if intent != Intent::Read {
        creature_key.clone_into(&mut panel.creature_key);
    }
    enqueue(mailbox, commands, intent);
    let panel = &mut mailbox.cyclopedia_tracker;
    panel.widget_feedback[usize::from(boss)] = panel.feedback;
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
