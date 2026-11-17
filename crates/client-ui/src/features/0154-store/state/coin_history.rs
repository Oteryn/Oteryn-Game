use crate::store::{StorePage, StorePanel};
use oteryn_client_runtime::{cui3::store::*, server_mechanic::ServerMechanicCommand};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoinHistory {
    pub pending: bool,
    pub page: Option<CoinHistoryPage>,
    pub kind: String,
    pub from: u64,
    pub until: Option<u64>,
}
pub fn coin_history(panel: &mut StorePanel, next: bool) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    let cursor = if next {
        let cursor = &panel.coin_history.page.as_ref()?.next_page_cursor;
        if cursor.is_empty() {
            return None;
        }
        cursor.clone()
    } else {
        String::new()
    };
    let command = crate::store::command(
        184,
        encode_coin_history_request(&StoreHistoryRequest {
            page_cursor: cursor,
        })
        .ok()?,
    )?;
    let command = crate::store::request(panel, command)?;
    panel.coin_history.pending = true;
    panel.page = StorePage::CoinHistory;
    Some(command)
}
pub fn coin_history_filter(panel: &mut StorePanel, kind: &str, from: &str, until: &str) -> bool {
    if panel.pending || !panel.selected {
        return false;
    }
    let parse = || {
        let from = if from.trim().is_empty() {
            0
        } else {
            from.trim().parse::<u64>().ok()?
        };
        let until = if until.trim().is_empty() {
            None
        } else {
            Some(until.trim().parse::<u64>().ok()?)
        };
        (until.is_none_or(|until| from <= until)
            && kind.len() <= 128
            && !kind.chars().any(char::is_control))
        .then_some((from, until))
    };
    let Some((from, until)) = parse() else {
        panel.feedback = Some("Enter a valid transaction kind and timestamp range".into());
        return false;
    };
    panel.coin_history.kind = kind.trim().to_uppercase();
    panel.coin_history.from = from;
    panel.coin_history.until = until;
    panel.feedback = None;
    true
}
pub fn coin_history_text(panel: &CoinHistory) -> String {
    let mut text = "Store — Coin ledger\r\nFilters and totals cover this page; use Next page for older entries.".to_owned();
    let Some(page) = &panel.page else {
        return text;
    };
    let mut totals = std::collections::BTreeMap::<&str, (i128, i128)>::new();
    for row in page.entries.iter().filter(|row| {
        row.kind.contains(&panel.kind)
            && row.created_at >= panel.from
            && panel.until.is_none_or(|until| row.created_at <= until)
    }) {
        text.push_str(&format!(
            "\r\nEntry {} — {} — {}\r\nAvailable: {:+}; reserved: {:+}; timestamp {}",
            row.entry_id,
            row.kind,
            row.currency,
            row.available_delta,
            row.reserved_delta,
            row.created_at
        ));
        if let Some(order) = row.order_id {
            let id = order.iter().map(|b| format!("{b:02x}")).collect::<String>();
            text.push_str(&format!(
                "\r\nOrder {id}; {} × {}",
                row.quantity.unwrap_or_default(),
                row.offer_key.as_deref().unwrap_or_default()
            ));
        }
        let total = totals.entry(&row.currency).or_default();
        total.0 += i128::from(row.available_delta);
        total.1 += i128::from(row.reserved_delta);
    }
    if totals.is_empty() {
        text.push_str("\r\nNo entries match this page's filters");
    }
    for (currency, (available, reserved)) in totals {
        text.push_str(&format!(
            "\r\nPage total {currency}: available {available:+}, reserved {reserved:+}"
        ));
    }
    text.push_str(if page.next_page_cursor.is_empty() {
        "\r\nEnd of coin history"
    } else {
        "\r\nMore entries: Next page"
    });
    text
}
