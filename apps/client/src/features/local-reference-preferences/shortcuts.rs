//! The button strip opens existing panels; it does not create gameplay commands.
use super::{PreferenceError, ReferencePreferences};
use crate::play::PlayLink;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

pub const SHORTCUTS: &[(&str, &str)] = &[
    ("battle", "Battle"),
    ("chat", "Chat"),
    ("guild", "Guild"),
    ("hireling", "Hirelings"),
    ("imbuement", "Imbuements"),
    ("inventory", "Inventory"),
    ("market", "Market"),
    ("party", "Party"),
    ("prey", "Prey"),
    ("quest-log", "Quests"),
    ("skills", "Skills"),
    ("spells", "Spells"),
    ("stash", "Stash"),
    ("task-board", "Tasks"),
    ("trade", "Trade"),
    ("vip", "Contacts"),
    ("wheel", "Wheel"),
];
pub(super) fn default_shortcuts() -> Vec<String> {
    ["battle", "chat", "inventory", "skills", "spells"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}
pub(super) fn valid(ids: &[String]) -> bool {
    ids.len() <= SHORTCUTS.len()
        && ids.iter().enumerate().all(|(i, id)| {
            SHORTCUTS.iter().any(|(known, _)| *known == id) && !ids[..i].contains(id)
        })
}
impl ReferencePreferences {
    pub fn add_shortcut(&mut self, id: &str) -> Result<(), PreferenceError> {
        if !SHORTCUTS.iter().any(|(known, _)| *known == id)
            || self.shortcuts.iter().any(|old| old == id)
        {
            return Err(PreferenceError);
        }
        self.shortcuts.push(id.to_owned());
        Ok(())
    }
    pub fn remove_shortcut(&mut self, index: usize) -> Result<(), PreferenceError> {
        if index >= self.shortcuts.len() {
            return Err(PreferenceError);
        }
        self.shortcuts.remove(index);
        Ok(())
    }
    pub fn move_shortcut(
        &mut self,
        index: usize,
        destination: usize,
    ) -> Result<(), PreferenceError> {
        if index >= self.shortcuts.len() || destination >= self.shortcuts.len() {
            return Err(PreferenceError);
        }
        let id = self.shortcuts.remove(index);
        self.shortcuts.insert(destination, id);
        Ok(())
    }
}
fn key(code: u16, modifier: Modifier, shift: bool) -> Option<NormalizedInputEvent> {
    let mut modifiers = Modifiers::one(modifier);
    if shift {
        modifiers = modifiers.with(Modifier::Shift);
    }
    Some(NormalizedInputEvent::Key {
        code: KeyCode::new(code).ok()?,
        modifiers,
        state: ButtonState::Pressed,
        repeat: false,
    })
}
impl PlayLink {
    /// Uses the same panel entry points as keyboard input, including admission checks.
    pub fn activate_shortcut(&self, id: &str) -> bool {
        let ctrl = |code, handler: fn(&PlayLink, &[NormalizedInputEvent]) -> bool| {
            key(code, Modifier::Control, false).is_some_and(|event| handler(self, &[event]))
        };
        match id {
            "battle" => key(5, Modifier::Alt, false).is_some_and(|e| self.battle_input(&[e])),
            "chat" => {
                self.open_chat();
                true
            }
            "guild" => {
                self.open_guild();
                true
            }
            "hireling" => {
                key(11, Modifier::Control, true).is_some_and(|e| self.hireling_input(&[e]))
            }
            "imbuement" => {
                self.open_imbue();
                true
            }
            "inventory" => {
                #[cfg(windows)]
                crate::play::feature_0004_item_view_move::view::open_window(self);
                #[cfg(not(windows))]
                {
                    let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
                    if !mailbox.inventory_shutdown {
                        mailbox.inventory.open = true;
                    }
                }
                true
            }
            "market" => {
                self.open_market();
                true
            }
            "party" => {
                self.open_party();
                true
            }
            "prey" => ctrl(19, Self::prey_input),
            "quest-log" => ctrl(13, Self::quest_input),
            "skills" => ctrl(14, Self::loyalty_skills_input),
            "spells" => {
                self.spell_catalog_open();
                true
            }
            "stash" => {
                self.open_stash();
                true
            }
            "task-board" => {
                self.open_task_board();
                true
            }
            "trade" => ctrl(23, Self::player_trade_input),
            "vip" => ctrl(25, Self::vip_input),
            "wheel" => ctrl(26, Self::wheel_input),
            _ => false,
        }
    }
}
// Editing the ordered shortcut strip; visibility never changes the action's key binding.
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
