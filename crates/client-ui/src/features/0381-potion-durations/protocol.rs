use super::PotionDurationsPanel;
pub fn request(panel: &mut PotionDurationsPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
