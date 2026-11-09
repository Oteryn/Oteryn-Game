use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return tr!("Leader advertisement is unavailable in this session").into();
    }
    lfmt!(
        "Your advertisement\r\n{}\r\n{:?}",
        panel.feedback,
        panel.confirmed
    )
}
