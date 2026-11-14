//! Admission for retained analyser controls; calculations and transport stay in the native owner.
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};

pub(crate) const ANALYSER_TOOLS_LABELS: [&str; 37] = [
    "Overview",
    "Hunting",
    "Experience",
    "Loot",
    "Supplies",
    "Damage",
    "Healing",
    "Impact",
    "Input",
    "Hunting session",
    "Party hunt",
    "Loot tracker",
    "Kill tracker",
    "Boss tracker",
    "Reset session",
    "Apply level goal",
    "Apply item price",
    "Resolve item info",
    "Export (replace file)",
    "Refresh party hunt",
    "Party supply price",
    "Track loot item",
    "Untrack loot item",
    "Tracked / all loot",
    "Set race kill target",
    "Remove race target",
    "Refresh hunting tasks",
    "Refresh boss cooldowns",
    "Next boss cooldowns",
    "Refresh boss drop chests",
    "Next boss drop chests",
    "Inspect / next drop items",
    "Import party summary",
    "Export party summary (replace file)",
    "Set party extra cost",
    "Exclude party member",
    "Include party member",
];
pub(crate) fn requires_confirmation(action: usize) -> bool {
    matches!(action, 14 | 18 | 22 | 25 | 32 | 33 | 35)
}
#[derive(Clone)]
pub(crate) struct AnalyserToolsSnapshot {
    pub text: String,
    pub fields: [String; 4],
    signature: String,
    session: Weak<Mutex<Mailbox>>,
    consumed: Arc<AtomicBool>,
}
impl AnalyserToolsSnapshot {
    pub fn same(&self, other: &Self) -> bool {
        self.signature == other.signature
            && self.session.ptr_eq(&other.session)
            && !self.consumed.load(Ordering::Relaxed)
    }
}
fn signature(mailbox: &Mailbox) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}",
        mailbox.analyser,
        mailbox.task_board,
        mailbox.boss_encounter,
        mailbox.boss_rewards,
        mailbox.party
    )
}
fn live(link: &PlayLink, mailbox: &Mailbox) -> bool {
    !link.commands.is_closed() && !mailbox.quest_shutdown && !mailbox.custom_hotkeys_ended
}
impl PlayLink {
    pub(crate) fn analyser_tools_snapshot(&self) -> AnalyserToolsSnapshot {
        let mailbox = lock(&self.mailbox);
        let panel = &mailbox.analyser;
        AnalyserToolsSnapshot {
            text: super::super::view::panel_text(&mailbox),
            fields: [
                panel.goal_input.clone(),
                panel.price_input[0].clone(),
                panel.price_input[1].clone(),
                panel.export_path.clone(),
            ],
            signature: signature(&mailbox),
            session: Arc::downgrade(&self.mailbox),
            consumed: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(crate) fn analyser_tools_open_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !live(self, &mailbox)
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
        {
            return false;
        }
        if !mailbox.analyser.open {
            super::resolve_items(&mut mailbox, &self.commands);
            mailbox.analyser.open = true;
        }
        true
    }
    pub(crate) fn analyser_tools_close_bound(&self, expected: &AnalyserToolsSnapshot) -> bool {
        let mut mailbox = lock(&self.mailbox);
        // Closing only affects presentation, even while input/capability/request admission is blocked.
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox)) || !live(self, &mailbox) {
            return false;
        }
        mailbox.analyser.open = false;
        true
    }
    pub(crate) fn analyser_tools_submit_bound(
        &self,
        expected: &AnalyserToolsSnapshot,
        action: usize,
        fields: [String; 4],
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || !live(self, &mailbox)
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || !mailbox.analyser.open
            || !mailbox.analyser.selected
            || expected.signature != signature(&mailbox)
            || action >= ANALYSER_TOOLS_LABELS.len()
            || (requires_confirmation(action) && !confirmed)
            || fields.iter().any(|s| s.len() > 7996 || s.contains('\0'))
            || mailbox.analyser.party_pending
            || !mailbox.analyser.catalogue_pending.is_empty()
            || mailbox.task_board.pending
            || mailbox.boss_encounter.pending
            || mailbox.boss_rewards.pending
            || expected.consumed.load(Ordering::Relaxed)
        {
            return false;
        }
        // The one-shot belongs to the displayed projection and is consumed under the owner lock.
        expected.consumed.store(true, Ordering::Relaxed);
        super::submit(&mut mailbox, &self.commands, action, fields.into());
        true
    }
}
