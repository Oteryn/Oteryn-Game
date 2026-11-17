//! The channel list dialog (Ctrl+O): selection, the private-message name box and what a
//! confirmation opens. The native window only renders this state.
use super::{ChannelEntry, Console};

/// What confirming the dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAction {
    /// A row of the list: a public room, the own private channel or an invitation.
    Entry(ChannelEntry),
    /// The name box holds a player: open a private-message tab for it.
    Conversation(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelDialog {
    pub open: bool,
    pub selected: usize,
    /// The entry box that opens a private-message channel by name.
    pub name: String,
}

impl ChannelDialog {
    pub fn show(&mut self) {
        self.open = true;
        self.selected = 0;
        self.name.clear();
    }
    pub fn hide(&mut self) {
        self.open = false;
    }
    /// Select a row; an index past the list is ignored.
    pub fn select(&mut self, index: usize, rows: usize) -> bool {
        if index >= rows {
            return false;
        }
        self.selected = index;
        true
    }
    /// Move the selection one row, wrapping around.
    pub fn step(&mut self, forward: bool, rows: usize) {
        if rows == 0 {
            return;
        }
        self.selected = if forward {
            (self.selected + 1) % rows
        } else {
            (self.selected + rows - 1) % rows
        };
    }
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }
    /// Confirm a shown dialog: a typed name wins over the selected row. The dialog closes when something opens.
    pub fn confirm(&mut self, entries: &[ChannelEntry]) -> Option<DialogAction> {
        if !self.open {
            return None;
        }
        let name = self.name.trim();
        let action = if name.is_empty() {
            DialogAction::Entry(entries.get(self.selected)?.clone())
        } else {
            DialogAction::Conversation(name.to_owned())
        };
        self.hide();
        Some(action)
    }
}

/// One caption per row, in the order of `entries`; a row that already has a tab is marked open.
#[must_use]
pub fn dialog_rows(entries: &[ChannelEntry], console: &Console) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match entry {
            ChannelEntry::CreateOwn => tr!("Create own private channel").to_owned(),
            _ => super::view::entry_text(entry, console)
                .trim_end()
                .to_owned(),
        })
        .collect()
}
