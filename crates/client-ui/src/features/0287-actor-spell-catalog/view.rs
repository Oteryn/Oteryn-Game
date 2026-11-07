use super::Panel;
const PAGE_SIZE: usize = 16;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Spell catalogue unavailable in this session".into();
    }
    let filter = panel.filter.to_lowercase();
    let entries: Vec<_> = panel
        .catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&filter)
                || entry.words.to_lowercase().contains(&filter)
        })
        .collect();
    let pages = entries.len().div_ceil(PAGE_SIZE).max(1);
    let page = panel.page.min(pages - 1);
    let mut text = format!(
        "Spells: {} | Page {}/{}\r\nCast indices can be assigned in the spell action bar.\r\n",
        entries.len(),
        page + 1,
        pages
    );
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
