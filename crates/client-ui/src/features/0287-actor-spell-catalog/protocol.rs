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

    let count = panel
        .catalog
        .entries()
        .iter()
        .filter(|entry| panel.matches(entry))
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

/// Local filters never change the server's eligibility decision.
pub fn set_filters(panel: &mut Panel, max_level: &str, group: &str, learned_only: bool) -> bool {
    fn number(text: &str) -> Option<Option<u32>> {
        if text.trim().is_empty() {
            Some(None)
        } else {
            text.trim()
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
                .map(Some)
        }
    }
    let (Some(max_level), Some(group)) = (number(max_level), number(group)) else {
        panel.filter_error = true;
        return false;
    };
    panel.filter_error = false;
    if (panel.max_level, panel.group, panel.learned_only) != (max_level, group, learned_only) {
        panel.page = 0;
    }
    panel.max_level = max_level;
    panel.group = group;
    panel.learned_only = learned_only;
    true
}
