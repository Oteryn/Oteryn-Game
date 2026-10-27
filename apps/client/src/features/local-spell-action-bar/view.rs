use crate::play::PlayView;
use crate::spell::SPELL_SLOTS;

impl PlayView {
    /// The minimal spell bar: its hotkeys and the last combat, cast or refused logout line.
    #[must_use]
    pub fn combat_status(&self) -> String {
        let line = self
            .logout_line
            .as_deref()
            .or(self.combat_line)
            .unwrap_or("");
        format!("spells 1-{SPELL_SLOTS} {line}")
            .trim_end()
            .to_owned()
    }
}
