//! The existing ANL durable outbox seam: registered EventEnvelope bytes, bounded
//! batches and digest-matched acknowledgements. No new network endpoint.
use super::*;

#[derive(Debug, Clone)]
pub struct AuditDelivery {
    pub event_id: [u8; 16],
    pub envelope: Vec<u8>,
    pub envelope_digest: [u8; 32],
}
impl DurabilityRoot {
    pub async fn expire_bazaar_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        limit: u16,
    ) -> Result<u64, DurabilityError> {
        if limit == 0 || limit > 64 {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let count: i32 = sqlx::query_scalar("SELECT game_bazaar_expire_audit($1)")
                        .bind(i32::from(limit))
                        .fetch_one(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    u64::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)
                })
            })
            .await
    }

    pub async fn read_bazaar_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        limit: u16,
    ) -> Result<Vec<AuditDelivery>, DurabilityError> {
        if limit == 0 || limit > 64 {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            let rows=sqlx::query("SELECT event_id::text,envelope,envelope_digest FROM game_bazaar_audit_outbox WHERE NOT acknowledged ORDER BY occurred_at,event_id LIMIT $1 FOR UPDATE SKIP LOCKED").bind(i64::from(limit)).fetch_all(&mut *tx).await?;
            let mut deliveries=Vec::with_capacity(rows.len());
            for row in rows {
                let digest:Vec<u8>=row.try_get("envelope_digest")?;
                deliveries.push(AuditDelivery{event_id:super::super::character_authority::uuid_text(row.try_get("event_id")?)?,envelope:row.try_get("envelope")?,envelope_digest:digest.try_into().map_err(|_|DurabilityError::InvalidStoredState)?});
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(deliveries)
        })).await
    }

    pub async fn acknowledge_bazaar_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        event: [u8; 16],
        digest: [u8; 32],
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            let changed=sqlx::query("UPDATE game_bazaar_audit_outbox SET acknowledged=true WHERE event_id=encode($1,'hex')::uuid AND envelope_digest=$2 AND NOT acknowledged").bind(event.as_slice()).bind(digest.as_slice()).execute(&mut *tx).await?.rows_affected()==1;
            commit_semantic_transaction(tx,deadline).await?;Ok(changed)
        })).await
    }
}
