use super::LoyaltySkillsPanel;

/// A read can only be queued after capability negotiation and one at a time.
pub fn request(panel: &mut LoyaltySkillsPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
