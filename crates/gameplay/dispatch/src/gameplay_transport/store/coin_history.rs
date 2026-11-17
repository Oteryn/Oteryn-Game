//! Read-only projection of the existing Platform ledger, scoped by the admitted account.
use super::*;

pub(super) async fn page(
    client: &mut StoreClient,
    account: &str,
    request: &wire::StoreHistoryRequest,
) -> Option<wire::CoinHistoryPage> {
    let before = if request.page_cursor.is_empty() {
        "0"
    } else {
        &request.page_cursor
    };
    let page = client.history(account, before, 32).await.ok()?;
    let entries = page
        .entries
        .into_iter()
        .map(|row| {
            Some(wire::CoinHistoryEntry {
                entry_id: row.entry_id.parse().ok()?,
                kind: row.kind,
                currency: row.currency,
                available_delta: row.available_delta.parse().ok()?,
                reserved_delta: row.reserved_delta.parse().ok()?,
                order_id: match row.order_id {
                    Some(id) => Some(uuid_bytes(&id)?),
                    None => None,
                },
                offer_key: row.offer_key,
                quantity: row.quantity,
                created_at: row.created_at.parse().ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let result = wire::CoinHistoryPage {
        entries,
        next_page_cursor: if page.next_before_entry_id == "0" {
            String::new()
        } else {
            page.next_before_entry_id
        },
    };
    wire::encode_coin_history_page(&result).ok()?;
    Some(result)
}

#[cfg(test)]
#[path = "coin_history_tests.rs"]
mod tests;
