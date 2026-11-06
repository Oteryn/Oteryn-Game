use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(p: &WeaponProficiencyPanel) -> String {
    let mut state = p
        .result
        .clone()
        .unwrap_or_else(|| "No server result yet".to_owned());
    if let Some(page) = &p.catalogue {
        use std::fmt::Write;
        let _ = writeln!(
            state,
            "\nWeapons {}-{} of {}",
            page.offset.saturating_add(1).min(page.total),
            page.offset.saturating_add(page.entries.len() as u32),
            page.total
        );
        for entry in &page.entries {
            let _ = writeln!(
                state,
                "{} (weapon id {}): {} XP",
                entry.name, entry.definition, entry.experience
            );
            let _ = writeln!(state, "Level thresholds: {:?}", entry.thresholds);
            for (level, data) in entry.levels.iter().enumerate() {
                let _ = writeln!(
                    state,
                    "  Level {} (selected {:?})",
                    level + 1,
                    data.selected
                );
                for (index, perk) in data.perks.iter().enumerate() {
                    let _ = writeln!(state, "    {index}: {perk}");
                }
            }
        }
    }
    match p.feedback {
        Some(f) => format!("Weapon proficiency\n{state}\n{f}"),
        None => format!("Weapon proficiency\n{state}"),
    }
}
