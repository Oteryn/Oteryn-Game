use super::RestStatusPanel;

pub fn request(panel: &mut RestStatusPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
