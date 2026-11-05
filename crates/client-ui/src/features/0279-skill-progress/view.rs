use super::SkillProgressPanel;

pub fn panel_text(panel: &SkillProgressPanel) -> String {
    if !panel.selected {
        return "Skill progress unavailable in this session".into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(snapshot) = panel.snapshot else {
        return "Loading skill progress…".into();
    };
    let mut text = String::from("Trained skill progress\r\n");
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
        let filled = usize::from(skill.percent / 1_000);
        let bar = format!("{}{}", "#".repeat(filled), "-".repeat(10 - filled));
        text.push_str(&format!(
            "{label}: {} [{bar}] {}.{:02}% ({}/{})\r\n",
            skill.level,
            skill.percent / 100,
            skill.percent % 100,
            skill.progress,
            skill.required
        ));
    }
    text
}
