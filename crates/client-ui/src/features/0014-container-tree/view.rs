use super::ContainerPanel;
pub fn panel_text(panel: &ContainerPanel) -> String {
    let mut text = match panel.current() {
        Some(view) => lfmt!(
            "Container {} — {} / {} items",
            view.container_handle,
            view.entries.len(),
            view.capacity
        ),
        None if panel.views.is_none() => tr!("Containers unavailable for this session").into(),
        None => tr!("No open containers").into(),
    };
    if panel.bulk.total > 0 {
        text.push_str(&format!(
            " — {} / {} moved",
            panel.bulk.completed, panel.bulk.total
        ));
        if let Some(feedback) = panel.bulk.feedback {
            text.push_str(" — ");
            text.push_str(feedback);
        }
    }
    if panel.pending || panel.bulk.pending {
        text.push_str(tr!(" — Waiting for server"));
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(" — ");
        text.push_str(feedback);
    }
    text
}
