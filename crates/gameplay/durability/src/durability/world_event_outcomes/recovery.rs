//! Operator-requested epoch changes, bounded full-lineage replay and P90D expiry.
use super::*;
use oteryn_gameplay_native_admission_source::account_characters::EpochFence;
impl DurabilityRoot {
    pub async fn t94_begin_resync(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: &str,
        fence: &mut impl EpochFence,
    ) -> Result<u64> {
        if !wire::uuid7_valid(request) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let floor = fence.read().map_err(invalid)?;
        let recovery = authority.record_for(self).map_err(invalid)?;
        let request = request.to_owned();
        let epoch=self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let (stored,previous,generation,reconciled):(String,Option<String>,String,bool)=sqlx::query_as("SELECT projection_epoch::text,last_resync_request_id::text,recovery_generation::text,reconciled FROM game_t94_control WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
   if wire::decimal(&generation,false).map_err(invalid)?!=recovery.recovery_generation||!reconciled{return Err(DurabilityError::Unavailable)}
   let current=wire::decimal(&stored,false).map_err(invalid)?;
   if previous.as_deref()==Some(&request){if current<floor{return Err(DurabilityError::Unavailable)}commit_semantic_transaction(tx,deadline).await?;return Ok(current)}
   let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_delivery_lanes WHERE lease_until>clock_timestamp())").fetch_one(&mut *tx).await?;
   if active{return Err(DurabilityError::Unavailable)}
   let next=current.max(floor).checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
   let changed=sqlx::query("UPDATE game_t94_control SET projection_epoch=$1::text::numeric,recovery_generation=$2::text::numeric,last_resync_request_id=$3::uuid,reconciled=true,resync_complete=false,complete_through=0 WHERE singleton AND NOT quarantined")
    .bind(next.to_string()).bind(recovery.recovery_generation.to_string()).bind(request).execute(&mut *tx).await?.rows_affected();
   if changed!=1{return Err(DurabilityError::Unavailable)}
   commit_semantic_transaction(tx,deadline).await?;Ok(next)
  })).await?;
        if epoch > floor {
            fence.persist(epoch).map_err(invalid)?
        }
        Ok(epoch)
    }
    pub async fn t94_expire(&self, authority: &ReconciledCharacterAuthority<'_, '_>) -> Result<()> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   sqlx::query("UPDATE game_t94_publications SET audit='\\x'::bytea,audit_acknowledged=true WHERE publication_id IN(SELECT publication_id FROM game_t94_publications WHERE occurred_at_ms<floor(extract(epoch FROM statement_timestamp())*1000)::bigint-7776000000 AND octet_length(audit)>0 ORDER BY delivery_order LIMIT 64 FOR UPDATE SKIP LOCKED)").execute(&mut *tx).await?;
   sqlx::query("DELETE FROM game_t94_publications WHERE publication_id IN(SELECT p.publication_id FROM game_t94_publications p JOIN game_t94_subjects s USING(family,world_id,subject_id) WHERE s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000 ORDER BY p.delivery_order LIMIT 64 FOR UPDATE OF p SKIP LOCKED)").execute(&mut *tx).await?;
   sqlx::query("UPDATE game_t94_subjects s SET record='\\x'::bytea WHERE (s.family,s.world_id,s.subject_id) IN(SELECT q.family,q.world_id,q.subject_id FROM game_t94_subjects q WHERE q.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000 AND octet_length(q.record)>0 AND NOT EXISTS(SELECT 1 FROM game_t94_publications p WHERE p.family=q.family AND p.world_id=q.world_id AND p.subject_id=q.subject_id) ORDER BY q.final_at,q.family,q.world_id,q.subject_id LIMIT 64 FOR UPDATE OF q SKIP LOCKED)").execute(&mut *tx).await?;
   sqlx::query("UPDATE game_t94_native_receipts SET receipt='\\x'::bytea,expired_at=floor(extract(epoch FROM statement_timestamp()))::bigint WHERE receipt_id IN(SELECT receipt_id FROM game_t94_native_receipts WHERE observed_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000 AND octet_length(receipt)>0 ORDER BY observed_at,receipt_id LIMIT 64 FOR UPDATE SKIP LOCKED)").execute(&mut *tx).await?;
   sqlx::query("UPDATE game_t94_evidence SET evidence='\\x'::bytea WHERE evidence_sha256 IN(SELECT evidence_sha256 FROM game_t94_evidence WHERE created_at<statement_timestamp()-interval '90 days' AND octet_length(evidence)>0 ORDER BY created_at,evidence_sha256 LIMIT 64 FOR UPDATE SKIP LOCKED)").execute(&mut *tx).await?;
   sqlx::query("DELETE FROM game_t94_privacy_watch WHERE (guild_id,world_id,season_id) IN(SELECT w.guild_id,w.world_id,w.season_id FROM game_t94_privacy_watch w JOIN game_t94_subjects s ON s.world_id=w.world_id AND s.subject_id=w.season_id AND s.family='guild_season' WHERE s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000 ORDER BY w.guild_id,w.world_id,w.season_id LIMIT 64)").execute(&mut *tx).await?;
   sqlx::query("UPDATE game_t94_contributions SET guild_name='' WHERE (season_id,challenge_id,occurrence_id,guild_id) IN(SELECT x.season_id,x.challenge_id,x.occurrence_id,x.guild_id FROM game_t94_contributions x JOIN game_t94_subjects s ON s.family='guild_season' AND s.world_id=x.world_id AND s.subject_id=x.season_id WHERE s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000 AND x.guild_name<>'' ORDER BY x.season_id,x.challenge_id,x.occurrence_id,x.guild_id LIMIT 64 FOR UPDATE OF x SKIP LOCKED)").execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}
