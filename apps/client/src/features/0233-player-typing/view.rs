use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_typing::PlayerTypingPanel;
impl PlayLink {
    #[must_use]
    pub fn player_typing_panel(&self) -> PlayerTypingPanel {
        lock(&self.mailbox).player_typing.clone()
    }
}
