use super::WorldEventPanel;
pub fn panel_text(panel: &WorldEventPanel) -> String {
    if !panel.selected {
        return "World events unavailable in this session".into();
    }
    if panel.pending {
        return "Waiting for the server…".into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    panel.state.as_ref().map_or_else(
        || "Refresh to query world events".into(),
        |state| {
            let announcement = if state.active {
                state.announcement.as_str()
            } else {
                "No active world event"
            };
            format!(
                "World event\r\n{announcement}\r\nReset epoch: {}",
                state.reset_epoch
            )
        },
    )
}
