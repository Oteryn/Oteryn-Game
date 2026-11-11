//! Capability 497: the window of the own sanction record, opened from the character window.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::sanction_record::{SanctionsPanel, request};
impl PlayLink {
    pub fn sanctions_panel(&self) -> SanctionsPanel {
        lock(&self.mailbox).sanctions.clone()
    }
    /// Asks the server for the own record; nothing is sent when the capability is not selected.
    pub fn sanctions_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.sanctions)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.sanctions.pending = false;
            mailbox.sanctions.feedback = Some(tr!("The session has ended"));
        }
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    if link.sanctions_panel().open {
        return;
    }
    link.sanctions_request();
    open(
        link,
        Form {
            title: "Rule violation record",
            labels: &[],
            buttons: &["Refresh", "Close"],
            snapshot: |mailbox| {
                let panel = &mailbox.sanctions;
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
                (
                    panel.open && !mailbox.cyclopedia_character_shutdown,
                    panel.pending,
                    vec![],
                    oteryn_client_ui::sanction_record::panel_text(panel, now_ms),
                )
            },
            set_open: |mailbox, open| mailbox.sanctions.open = open,
            submit: |mailbox, commands, action, _| {
                if action == 1 {
                    mailbox.sanctions.open = false;
                    return;
                }
                if action != 0 {
                    return;
                }
                if let Some(command) = request(&mut mailbox.sanctions)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    mailbox.sanctions.pending = false;
                    mailbox.sanctions.feedback = Some(tr!("The session has ended"));
                }
            },
        },
    );
}
