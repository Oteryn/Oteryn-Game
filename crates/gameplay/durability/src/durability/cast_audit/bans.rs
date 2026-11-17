//! Game-owned, account-scoped persistent bans. Account UUIDs come exclusively
//! from authenticated admission; no Platform foreign key or character lease.
use super::*;
impl DurabilityRoot {
    pub async fn cast_account_banned(
        &self,
        node: &NodeIncarnationProof,
        owner: [u8; 16],
        subject: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            if !prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::Unavailable);}
            let banned=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_cast_account_bans WHERE owner_account=$1 AND subject_account=$2)").bind(owner.as_slice()).bind(subject.as_slice()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(banned)
        })).await
    }
    /// Ban and its immutable audit event commit together, before active viewers
    /// are disconnected. Retrying an existing ban emits no duplicate event.
    pub async fn ban_cast_account(
        &self,
        node: &NodeIncarnationProof,
        owner: [u8; 16],
        subject: [u8; 16],
        record: CastAuditRecord,
    ) -> Result<(), DurabilityError> {
        if owner == [0; 16]
            || subject == [0; 16]
            || owner == subject
            || record.kind != 12
            || record.subject_account.as_deref() != Some(subject.as_slice())
            || record.target_session.len() != 16
            || record.cast_id.len() != 16
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            if !prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::Unavailable);}
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),1095))").bind(owner.as_slice()).execute(&mut *tx).await?;
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_cast_account_bans WHERE owner_account=$1 AND subject_account=$2)").bind(owner.as_slice()).bind(subject.as_slice()).fetch_one(&mut *tx).await?;
            if !exists {
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_cast_account_bans WHERE owner_account=$1").bind(owner.as_slice()).fetch_one(&mut *tx).await?;
                if count>=512 {return Err(DurabilityError::InvalidStoredState);}
                sqlx::query("INSERT INTO game_cast_account_bans(owner_account,subject_account) VALUES($1,$2)").bind(owner.as_slice()).bind(subject.as_slice()).execute(&mut *tx).await?;
                let raw:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())").fetch_one(&mut *tx).await?;
                let incarnation=raw.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                let record=CastAuditRecord {ordinal:1,..record};
                write_batch(&mut tx,&node,incarnation,vec![record]).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(())
        })).await
    }
}
