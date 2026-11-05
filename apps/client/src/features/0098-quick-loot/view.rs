use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn discovery_panel(&self) -> oteryn_client_ui::quick_loot::Discovery {
        lock(&self.mailbox).discovery.clone()
    }
    pub fn discovery_open(&self, panel: u32) {
        #[cfg(windows)]
        native::open_window(self, panel);
        #[cfg(not(windows))]
        {
            let mut mailbox = lock(&self.mailbox);
            match panel {
                98 => mailbox.discovery.loot.open = true,
                132 => mailbox.discovery.inspection.open = true,
                232 => mailbox.discovery.teams.open = true,
                285 => mailbox.discovery.history.open = true,
                _ => {}
            }
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;

#[cfg(windows)]
pub(in crate::play) use native_context::{available, choose, decorate};
#[cfg(windows)]
#[path = "native_context.rs"]
mod native_context;
