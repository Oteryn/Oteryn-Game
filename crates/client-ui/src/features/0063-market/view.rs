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

/// Whether an active book offer is outside 75%–125% of its side's settled average.
pub fn offer_price_outlier(panel: &MarketPanel, row: &market::MarketRow) -> bool {
    if panel.view != MarketView::Browse || row.state != 1 || row.definition != panel.definition {
        return false;
    }
    let Some(statistics) = panel
        .details
        .statistics
        .as_ref()
        .filter(|statistics| statistics.definition == row.definition)
    else {
        return false;
    };
    let average = match row.side {
        1 => statistics.buy.average,
        2 => statistics.sell.average,
        _ => None,
    };
    average.is_some_and(|average| {
        let average = u128::from(average);
        let scaled_price = u128::from(row.price) * 100;
        scaled_price < average * 75 || scaled_price > average * 125
    })
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
        lines.push(super::details::statistics_text(panel));
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
        for index in visible_rows(panel) {
            let row = &panel.rows[index];
            if offer_price_outlier(panel, row) {
                lines.push(lfmt!("Warning: {text}", text = row_text(row)));
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

pub fn coin_panel_text(p: &super::MarketPanel) -> String {
    super::coin_text(&p.coins)
}

/// A native table click refers to the identity that was displayed before asynchronous replies.
pub fn table_selection<'a>(
    rows: &'a [market::MarketRow],
    shown: &[([u8; 16], u32)],
    index: usize,
) -> Option<&'a market::MarketRow> {
    let &(offer, definition) = shown.get(index)?;
    rows.iter()
        .find(|row| row.offer == offer && row.definition == definition)
}
