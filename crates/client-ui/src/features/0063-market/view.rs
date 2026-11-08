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

    if panel.definition != 0 {
        lines.push("Statistics cover the loaded item page".into());
        for (side, name) in [(1, "buy"), (2, "sell")] {
            let mut count = 0u64;
            let mut units = 0u128;
            let mut value = 0u128;
            let mut min = u64::MAX;
            let mut max = 0;
            for row in panel.rows.iter().filter(|row| {
                row.definition == panel.definition && row.side == side && row.amount > 0
            }) {
                count += 1;
                units += u128::from(row.amount);
                value += u128::from(row.amount) * u128::from(row.price);
                min = min.min(row.price);
                max = max.max(row.price);
            }
            if let Some(average) = value.checked_div(units) {
                lines.push(format!("Loaded page {name}: {count} offers, {units} units, price {min}–{max}, weighted average {average}"));
            } else {
                lines.push(format!("Loaded page {name}: no offers"));
            }
        }
    }
    lines.join("\r\n")
}
