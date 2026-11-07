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
    lines.extend(panel.rows.iter().map(row_label));
    lines.join("\r\n")
}
