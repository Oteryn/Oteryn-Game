use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::cyclopedia_character::{BlessingsPanel, request_blessings};
impl PlayLink {
    pub fn blessings_panel(&self) -> BlessingsPanel {
        lock(&self.mailbox).cyclopedia_character.blessings.clone()
    }
    pub fn blessings_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request_blessings(&mut mailbox.cyclopedia_character)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            let panel = &mut mailbox.cyclopedia_character.blessings;
            panel.pending = false;
            panel.feedback = Some("The session has ended");
        }
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    if link.blessings_panel().open {
        return;
    }
    link.blessings_request();
    open(
        link,
        Form {
            title: "Blessings / death protection",
            labels: &[],
            buttons: &["Refresh", "Close"],
            snapshot: |mailbox| {
                let panel = &mailbox.cyclopedia_character.blessings;
                (
                    panel.open && !mailbox.cyclopedia_character_shutdown,
                    panel.pending,
                    vec![],
                    oteryn_client_ui::cyclopedia_character::blessings_text(
                        &mailbox.cyclopedia_character,
                    ),
                )
            },
            set_open: |mailbox, open| mailbox.cyclopedia_character.blessings.open = open,
            submit: |mailbox, commands, action, _| {
                if action == 1 {
                    mailbox.cyclopedia_character.blessings.open = false;
                    return;
                }
                if action != 0 {
                    return;
                }
                if let Some(command) = request_blessings(&mut mailbox.cyclopedia_character)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    let panel = &mut mailbox.cyclopedia_character.blessings;
                    panel.pending = false;
                    panel.feedback = Some("The session has ended");
                }
            },
        },
    );
}
