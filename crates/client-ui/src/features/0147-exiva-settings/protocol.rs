use super::*;
/// All UI actions use the existing command 170 and its selected capability.
pub fn request(panel: &mut ExivaPanel, payload: Vec<u8>) -> Option<Vec<u8>> {
    if Request::decode(&payload).is_none() {
        panel.error = Some("Enter a valid character name");
        return None;
    }
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(payload)
}
