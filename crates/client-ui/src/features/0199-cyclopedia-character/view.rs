use super::CyclopediaCharacterPanel;
#[must_use]
pub fn panel_text(p: &CyclopediaCharacterPanel) -> String {
    let state = p
        .result
        .as_deref()
        .unwrap_or("No server character view yet");
    let heading = format!(
        "Cyclopedia character — {:?}, offset {}, limit {}",
        p.tab, p.offset, p.limit
    );
    match p.feedback {
        Some(f) => format!("{heading}\r\n{state}\r\n{f}"),
        None => format!("{heading}\r\n{state}"),
    }
}

/// Filter only the confirmed history page; the wire pagination remains unchanged.
pub fn history_rows(p: &CyclopediaCharacterPanel) -> Vec<&super::HistoryRow> {
    if p.history_tab != Some(p.tab) {
        return Vec::new();
    }
    p.history
        .iter()
        .filter(|row| {
            row.status.is_none() || p.kill_filter.is_none() || row.status == p.kill_filter
        })
        .collect()
}
