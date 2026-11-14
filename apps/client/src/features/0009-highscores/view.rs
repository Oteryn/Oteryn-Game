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

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bevy_bound.rs"]
mod bevy_bound;

#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
mod bound;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_uni16_highscores_initialize(&self) {
        self.highscores_initialize(true, false);
    }
    pub(crate) fn bevy_uni16_highscores_reply(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.highscores_result(response);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_uni16_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
