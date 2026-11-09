use super::Panel;
pub fn request(panel: &mut Panel, payload: Vec<u8>) -> bool {
    if !panel.selected {
        panel.feedback = tr!("Unavailable in this session").into();
        return false;
    }
    if panel.pending {
        panel.feedback = tr!("Waiting for the server").into();
        return false;
    }
    panel.pending = true;
    panel.pending_payload = payload;
    panel.feedback.clear();
    true
}
