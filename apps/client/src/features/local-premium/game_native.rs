//! Real in-game Premium window using the shared native form owner.
use crate::play::PlayLink;
pub(in crate::play) fn open(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Oteryn Premium overview",
            labels: &[],
            buttons: &["Refresh Premium status"],
            snapshot: |mailbox| {
                let p = &mailbox.premium_game;
                (p.open, p.pending, Vec::new(), p.view.status_text())
            },
            set_open: |mailbox, open| mailbox.premium_game.open = open,
            submit: |mailbox, commands, _, _| {
                if let Some(command) = mailbox.premium_game.request()
                    && commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .is_err()
                {
                    mailbox.premium_game.fail("The session has ended.");
                }
            },
        },
    );
}
