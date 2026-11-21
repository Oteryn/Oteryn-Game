#[path = "archive.rs"]
mod archive;
pub use archive::ArchiveBindings;
pub(in crate::play) use archive::assign_cast_index;
pub(in crate::play) use archive::{
    manual as manual_archive_bindings, resolve as resolve_archive_bindings,
};
#[path = "book.rs"]
mod book;
#[cfg(test)]
#[path = "book_detail.rs"]
mod book_detail;
#[path = "preferences.rs"]
mod preferences;
#[cfg(bevy_shell)]
pub use book::{GROUPS as SPELLBOOK_GROUPS, VOCATIONS as SPELLBOOK_VOCATIONS};
pub use book::{SpellbookFilter, SpellbookState};
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    /// Current actor level from the confirmed skills snapshot.
    pub fn spellbook_actor_level(&self) -> Option<u32> {
        lock(&self.mailbox).skill_progress.complete.as_ref().map(|snapshot| snapshot.level)
    }

    #[must_use]
    pub fn spellbook_window_open(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        mailbox.spellbook_details.open
            || mailbox.spell_catalog.open
            || mailbox.spell_catalog.archive_open
    }

    /// Shared window closure retains the confirmed pages and filters.
    pub fn spellbook_set_open(&self, open: bool) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.spellbook_details.open = open;
        if !open {
            mailbox.spell_catalog.open = false;
            mailbox.spell_catalog.archive_open = false;
        }
    }
    /// Resolve the painted spell id through the current owner's filtered catalogue.
    pub fn spell_catalog_select(&self, cast_index: u32) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.spell_catalog;
        if !panel.selected || panel.filter_error {
            return false;
        }
        let Some(row) = panel
            .visible_spells()
            .position(|spell| spell.cast_index == cast_index)
        else {
            return false;
        };
        panel.selection = row;
        panel.page = row / 16;
        true
    }
    pub fn spell_catalog_navigate(&self, next: bool) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.spell_catalog;
        let filter = panel.filter.clone();
        oteryn_client_ui::spell_catalog::protocol::navigate(
            panel,
            &filter,
            if next { 2 } else { 1 },
        );
    }

    pub fn spell_catalog_panel(&self) -> oteryn_client_ui::spell_catalog::Panel {
        lock(&self.mailbox).spell_catalog.clone()
    }
}


#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_spellbook_select(&self) {
        self.spellbook_details_initialize(true);
    }
    pub(crate) fn bevy_journal_spellbook_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        self.spellbook_details_result(result);
    }
}
#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_catalogue(
        &self,
        payload: Vec<u8>,
    ) -> Result<(), oteryn_session::SessionError> {
        self.spell_catalog_initialize(true, Some(payload))
    }
}
