use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn highscores_panel(&self) -> oteryn_client_ui::highscores::HighscoresPanel {
        lock(&self.mailbox).highscores.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
