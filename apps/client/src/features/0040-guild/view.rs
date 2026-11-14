use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guild_panel(&self) -> oteryn_client_ui::guild::GuildPanel {
        lock(&self.mailbox).guild.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
#[path = "view/bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{GuildWindowAction, GuildWindowView, needs_confirmation, permitted};
