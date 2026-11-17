use super::LoyaltySkillsPanel;

pub fn panel_text(panel: &LoyaltySkillsPanel) -> String {
    if !panel.selected {
        return tr!("Skills unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(snapshot) = &panel.snapshot else {
        return tr!("Loading skills…").into();
    };
    let mut text = lfmt!(
        "Loyalty: {} points · {}% skill-point bonus\r\n{}\r\n\r\nSkills: base / with Loyalty\r\n",
        snapshot.points,
        snapshot.bonus_percent,
        snapshot.title.as_deref().unwrap_or(tr!("No loyalty title")),
    );
    for (label, skill) in [
        tr!("Magic"),
        tr!("Fist"),
        tr!("Club"),
        tr!("Sword"),
        tr!("Axe"),
        tr!("Distance"),
        tr!("Shielding"),
        tr!("Fishing"),
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
