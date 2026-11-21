use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn highscores_panel(&self) -> oteryn_client_ui::highscores::HighscoresPanel {
        lock(&self.mailbox).highscores.clone()
    }
}
