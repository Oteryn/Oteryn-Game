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
        mailbox.dialogue.selected && mailbox.dialogue.visible()
    }
    /// Dismisses presentation while preserving an in-flight request's pending gate.
    pub fn npc_dialogue_close(&self) {
        let mut mailbox = lock(&self.mailbox);
        let dialogue = &mut mailbox.dialogue;
        dialogue.open = false;
        dialogue.target = None;
        dialogue.lines.clear();
        dialogue.feedback = None;
    }
}
