use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Leader advertisement is unavailable in this session".into();
    }
    format!(
        "Your advertisement\r\n{}\r\n{:?}",
        panel.feedback, panel.confirmed
    )
}
