//! Editing the ordered shortcut strip; visibility never changes the action's key binding.
use super::{SettingsState, WINDOW_ACTIONS};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutEdit {
    Add,
    Remove,
    Up,
    Down,
}
impl SettingsState {
    pub fn shortcut_choices(&self, displayed: bool) -> Vec<(&'static str, &'static str)> {
        if displayed {
            self.preferences
                .shortcut_order
                .iter()
                .filter_map(|id| WINDOW_ACTIONS.iter().find(|(key, _)| *key == id).copied())
                .collect()
        } else {
            WINDOW_ACTIONS
                .iter()
                .filter(|(id, _)| !self.preferences.shortcut_order.iter().any(|key| key == id))
                .copied()
                .collect()
        }
    }
    pub fn edit_shortcut(&mut self, id: &str, edit: ShortcutEdit) -> bool {
        if !WINDOW_ACTIONS.iter().any(|(key, _)| *key == id) {
            self.feedback = "Choose a registered shortcut".into();
            return false;
        }
        let mut prefs = self.preferences.clone();
        let position = prefs.shortcut_order.iter().position(|key| key == id);
        match (edit, position) {
            (ShortcutEdit::Add, None) => prefs.shortcut_order.push(id.into()),
            (ShortcutEdit::Remove, Some(i)) => {
                prefs.shortcut_order.remove(i);
            }
            (ShortcutEdit::Up, Some(i)) if i > 0 => prefs.shortcut_order.swap(i, i - 1),
            (ShortcutEdit::Down, Some(i)) if i + 1 < prefs.shortcut_order.len() => {
                prefs.shortcut_order.swap(i, i + 1)
            }
            _ => return false,
        }
        self.apply(prefs);
        true
    }
}
