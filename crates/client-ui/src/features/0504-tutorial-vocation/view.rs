use super::VocationPanel;
use oteryn_client_runtime::tutorial_vocation as wire;
pub fn choices(panel: &VocationPanel) -> Vec<(u8, &'static str)> {
    if !panel.observation.is_some_and(|o| o.offered) {
        return Vec::new();
    }
    wire::TITLES
        .iter()
        .enumerate()
        .map(|(i, title)| ((i + 1) as u8, *title))
        .collect()
}
