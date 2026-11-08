use super::Panel;
const PAGE_SIZE: usize = 16;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Spell catalogue unavailable in this session".into();
    }
    if panel.filter_error {
        return "Enter a positive maximum level and cooldown group, or leave blank".into();
    }
    let entries: Vec<_> = panel
        .catalog
        .entries()
        .iter()
        .filter(|entry| panel.matches(entry))
        .collect();
    let pages = entries.len().div_ceil(PAGE_SIZE).max(1);
    let page = panel.page.min(pages - 1);
    let mut text = format!(
        "Spells: {} | Page {}/{}\r\nCast indices can be assigned in the spell action bar.\r\n",
        entries.len(),
        page + 1,
        pages
    );
    if let Some(selected) = panel.selected_spell() {
        text.push_str(&format!(
            "Selected: {} (cast {}) — Assign to a slot with the buttons below.\r\n",
            selected.name, selected.cast_index
        ));
    }
    for entry in entries.iter().skip(page * PAGE_SIZE).take(PAGE_SIZE) {
        use std::fmt::Write;
        let _ = write!(
            text,
            "{}: {} | {} | Level {} | Mana {}{} | Soul {} | Learned {} | Available {} | Groups {:?} | Rune {:?}\r\n",
            entry.cast_index,
            entry.name,
            entry.words,
            entry.level,
            entry.mana,
            if entry.mana_percent { "%" } else { "" },
            entry.soul,
            entry.learned,
            entry.available,
            entry.cooldown_groups,
            entry.rune_item
        );
    }
    text
}
