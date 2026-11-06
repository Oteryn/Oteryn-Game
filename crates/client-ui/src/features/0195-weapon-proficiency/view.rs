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
    let mut catalogue = String::new();
    if let Some(page) = &p.page {
        catalogue.push_str(&format!(
            "Weapons {}–{} / {}\n",
            if page.rows.is_empty() {
                0
            } else {
                page.offset + 1
            },
            page.offset + page.rows.len() as u32,
            page.total
        ));
        for row in &page.rows {
            catalogue.push_str(&format!(
                "{} [{}]: {} XP, level {}, perks {:?}\n",
                row.name, row.definition, row.experience, row.level, row.selections
            ));
        }
    }
    catalogue.push_str(&state);
    let state = catalogue.as_str();
    match p.feedback {
        Some(f) => format!("Weapon proficiency\n{state}\n{f}"),
        None => format!("Weapon proficiency\n{state}"),
    }
}
