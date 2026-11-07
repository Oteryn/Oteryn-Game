use super::*;
impl DurabilityRoot {
    pub async fn expire_staff_audit(&self) -> Result<u64, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(|holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let count: i64 = sqlx::query_scalar("SELECT game_staff_expire_action_audit()")
                        .fetch_one(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    count
                        .try_into()
                        .map_err(|_| DurabilityError::InvalidStoredState)
                })
            })
            .await
    }
}
