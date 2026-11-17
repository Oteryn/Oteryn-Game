//! Equipment shares the inventory window and the production cross-window drag bridge.
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::Slots;

impl crate::play::PlayLink {
    #[must_use]
    pub fn equipment_slots(&self) -> Vec<oteryn_client_ui::equipment::Slot> {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        oteryn_client_ui::equipment::protocol::project(
            mailbox.inventory.inventory.as_ref(),
            mailbox.equipment_selected,
        )
    }
}
