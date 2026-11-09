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
    let mut text = panel.complete.map_or_else(|| String::from("Trained skill progress\r\n"), |full| format!(
        "Level: {} — Experience: {} — {}.{:02}% to next level\r\nLoyalty skill-point bonus: {}%\r\nSkills: trained / with Loyalty / current\r\n",
        full.level, full.experience, full.level_percent / 100, full.level_percent % 100, full.loyalty_bonus_percent,
    ));
    for family in panel.ordered_families() {
        let label = super::SKILL_LABELS[family];
        let skill = snapshot.skills[family];
        if panel.hidden_skills & (1 << family) != 0 {
            continue;
        }
        let filled = usize::from(skill.percent / 1_000);
        let bar = format!("{}{}", "#".repeat(filled), "-".repeat(10 - filled));
        let levels = panel.complete.map_or_else(
            || skill.level.to_string(),
            |full| {
                format!(
                    "{} / {} / {}",
                    skill.level,
                    full.loyalty_levels[family],
                    full.current_levels.map_or_else(
                        || "unavailable".to_owned(),
                        |values| values[family].to_string()
                    )
                )
            },
        );
        text.push_str(&format!(
            "{label}: {levels} [{bar}] {}.{:02}% ({}/{})\r\n",
            skill.percent / 100,
            skill.percent % 100,
            skill.progress,
            skill.required
        ));
    }
    text
}
