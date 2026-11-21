use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn animus_panel(&self) -> oteryn_client_ui::animus_mastery::AnimusPanel {
        lock(&self.mailbox).animus.clone()
    }
}


/// Animus Mastery pages for the shared frontend.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::animus_mastery::{Intent, panel_text, request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Animus Mastery — Ctrl+A"),
        labels: &[],
        buttons: &["Refresh", "Previous", "Next"],
        snapshot: |mailbox| {
            let panel = &mailbox.animus;
            (
                panel.open && !mailbox.quest_shutdown,
                panel.pending.is_some(),
                Vec::new(),
                panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.animus.open = value,
        submit: |mailbox, commands, action, _| {
            let intent = match action {
                0 => Intent::Refresh,
                1 => Intent::Previous,
                2 => Intent::Next,
                _ => return,
            };
            if let Some(payload) = request(&mut mailbox.animus, intent)
                && commands
                    .send(crate::combat_input::PlayCommand::Animus(payload))
                    .is_err()
            {
                mailbox.animus.pending = None;
                mailbox.animus.error = Some(tr!("Session ended"));
            }
        },
    }
}
