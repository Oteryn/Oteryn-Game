use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::WheelPanel;

impl PlayLink {
    #[must_use]
    pub fn wheel_panel(&self) -> WheelPanel {
        lock(&self.mailbox).wheel.clone()
    }
}


#[path = "presets.rs"]
pub(in crate::play) mod presets;

#[cfg(test)]
#[path = "gem_surface.rs"]
pub(in crate::play) mod gem_surface;

#[cfg(test)]
#[path = "wheel_surface.rs"]
pub(in crate::play) mod wheel_surface;

#[cfg(test)]
pub(super) fn vessel_choices(panel: &WheelPanel) -> impl ExactSizeIterator<Item = String> {
    (0..panel.vessel_slot_count()).map(oteryn_client_ui::wheel_allocation::vessel_name)
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    #[test]
    fn vessel_controls_follow_capability_selection() {
        let (link, _, outbox) = crate::play::play_channel();
        for atelier_v2 in [false, true, false] {
            outbox.wheel_initialize(true, atelier_v2);
            let panel = link.wheel_panel();
            let choices = vessel_choices(&panel);
            assert_eq!(choices.len(), usize::from(panel.vessel_slot_count()));
            for slot in 0..panel.vessel_slot_count() {
                super::super::input::gem_presentation::activate(
                    &mut lock(&link.mailbox),
                    &link.commands,
                    super::super::input::gem_presentation::Action::Vessel(slot),
                );
                assert_eq!(link.wheel_panel().vessel_slot, slot);
            }
            assert_eq!(
                choices.last(),
                Some(oteryn_client_ui::wheel_allocation::vessel_name(
                    panel.vessel_slot_count() - 1
                ))
            );
        }
    }
}
