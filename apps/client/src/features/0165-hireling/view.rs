use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::HirelingPanel;

impl PlayLink {
    #[must_use]
    pub fn hireling_panel(&self) -> HirelingPanel {
        lock(&self.mailbox).hireling.clone()
    }
}

/// Banker hireling controls for the shared frontend.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::hireling::{Intent, panel_text, request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Banker Hireling"),
        labels: &[],
        buttons: &["Summon banker", "Dismiss", "Refresh", "Map location"],
        snapshot: |mailbox| {
            let panel = &mailbox.hireling;
            (
                panel.open && !mailbox.hireling_shutdown,
                panel.pending.is_some(),
                Vec::new(),
                panel_text(panel),
            )
        },
        set_open: |mailbox, value| {
            mailbox.hireling.open = value;
            mailbox.hireling_picking &= value;
        },
        submit: |mailbox, commands, action, _| {
            if action == 3 {
                // Tibia Global's crosshair pick: the next map click next to the player.
                mailbox.hireling_picking = true;
                mailbox.hireling.feedback = Some(tr!("Select an adjacent summon location"));
                return;
            }
            let intent = match action {
                0 => Intent::Summon,
                1 => Intent::Dismiss,
                2 => Intent::Refresh,
                _ => return,
            };
            if let Some(command) = request(&mut mailbox.hireling, intent)
                && commands
                    .send(crate::combat_input::PlayCommand::Hireling(command))
                    .is_err()
            {
                mailbox.hireling.pending = None;
                mailbox.hireling.feedback = Some(tr!("The session has ended"));
            }
        },
    }
}
