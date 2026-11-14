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

impl PlayLink {
    pub fn npc_dialogue_is_visible(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        !mailbox.npc_dialog_dismissed && mailbox.dialogue.selected && mailbox.dialogue.visible()
    }
    /// Dismisses the shared presentation without changing canonical data or pending requests.
    pub fn npc_dialogue_close(&self) {
        lock(&self.mailbox).npc_dialog_dismissed = true;
    }
}
