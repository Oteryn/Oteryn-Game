use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(p: &WeaponProficiencyPanel) -> String {
    let state = p.result.as_deref().unwrap_or("No server result yet");
    match p.feedback {
        Some(f) => format!("Weapon proficiency\n{state}\n{f}"),
        None => format!("Weapon proficiency\n{state}"),
    }
}
