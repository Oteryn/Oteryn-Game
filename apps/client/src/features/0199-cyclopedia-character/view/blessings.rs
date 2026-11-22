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
            panel.feedback = Some(tr!("The session has ended"));
        }
    }
}
