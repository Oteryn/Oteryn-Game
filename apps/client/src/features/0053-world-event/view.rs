use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::world_event::WorldEventPanel;
impl PlayLink {
    pub fn world_event_panel(&self) -> WorldEventPanel {
        lock(&self.mailbox).world_event.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// World event schedule for the shared frontend; Refresh asks the server again.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("World events — Ctrl+N"),
        labels: &[],
        buttons: &["Refresh"],
        snapshot: |mailbox| {
            let panel = &mailbox.world_event;
            (
                panel.open && !mailbox.world_event_shutdown,
                panel.pending,
                Vec::new(),
                oteryn_client_ui::world_event::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.world_event.open = value,
        submit: |mailbox, commands, _, _| {
            if let Some(command) = oteryn_client_ui::world_event::request(&mut mailbox.world_event)
                && commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
            {
                mailbox.world_event.pending = false;
                mailbox.world_event.feedback = Some(tr!("The session has ended"));
            }
        },
    }
}
