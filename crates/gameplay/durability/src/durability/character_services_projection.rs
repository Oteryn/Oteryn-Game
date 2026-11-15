//! Fenced service-state reads and independent delivery acknowledgement (2833).
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::native_admission_source::account_characters::WatermarkFacts;
use crate::native_admission_source::character_services_projection::{Snapshot, Store};
use sqlx::Row;
/// Publication authority is borrowed from the same reconciled root as mutations.
pub struct CharacterServicesStore<'r, 'a, 'f, 's> {
    pub root: &'r DurabilityRoot,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub proof: &'r NodeIncarnationProof,
}
impl Store for CharacterServicesStore<'_, '_, '_, '_> {
    async fn next(&mut self) -> Result<Option<Snapshot>, ()> {
        let recovery = self.authority.record_for(self.root).map_err(|_| ())?;
        let proof = self.proof.clone();
        self.root.try_issue_semantic_pass().map_err(|_|())?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            sqlx::query("SELECT game_character_service_resync_page()").execute(&mut *tx).await?;
            let row=sqlx::query("SELECT o.account_id::text,o.character_id::text,o.projection_epoch,o.projection_revision,o.snapshot FROM game_character_service_outbox o CROSS JOIN game_character_service_feed f JOIN game_character_account_projection_epoch e ON e.epoch_scope=1 WHERE f.singleton AND f.quarantined_epoch<e.projection_epoch AND o.projection_epoch=e.projection_epoch AND o.projection_revision>o.delivered_revision AND NOT EXISTS(SELECT 1 FROM game_character_service_outbox q WHERE q.account_id=o.account_id AND q.projection_epoch=e.projection_epoch AND q.quarantined) ORDER BY o.committed_at_ms,o.account_id,o.character_id LIMIT 1")
                .fetch_optional(&mut *tx).await?;
            let row=row.map(|row|->Result<_,DurabilityError>{Ok(Snapshot {
                account_id:row.try_get("account_id")?,character_id:row.try_get("character_id")?,
                epoch:u64::try_from(row.try_get::<i64,_>("projection_epoch")?).map_err(|_|DurabilityError::InvalidStoredState)?,
                revision:u64::try_from(row.try_get::<i64,_>("projection_revision")?).map_err(|_|DurabilityError::InvalidStoredState)?,
                facts:row.try_get("snapshot")?,
            })}).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(row)
        })).await.map_err(|_|())
    }
    async fn acknowledge(&mut self, snapshot: &Snapshot) -> Result<(), ()> {
        self.disposition(snapshot, false).await
    }
    async fn quarantine(&mut self, snapshot: &Snapshot) -> Result<(), ()> {
        self.disposition(snapshot, true).await
    }
    async fn quarantine_feed(&mut self, epoch: u64) -> Result<(), ()> {
        let recovery = self.authority.record_for(self.root).map_err(|_| ())?;
        let proof = self.proof.clone();
        self.root.try_issue_semantic_pass().map_err(|_|())?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            sqlx::query("UPDATE game_character_service_feed SET quarantined_epoch=projection_epoch WHERE singleton AND projection_epoch=$1::numeric")
                .bind(epoch.to_string()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await
        })).await.map_err(|_|())
    }
    async fn facts(&mut self) -> Result<WatermarkFacts, ()> {
        let recovery = self.authority.record_for(self.root).map_err(|_| ())?;
        let proof = self.proof.clone();
        self.root.try_issue_semantic_pass().map_err(|_|())?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            // Enable capture in its own short transaction before enumerating roots.
            // Root triggers hold a shared row lock, so all pre-enable writes finish
            // before resync takes its first source snapshot.
            sqlx::query("UPDATE game_character_service_feed SET capture_started=true WHERE singleton AND NOT capture_started").execute(&mut *tx).await?;
            let row=sqlx::query("SELECT e.projection_epoch,game_character_service_now()*1000 AS now_ms,(SELECT min(value) FROM (SELECT committed_at_ms AS value FROM game_character_service_outbox WHERE projection_revision>delivered_revision OR quarantined UNION ALL SELECT resync_started_at_ms FROM game_character_service_feed WHERE phase<>3) pending) AS oldest FROM game_character_account_projection_epoch e JOIN game_character_service_feed f USING(projection_epoch) WHERE e.epoch_scope=1 AND f.singleton AND f.quarantined_epoch<e.projection_epoch")
                .fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            let facts=WatermarkFacts {projection_epoch:u64::try_from(row.try_get::<i64,_>("projection_epoch")?).map_err(|_|DurabilityError::InvalidStoredState)?,oldest_undelivered_ms:row.try_get("oldest")?,now_ms:row.try_get::<Option<i64>,_>("now_ms")?.ok_or(DurabilityError::Unavailable)?};
            commit_semantic_transaction(tx,deadline).await?;Ok(facts)
        })).await.map_err(|_|())
    }
}
impl CharacterServicesStore<'_, '_, '_, '_> {
    async fn disposition(&self, snapshot: &Snapshot, quarantine: bool) -> Result<(), ()> {
        let recovery = self.authority.record_for(self.root).map_err(|_| ())?;
        let proof = self.proof.clone();
        let snapshot = snapshot.clone();
        self.root.try_issue_semantic_pass().map_err(|_|())?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            // Neither an old epoch nor an ACK racing a new revision clears work.
            sqlx::query("UPDATE game_character_service_outbox o SET delivered_revision=CASE WHEN $5 THEN least(delivered_revision,projection_revision-1) ELSE projection_revision END,quarantined=$5 WHERE account_id=$1::uuid AND character_id=$2::uuid AND projection_epoch=$3::numeric AND ($5 OR projection_revision=$4::numeric) AND (NOT quarantined OR $5) AND projection_epoch=(SELECT projection_epoch FROM game_character_account_projection_epoch WHERE epoch_scope=1)")
                .bind(snapshot.account_id).bind(snapshot.character_id).bind(snapshot.epoch.to_string()).bind(snapshot.revision.to_string()).bind(quarantine).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await
        })).await.map_err(|_|())
    }
}
