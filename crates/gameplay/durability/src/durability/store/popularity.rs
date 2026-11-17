//! Public ranking from immutable in-game purchase receipts; never a coin ledger.
use super::{StoreDurabilityError, StoreValidationError};
use crate::durability::{DurabilityRoot, db::begin_semantic_transaction};
use sqlx::Row;
use std::collections::BTreeMap;
impl DurabilityRoot {
    pub async fn store_popularity(
        &self,
        keys: Vec<String>,
    ) -> Result<BTreeMap<String, u64>, StoreDurabilityError> {
        if keys.len() > 512 || keys.iter().any(|k| k.is_empty() || k.len() > 128) {
            return Err(StoreValidationError::InvalidBytes.into());
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let rows = sqlx::query(
                        "SELECT offer_key,purchases FROM game_store_offer_popularity \
                         WHERE offer_key=ANY($1) LIMIT 512",
                    )
                    .bind(keys)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut result = BTreeMap::new();
                    for row in rows {
                        let count = row.try_get::<i64, _>("purchases")?;
                        let count = u64::try_from(count)
                            .map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                        result.insert(row.try_get("offer_key")?, count);
                    }
                    Ok(Ok(result))
                })
            })
            .await?
    }
}
