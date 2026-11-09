use super::*;

pub fn row_text(row: &market::MarketRow) -> String {
    if row.side == 0 {
        return lfmt!("Item {} · select and Browse to see offers", row.definition);
    }
    lfmt!(
        "Item {} · {} · amount {} · price {} · total {}{}",
        row.definition,
        if row.side == 1 { "buy" } else { "sell" },
        row.amount,
        row.price,
        u128::from(row.amount) * u128::from(row.price),
        if row.anonymous {
            tr!(" · anonymous")
        } else {
            ""
        }
    )
}
pub fn panel_text(panel: &MarketPanel) -> String {
    let mut lines = vec![
        match panel.view {
            MarketView::Browse => tr!("Market offers"),
            MarketView::Own => tr!("My offers"),
            MarketView::History => tr!("Market history"),
        }
        .to_owned(),
    ];
    lines.push(format!(
        "Loaded page side: {}",
        match panel.side_filter {
            1 => "buy",
            2 => "sell",
            _ => "all",
        }
    ));
    lines.push(lfmt!(
        "Page {} · Total: {}",
        panel.page + 1,
        u128::from(panel.amount) * u128::from(panel.price)
    ));
    if !panel.selected {
        lines.push(tr!("Market is unavailable in this session").into());
    }
    if panel.pending.is_some() {
        lines.push(tr!("Waiting for the server…").into());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.into());
    }

    if panel.definition != 0 {
        lines.push(tr!("Statistics cover the loaded item page").into());
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
                lines.push(lfmt!("Loaded page {name}: {count} offers, {units} units, price {min}–{max}, weighted average {average}",average=average,count=count,max=max,min=min,name=name,units=units));
            } else {
                lines.push(lfmt!("Loaded page {name}: no offers", name = name));
            }
        }
    }
    lines.join("\r\n")
}

/// Original server indices keep selection bound to the offer after filtering.
pub fn visible_rows(panel: &MarketPanel) -> Vec<usize> {
    panel
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| panel.side_filter == 0 || row.side == 0 || row.side == panel.side_filter)
        .map(|(index, _)| index)
        .collect()
}
