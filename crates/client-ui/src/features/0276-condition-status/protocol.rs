use super::ConditionStatusPanel;
pub fn request(panel: &mut ConditionStatusPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
