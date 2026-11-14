//! Atomic frontend bindings; the retained panel and request helpers remain authoritative.
use super::native;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel as Panel, request};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Close,
    Track(bool),
    Weapon(usize),
    Choose(usize, usize),
    Search(String),
    Filter(u8),
    Sort(bool),
    ClearPerk(bool),
    Query(usize),
    Confirmed(usize),
}
impl PlayLink {
    /// Identity is tied to the actual mailbox, including replacement within a host generation.
    pub(crate) fn weapon_proficiency_snapshot(&self) -> (usize, Panel, String) {
        let mailbox = lock(&self.mailbox);
        let panel = &mailbox.weapon_proficiency;
        (
            std::sync::Arc::as_ptr(&self.mailbox) as usize,
            panel.clone(),
            native::display(panel).replace("\r\n", "\n"),
        )
    }
    pub(crate) fn weapon_proficiency_dispatch_bound(
        &self,
        session: usize,
        expected: &Panel,
        action: Action,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if session != std::sync::Arc::as_ptr(&self.mailbox) as usize
            || !mailbox.shared_frontend
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.weapon_proficiency != *expected
            || !expected.selected
        {
            return false;
        }
        if action == Action::Close {
            if !expected.open {
                return false;
            }
            mailbox.weapon_proficiency.open = false;
            return true;
        }
        if expected.pending
            || (!expected.open && !matches!(action, Action::Open | Action::Track(_)))
        {
            return false;
        }
        let mut next = expected.clone();
        let command = match action {
            Action::Open => {
                next.open = true;
                next.catalogue_offset = 0;
                request(&mut next, Intent::Catalogue)
            }
            Action::Track(advancement) => {
                let definition = if advancement {
                    next.advancements.first().copied()
                } else {
                    next.progress_entry().map(|entry| entry.definition)
                };
                let Some(row) = definition.and_then(|id| {
                    next.catalogue
                        .as_ref()?
                        .entries
                        .iter()
                        .position(|entry| entry.definition == id)
                }) else {
                    return false;
                };
                if !next.select_weapon(row) {
                    return false;
                }
                next.advancements.retain(|id| *id != next.weapon);
                next.open = true;
                request(&mut next, Intent::State)
            }
            Action::Weapon(row) => {
                if !native::choose(&mut next, 0, row) {
                    return false;
                }
                request(&mut next, Intent::State)
            }
            Action::Choose(field, row) => {
                if !native::choose(&mut next, field, row) {
                    return false;
                }
                None
            }
            Action::Search(value) => {
                next.search = value;
                None
            }
            Action::Filter(value) if value <= 3 => {
                next.progress_filter = value;
                None
            }
            Action::Sort(value) => {
                next.sort_progress = value;
                None
            }
            Action::ClearPerk(value) => {
                next.clear_perk = value;
                None
            }
            Action::Query(id) if matches!(id, 109..=114 | 116) => {
                native::visual_submit(&mut next, id, false)
            }
            Action::Confirmed(id) if matches!(id, 101..=108) && native::action_clicked(id, 0) => {
                let clear = next.clear_perk;
                if id == 108 {
                    let _ = native::visual_submit(&mut next, 108, clear);
                    native::visual_submit(&mut next, 117, clear)
                } else {
                    native::visual_submit(&mut next, id, clear)
                }
            }
            Action::Close | Action::Filter(_) | Action::Query(_) | Action::Confirmed(_) => {
                return false;
            }
        };
        if let Some(command) = command
            && self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
        {
            return false;
        }
        let changed = next != *expected;
        mailbox.weapon_proficiency = next;
        changed
    }
    /// Automatic polling uses the same atomic fence and canonical scheduling helper.
    pub(crate) fn weapon_proficiency_tick_bound(
        &self,
        session: usize,
        expected: &Panel,
        now: std::time::Instant,
    ) {
        let mut mailbox = lock(&self.mailbox);
        if session != std::sync::Arc::as_ptr(&self.mailbox) as usize
            || !mailbox.shared_frontend
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.weapon_proficiency != *expected
        {
            return;
        }
        let mut next = expected.clone();
        if next.open
            && !next.pending
            && next.weapon == 0
            && next
                .catalogue
                .as_ref()
                .is_some_and(|page| !page.entries.is_empty())
        {
            next.select_weapon(0);
        }
        if let Some(command) = oteryn_client_ui::weapon_proficiency::tick(&mut next, now)
            && self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
        {
            return;
        }
        mailbox.weapon_proficiency = next;
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_weapon_proficiency_initialize(&self) {
        self.weapon_proficiency_initialize(true);
        self.weapon_proficiency_state_initialize(true);
    }
    pub(crate) fn bevy_weapon_proficiency_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.weapon_proficiency_result(result);
    }
    pub(crate) fn bevy_weapon_proficiency_progress(&self, definition: u32, experience: u32) {
        lock(&self.0)
            .weapon_proficiency
            .observe_progress(definition, experience);
    }
    pub(crate) fn bevy_weapon_proficiency_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(test)]
pub(crate) fn native_request(
    panel: &mut Panel,
    id: usize,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    native::visual_submit(panel, id, panel.clear_perk)
}

#[cfg(test)]
#[path = "tests/bevy_bound.rs"]
mod tests;
