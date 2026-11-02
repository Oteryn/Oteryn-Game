use super::LoyaltySkillsPanel;

pub fn panel_text(panel: &LoyaltySkillsPanel) -> String {
    if !panel.selected {
        return "Skills unavailable in this session".into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(snapshot) = &panel.snapshot else {
        return "Loading skills…".into();
    };
    let mut text = format!(
        "Loyalty: {} points · {}% skill-point bonus\r\n{}\r\n\r\nSkills: base / with Loyalty\r\n",
        snapshot.points,
        snapshot.bonus_percent,
        snapshot.title.as_deref().unwrap_or("No loyalty title"),
    );
    for (label, skill) in [
        "Magic",
        "Fist",
        "Club",
        "Sword",
        "Axe",
        "Distance",
        "Shielding",
        "Fishing",
    ]
    .into_iter()
    .zip(snapshot.skills)
    {
        text.push_str(&format!(
            "{label}: {} / {}\r\n",
            skill.base, skill.with_loyalty
        ));
    }
    text
}
