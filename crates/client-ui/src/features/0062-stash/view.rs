use super::*;

pub fn row_label(row: &stash::StashRow) -> String {
    format!(
        "{} ({}) × {}{}",
        row.definition_key,
        row.definition_revision,
        row.quantity,
        if row.migration_required {
            " · migration required"
        } else {
            ""
        }
    )
}

pub fn panel_text(panel: &StashPanel) -> String {
    let mut lines = Vec::new();
    if !panel.selected {
        lines.push("Stash is unavailable in this session".to_owned());
    }
    if panel.pending {
        lines.push("Waiting for the server…".to_owned());
    }
    if let Some(feedback) = &panel.feedback {
        lines.push(feedback.clone());
    }
    lines.push(format!(
        "Page {}{}",
        panel.page + 1,
        if panel.has_more {
            " · more available"
        } else {
            ""
        }
    ));
    lines.push("Select a carried item to stow its stack. Open carried bags in Depot to list their items. Retrieve depot items there before stowing. The server accepts eligible items in the main backpack.".into());
    lines.push(format!(
        "Search/category/trader apply across the stash. Sort: {:?}",
        panel.sort
    ));
    let visible = visible_rows(panel);
    if visible.is_empty() {
        lines.push("No matching items".into());
    }
    lines.extend(
        visible
            .into_iter()
            .map(|index| row_label(&panel.rows[index])),
    );
    lines.join("\r\n")
}

/// Keep display order separate from the authoritative server row order.
pub fn visible_rows(panel: &StashPanel) -> Vec<usize> {
    if panel.server_filtered {
        return (0..panel.rows.len()).collect();
    }
    let query = panel.query.trim().to_lowercase();
    let mut rows: Vec<_> = panel
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.definition_key.to_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect();
    rows.sort_by(|a, b| {
        let (a, b) = (&panel.rows[*a], &panel.rows[*b]);
        match panel.sort {
            Sort::Stock => b
                .quantity
                .cmp(&a.quantity)
                .then_with(|| a.definition_key.cmp(&b.definition_key)),
            Sort::Name => a.definition_key.cmp(&b.definition_key),
        }
    });
    rows
}

pub fn visible_withdraw(panel: &StashPanel, row: usize, amount: u32) -> Option<Intent> {
    visible_rows(panel)
        .get(row)
        .copied()
        .map(|row| Intent::Withdraw { row, amount })
}
