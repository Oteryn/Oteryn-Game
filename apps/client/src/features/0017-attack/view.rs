use crate::play::{PlayLink, mailbox::lock};
#[path = "preferences.rs"]
mod preferences;
#[cfg(test)]
pub use preferences::BattleSort;
pub(crate) use preferences::next_edit;
pub use preferences::{BattleListPreferences, BattleListsState, default_lists, validate_lists};
impl PlayLink {
    pub fn battle_panel(&self) -> oteryn_client_ui::battle::BattlePanel {
        lock(&self.mailbox).battle.clone()
    }

    /// A native list edit wins once; otherwise the saved settings drive the list windows.
    pub fn sync_battle_lists(
        &self,
        saved: &mut Vec<BattleListPreferences>,
        saved_edit: u64,
    ) -> Option<u64> {
        let mut active = lock(&self.mailbox).battle_lists.active;
        self.sync_battle_list_state(saved, &mut active, saved_edit)
    }
    pub fn battle_select_list(&self, active: usize) -> bool {
        lock(&self.mailbox).battle_lists.select(active)
    }
    pub fn active_battle_list(&self) -> usize {
        lock(&self.mailbox).battle_lists.active
    }
    pub fn sync_battle_list_state(
        &self,
        saved: &mut Vec<BattleListPreferences>,
        active: &mut usize,
        saved_edit: u64,
    ) -> Option<u64> {
        let mut mailbox = lock(&self.mailbox);
        let lists = &mut mailbox.battle_lists;
        if lists.dirty && lists.edit > saved_edit {
            saved.clone_from(&lists.preferences);
            *active = lists.active;
            lists.dirty = false;
            Some(lists.edit)
        } else {
            lists.dirty = false;
            lists.preferences.clone_from(saved);
            lists.active = (*active).min(saved.len().saturating_sub(1));
            None
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
