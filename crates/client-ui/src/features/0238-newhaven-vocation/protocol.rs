use super::VocationPanel;

/// The app encodes the selected identity through the protocol owner before sending it.
pub fn request(panel: &mut VocationPanel, vocation: u8) -> Option<u8> {
    if !panel.selected || !panel.open || panel.pending || !(1..=5).contains(&vocation) {
        return None;
    }
    panel.pending = true;
    panel.refused = false;
    Some(vocation)
}
