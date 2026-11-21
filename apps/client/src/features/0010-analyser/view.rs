use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn analyser_text(&self) -> String {
        let mailbox = lock(&self.mailbox);
        panel_text(&mailbox, mailbox.analyser.tool)
    }
    #[cfg(bevy_shell)]
    pub(crate) fn analyser_text_for(&self, tool: oteryn_client_ui::analyser::Tool) -> String {
        panel_text(&lock(&self.mailbox), tool)
    }
    pub fn analyser_panel(&self) -> oteryn_client_ui::analyser::Panel {
        lock(&self.mailbox).analyser.clone()
    }
}
fn panel_text(
    mailbox: &crate::play::mailbox::Mailbox,
    tool: oteryn_client_ui::analyser::Tool,
) -> String {
    if mailbox.analyser.selected && tool == oteryn_client_ui::analyser::Tool::Bosses {
        oteryn_client_ui::analyser::bosses::text(
            &mailbox.analyser,
            &mailbox.boss_encounter,
            &mailbox.boss_rewards,
        )
    } else {
        let mut panel = mailbox.analyser.clone();
        panel.tool = tool;
        oteryn_client_ui::analyser::panel_text(&panel)
    }
}

/// Auxiliary windows borrow the mailbox; only the shell's PlayLink owns session teardown.
#[cfg(test)]
pub(in crate::play) fn open_shared(
    shared: &std::sync::Arc<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) -> bool {
    let ready = super::input::begin_shared_window(&mut lock(shared), commands);
    // Unit tests exercise the same mailbox transition without creating OS windows.
    ready
}
