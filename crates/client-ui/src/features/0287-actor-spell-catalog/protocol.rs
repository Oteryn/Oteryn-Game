use super::Panel;
use oteryn_client_runtime::spell_catalog::SpellCatalog;
// oteryn-choice: replacement resets pagination but keeps the player's local search.
pub fn apply(panel: &mut Panel, catalog: SpellCatalog) {
    if panel.selected {
        panel.catalog.replace(catalog);
        panel.page = 0;
    }
}

// oteryn-choice: filtered paging uses the same name/words search as the visible list.
pub fn navigate(panel: &mut Panel, filter: &str, action: usize) {
    if filter.len() > 64 || filter.contains('\0') {
        return;
    }
    if panel.filter != filter {
        panel.filter = filter.to_owned();
        panel.page = 0;
    }
    let search = filter.to_lowercase();
    let count = panel
        .catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&search)
                || entry.words.to_lowercase().contains(&search)
        })
        .count();
    panel.page = match action {
        1 => panel.page.saturating_sub(1),
        2 => panel
            .page
            .saturating_add(1)
            .min(count.saturating_sub(1) / 16),
        _ => 0,
    };
}