/// One retained publication at a time, including every correction predecessor.
/// Ordinary restart does not call begin_resync and cannot mint an epoch.
pub(super) async fn resync_page(
    tx: &mut Transaction<'_, Postgres>,
    c: &Control,
) -> Result<Disposition> {
    let row=sqlx::query("SELECT p.*,p.publication_id::text AS publication_text FROM game_t94_publications p JOIN game_t94_subjects s USING(family,world_id,subject_id) WHERE (s.final_at IS NULL OR s.final_at>=floor(extract(epoch FROM statement_timestamp()))::bigint-7776000) AND p.projection_epoch=(SELECT max(q.projection_epoch) FROM game_t94_publications q WHERE q.family=p.family AND q.world_id=p.world_id AND q.subject_id=p.subject_id AND q.projection_epoch<$1::text::numeric) AND NOT EXISTS(SELECT 1 FROM game_t94_publications n WHERE (n.family,n.world_id,n.subject_id,n.projection_revision)=(p.family,p.world_id,p.subject_id,p.projection_revision) AND n.projection_epoch=$1::text::numeric) ORDER BY p.delivery_order LIMIT 1 FOR SHARE OF p,s")
  .bind(c.epoch.to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        sqlx::query("UPDATE game_t94_control SET resync_complete=true WHERE singleton")
            .execute(&mut **tx)
            .await?;
        return Ok(Disposition::Applied);
    };
    let bytes: Vec<u8> = row.try_get("wire")?;
    if row.try_get::<Vec<u8>, _>("wire_sha256")? != Sha256::digest(&bytes).as_slice() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut snapshot =
        Snapshot::decode(std::str::from_utf8(&bytes).map_err(invalid)?).map_err(invalid)?;
    match &mut snapshot {
        Snapshot::Event(s) => s.projection_epoch = c.epoch.to_string(),
        Snapshot::Season(s) => s.projection_epoch = c.epoch.to_string(),
    }
    let raw = snapshot.encode().map_err(invalid)?;
    sqlx::query("INSERT INTO game_t94_publications(family,world_id,subject_id,projection_epoch,projection_revision,source_observed_at,wire,wire_sha256,audit_event_id,transaction_id,occurred_at_ms,audit,audit_acknowledged,resync_member) SELECT family,world_id,subject_id,$2::text::numeric,projection_revision,source_observed_at,$3,sha256($3),audit_event_id,transaction_id,occurred_at_ms,audit,audit_acknowledged,true FROM game_t94_publications WHERE publication_id=$1::uuid ON CONFLICT(family,world_id,subject_id,projection_epoch,projection_revision) DO NOTHING")
  .bind(row.try_get::<String,_>("publication_text")?).bind(c.epoch.to_string()).bind(raw.as_bytes()).execute(&mut **tx).await?;
    Ok(Disposition::Applied)
}

impl DurabilityRoot {
    /// F=0 is an operator-provisioned proof that this namespace has never posted.
    /// A missing file, F>=1, nonempty history or mismatched recovery can never
    /// turn an empty restored cache into fresh authority.
    pub async fn t94_initialize(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        source: &str,
        fence: &mut impl EpochFence,
    ) -> Result<Disposition> {
        if !oteryn_gameplay_native_admission_source::character_commands::source_authority_valid(
            source,
        ) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let floor = fence.read().map_err(invalid)?;
        let recovery = authority.record_for(self).map_err(invalid)?;
        let source = source.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let row=sqlx::query("SELECT enabled,reconciled,quarantined,recovery_generation::text,source_authority FROM game_t94_control WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
   if row.try_get::<bool,_>("quarantined")?{return Err(DurabilityError::Unavailable)}
   if !row.try_get::<bool,_>("enabled")?{return Ok(Disposition::Pending)}
   let generation=wire::decimal(&row.try_get::<String,_>("recovery_generation")?,true).map_err(invalid)?;
   if row.try_get::<bool,_>("reconciled")?&&generation==recovery.recovery_generation&&row.try_get::<String,_>("source_authority")?==source{commit_semantic_transaction(tx,deadline).await?;return Ok(Disposition::Replayed)}
   if floor!=0||generation!=0||row.try_get::<bool,_>("reconciled")?{return Err(DurabilityError::Unavailable)}
   let history:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_subjects) OR EXISTS(SELECT 1 FROM game_t94_publications) OR EXISTS(SELECT 1 FROM game_t94_native_receipts) OR EXISTS(SELECT 1 FROM game_t94_contributions) OR EXISTS(SELECT 1 FROM game_t94_world_policies) OR EXISTS(SELECT 1 FROM game_t94_default_seasons) OR EXISTS(SELECT 1 FROM game_t94_evidence) OR EXISTS(SELECT 1 FROM game_t94_raid_windows) OR EXISTS(SELECT 1 FROM game_t94_privacy_watch) OR EXISTS(SELECT 1 FROM game_t94_corrections WHERE receipt IS NOT NULL)").fetch_one(&mut *tx).await?;
   if history{return Err(DurabilityError::Unavailable)}
   sqlx::query("UPDATE game_t94_control SET recovery_generation=$1::text::numeric,reconciled=true,resync_complete=true,source_authority=$2,source_started_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE singleton").bind(recovery.recovery_generation.to_string()).bind(source).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(Disposition::Applied)
  })).await
    }
}
