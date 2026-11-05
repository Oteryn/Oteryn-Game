use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn npc_dialogue_panel(&self) -> oteryn_client_ui::npc_dialogue::DialoguePanel {
        lock(&self.mailbox).dialogue.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{context_menu, open_window};
