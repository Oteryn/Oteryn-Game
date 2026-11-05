use super::super::DurabilityRoot;
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::codec::StoreValidationError;
use super::storage::StoreDurabilityError;
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePurchaseHistoryEntry {
    pub order_id: [u8; 16],
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: u64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePurchaseHistoryRow {
    pub purchase_request_id: [u8; 16],
    pub entry: StorePurchaseHistoryEntry,
    pub purchased_at_ms: u64,
}

impl DurabilityRoot {
    pub async fn store_purchase_history_page(
        &self,
        account_id: [u8; 16],
        after: Option<(u64, [u8; 16])>,
        limit: u16,
    ) -> Result<Vec<StorePurchaseHistoryRow>, StoreDurabilityError> {
        if !(1..=33).contains(&limit)
            || after.is_some_and(|(milliseconds, _)| milliseconds > i64::MAX as u64)
        {
            return Err(StoreValidationError::InvalidBytes.into());
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let (after_ms, after_id) = after
                        .map(|(ms, id)| (Some(ms as i64), Some(id)))
                        .unwrap_or((None, None));
                    let rows = sqlx::query(
                        "SELECT replace(purchase_request_id::text,'-','') AS purchase_request_id, \
                         replace(order_id::text,'-','') AS order_id, offer_key, catalog_revision, \
                         price, currency, purchased_at_ms FROM game_store_purchase_history \
                         WHERE account_id=encode($1,'hex')::uuid AND ($2::bigint IS NULL OR \
                         (purchased_at_ms,purchase_request_id)<($2,encode($3,'hex')::uuid)) \
                         ORDER BY purchased_at_ms DESC,purchase_request_id DESC LIMIT $4",
                    )
                    .bind(account_id.as_slice())
                    .bind(after_ms)
                    .bind(after_id.map(|id| id.to_vec()))
                    .bind(i64::from(limit))
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut result = Vec::with_capacity(rows.len());
                    for row in rows {
                        let Some(purchase_request_id) =
                            parse_hex_uuid(row.try_get("purchase_request_id")?)
                        else {
                            return Ok(Err(StoreValidationError::InvalidBytes.into()));
                        };
                        let Some(order_id) = parse_hex_uuid(row.try_get("order_id")?) else {
                            return Ok(Err(StoreValidationError::InvalidBytes.into()));
                        };
                        result.push(StorePurchaseHistoryRow {
                            purchase_request_id,
                            entry: StorePurchaseHistoryEntry {
                                order_id,
                                offer_key: row.try_get("offer_key")?,
                                catalog_revision: row.try_get("catalog_revision")?,
                                price: row.try_get::<i64, _>("price")? as u64,
                                currency: row.try_get("currency")?,
                            },
                            purchased_at_ms: row.try_get::<i64, _>("purchased_at_ms")? as u64,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }
}

fn parse_hex_uuid(text: String) -> Option<[u8; 16]> {
    if text.len() != 32 {
        return None;
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}
