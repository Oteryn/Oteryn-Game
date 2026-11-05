use super::*;

pub fn row_text(row: &market::MarketRow) -> String {
    if row.side == 0 {
        return format!("Item {} · select and Browse to see offers", row.definition);
    }
    format!(
        "Item {} · {} · amount {} · price {} · total {}{}",
        row.definition,
        if row.side == 1 { "buy" } else { "sell" },
        row.amount,
        row.price,
        u128::from(row.amount) * u128::from(row.price),
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
    lines.push(format!(
        "Page {} · Total: {}",
        panel.page + 1,
        u128::from(panel.amount) * u128::from(panel.price)
    ));
    if !panel.selected {
        lines.push("Market is unavailable in this session".into());
    }
    if panel.pending.is_some() {
        lines.push("Waiting for the server…".into());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.into());
    }

    lines.join("\r\n")
}
