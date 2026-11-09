#[path = "archive.rs"]
mod archive;
pub use archive::ArchiveBindings;
pub(in crate::play) use archive::assign_cast_index;
pub(in crate::play) use archive::{
    manual as manual_archive_bindings, resolve as resolve_archive_bindings,
};
#[path = "book.rs"]
mod book;
#[cfg(any(windows, test))]
#[path = "book_detail.rs"]
mod book_detail;
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
            labels: &[
                "Search name or words",
                "Maximum level (blank: all)",
                "Cooldown group (blank: all)",
                "Learned only (0 / 1)",
            ],
            buttons: &["Search", "Previous", "Next"],
            snapshot: |mailbox| {
                let panel = &mailbox.spell_catalog;
                (
                    panel.open,
                    false,
                    vec![
                        panel.filter.clone(),
                        panel
                            .max_level
                            .map_or_else(String::new, |level| level.to_string()),
                        panel
                            .group
                            .map_or_else(String::new, |group| group.to_string()),
                        u8::from(panel.learned_only).to_string(),
                    ],
                    oteryn_client_ui::spell_catalog::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.spell_catalog.open = value,

            submit: |mailbox, _, action, values| {
                if values.len() != 4 || !matches!(values[3].as_str(), "0" | "1") {
                    mailbox.spell_catalog.filter_error = true;
                    return;
                }
                if !oteryn_client_ui::spell_catalog::protocol::set_filters(
                    &mut mailbox.spell_catalog,
                    &values[1],
                    &values[2],
                    values[3] == "1",
                ) {
                    return;
                }
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
