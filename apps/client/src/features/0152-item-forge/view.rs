#[cfg(bevy_shell)]
mod bevy;
#[cfg(bevy_shell)]
pub(crate) use bevy::{ForgeControl, forge_action_enabled};
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn item_forge_panel(&self) -> oteryn_client_ui::item_forge::ItemForgePanel {
        lock(&self.mailbox).item_forge.clone()
    }
}
#[cfg(test)]
#[path = "native.rs"]
pub(in crate::play) mod native;

