#[path = "archive.rs"]
mod archive;
pub use archive::ArchiveBindings;
pub(in crate::play) use archive::assign_cast_index;
pub(in crate::play) use archive::{
    manual as manual_archive_bindings, resolve as resolve_archive_bindings,
};
#[path = "book.rs"]
mod book;
#[path = "preferences.rs"]
mod preferences;
pub use book::{SpellbookFilter, SpellbookState};
#[cfg(windows)]
#[path = "book_native.rs"]
mod book_native;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn spell_catalog_panel(&self) -> oteryn_client_ui::spell_catalog::Panel {
        lock(&self.mailbox).spell_catalog.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Spell catalogue",
            labels: &["Search name or words"],
            buttons: &["Search", "Previous", "Next"],
            snapshot: |mailbox| {
                let panel = &mailbox.spell_catalog;
                (
                    panel.open,
                    false,
                    vec![panel.filter.clone()],
                    oteryn_client_ui::spell_catalog::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.spell_catalog.open = value,

            submit: |mailbox, _, action, values| {
                if let Some(filter) = values.first() {
                    oteryn_client_ui::spell_catalog::protocol::navigate(
                        &mut mailbox.spell_catalog,
                        filter,
                        action,
                    );
                }
            },
        },
    );
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;
