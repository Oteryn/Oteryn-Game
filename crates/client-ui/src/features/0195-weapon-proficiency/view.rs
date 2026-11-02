use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(panel: &WeaponProficiencyPanel) -> String {
    if !panel.selected {
        return "Unavailable".into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    match panel.last_command {
        Some(command) => format!("Command {command}: {} response bytes", panel.result.len()),
        None => "Ready".into(),
    }
}
