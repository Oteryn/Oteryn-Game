use crate::play::PlayView;
use crate::spell::SPELL_SLOTS;

impl PlayView {
    /// The native status projection: spell hotkeys, combat/logout feedback and server notices.
    #[must_use]
    pub fn combat_status(&self) -> String {
        let line = self
            .logout_line
            .as_deref()
            .or(self.combat_line)
            .unwrap_or("");
        let status = format!("spells 1-{SPELL_SLOTS} {line}")
            .trim_end()
            .to_owned();
        match self.server_message_status() {
            Some(message) => format!("{status} — {message}"),
            None => status,
        }
    }
}
