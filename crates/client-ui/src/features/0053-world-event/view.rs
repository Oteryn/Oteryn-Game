use super::WorldEventPanel;
pub fn panel_text(panel: &WorldEventPanel) -> String {
    if !panel.selected {
        return tr!("World events unavailable in this session").into();
    }
    if panel.pending {
        return tr!("Waiting for the server…").into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    panel.state.as_ref().map_or_else(
        || tr!("Refresh to query world events").into(),
        |state| {
            let announcement = if state.active {
                state.announcement.as_str()
            } else {
                tr!("No active world event")
            };
            lfmt!(
                "World event\r\n{announcement}\r\nReset epoch: {}",
                state.reset_epoch,
                announcement = announcement
            )
        },
    )
}
