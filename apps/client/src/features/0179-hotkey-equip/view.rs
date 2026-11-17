#[path = "preferences.rs"]
mod preferences;
use crate::play::{PlayLink, mailbox::lock};
pub use preferences::ItemHotkeys;
pub(super) use preferences::edited;
pub(crate) use preferences::supported_capabilities;
impl PlayLink {
    #[must_use]
    pub fn hotkey_equip_panel(&self) -> oteryn_client_ui::hotkey_equip::HotkeyEquipPanel {
        lock(&self.mailbox).hotkey_equip.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
