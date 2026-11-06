use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(p: &WeaponProficiencyPanel) -> String {
    let mut catalogue = String::new();
    if let Some(page) = &p.page {
        catalogue.push_str(&format!(
            "Weapons {}–{} / {}\r\n",
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
                "{} [{}]: {} XP, level {}, perks {:?}\r\n",
                row.name, row.definition, row.experience, row.level, row.selections
            ));
        }
    }
    catalogue.push_str(p.result.as_deref().unwrap_or(""));
    let state = catalogue.as_str();
    match p.feedback {
        Some(f) => format!("Weapon proficiency\n{state}\n{f}"),
        None => format!("Weapon proficiency\n{state}"),
    }
}
