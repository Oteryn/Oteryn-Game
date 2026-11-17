use super::PartyEffectsPanel;
pub fn request(panel: &mut PartyEffectsPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
