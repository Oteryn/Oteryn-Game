use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn battle_panel(&self) -> oteryn_client_ui::battle::BattlePanel {
        lock(&self.mailbox).battle.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
