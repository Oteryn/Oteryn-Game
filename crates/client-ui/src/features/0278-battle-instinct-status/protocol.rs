use super::BattleInstinctStatusPanel;

pub fn request(panel: &mut BattleInstinctStatusPanel) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(Vec::new())
}
