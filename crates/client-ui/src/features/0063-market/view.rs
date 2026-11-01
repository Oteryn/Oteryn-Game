use super::*;

pub fn row_text(row: &market::MarketRow) -> String {
    format!(
        "Item {} · {} · amount {} · price {}{}",
        row.definition,
        if row.side == 1 { "buy" } else { "sell" },
        row.amount,
        row.price,
        if row.anonymous { " · anonymous" } else { "" }
    )
}
pub fn panel_text(panel: &MarketPanel) -> String {
    let mut lines = vec![
        match panel.view {
            MarketView::Browse => "Market offers",
            MarketView::Own => "My offers",
            MarketView::History => "Market history",
        }
        .to_owned(),
    ];
    if !panel.selected {
        lines.push("Market is unavailable in this session".into());
    }
    if panel.pending.is_some() {
        lines.push("Waiting for the server…".into());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.into());
    }
    lines.extend(panel.rows.iter().map(row_text));
    lines.join("\r\n")
}
