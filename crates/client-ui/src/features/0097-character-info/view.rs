use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    lfmt!(
        "Character information\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            tr!("Waiting for the server")
        } else if !panel.selected && panel.feedback.is_empty() {
            tr!("Unavailable in this session")
        } else {
            &panel.feedback
        }
    )
}

pub fn title_rows(panel: &Panel) -> Vec<super::titles::TitleChoice> {
    super::titles::title_rows(panel)
}
