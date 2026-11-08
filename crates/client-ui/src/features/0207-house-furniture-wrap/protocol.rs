use super::Panel;
pub fn request(panel: &mut Panel, payload: Vec<u8>) -> bool {
    if !panel.selected {
        panel.feedback = "Unavailable in this session".into();
        return false;
    }
    if panel.pending {
        panel.feedback = "Waiting for the server".into();
        return false;
    }
    if let Some(action) = oteryn_client_runtime::raw_feature::house_wrap::Request::decode(&payload)
        && panel.reviewed_payload.as_ref() != Some(&payload)
    {
        panel.feedback = format!(
            "Review {:?} for item {:02x?} at {}, {}, floor {}. Press Wrap / place again to confirm. Changing the item or destination requires another review.",
            action.action, action.item, action.position[0], action.position[1], action.position[2]
        );
        panel.reviewed_payload = Some(payload);
        return false;
    }
    panel.reviewed_payload = None;
    panel.pending = true;
    panel.pending_payload = payload;
    panel.feedback.clear();
    true
}
