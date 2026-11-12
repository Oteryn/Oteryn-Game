use super::super::{
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction},
};
use super::*;
use oteryn_gameplay_native_admission_source::{
    SourceError,
    journal::{
        ReadGameJournalCursorV1Response,
        publisher::{JournalOutbox, StoredPublication},
    },
};
pub struct ProducerStore<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
}
fn transport_error(error: DurabilityError) -> SourceError {
    match error {
        DurabilityError::InvalidStoredState => SourceError::InvalidInput,
        _ => SourceError::Unavailable,
    }
}
async fn current(
    tx: &mut Transaction<'_, Postgres>,
    stream: &ReadGameJournalCursorV1,
    generation: u64,
) -> Result<bool> {
    if stream.recovery_generation != generation.to_string() {
        return Ok(false);
    }
    let key = stream.encode().map_err(invalid)?;
    let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_journal_export_settings c JOIN game_journal_streams s ON s.source_authority=c.source_authority JOIN game_journal_generations g ON (g.source_authority,g.account_id,g.recovery_generation)=(s.source_authority,s.account_id,s.recovery_generation) WHERE c.singleton AND c.game_journal_items_export_enabled AND s.stream_key=$1 AND s.active AND (convert_from(s.stream_key,'UTF8')::jsonb->>'character_id' IS NULL OR EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=(convert_from(s.stream_key,'UTF8')::jsonb->>'character_id')::uuid AND r.account_id=s.account_id AND r.world_id=(convert_from(s.stream_key,'UTF8')::jsonb->>'world_id')::uuid AND r.character_revision>=(convert_from(s.stream_key,'UTF8')::jsonb->>'character_revision')::numeric AND r.lifecycle=1)))")
  .bind(key).fetch_one(&mut **tx).await?;
    if !live {
        return Ok(false);
    }
    super::ownership::approved_cut(tx, stream).await
}
impl JournalOutbox for ProducerStore<'_, '_, '_> {
    async fn current(
        &self,
        stream: &ReadGameJournalCursorV1,
    ) -> std::result::Result<bool, SourceError> {
        let recovery = self
            .authority
            .record_for(self.root)
            .map_err(|_| SourceError::Unavailable)?;
        let stream = stream.clone();
        self.root
            .try_issue_semantic_pass()
            .map_err(transport_error)?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let live = current(&mut tx, &stream, recovery.recovery_generation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(live)
                })
            })
            .await
            .map_err(transport_error)
    }
    async fn first_for_replay(
        &self,
        stream: &ReadGameJournalCursorV1,
    ) -> std::result::Result<Option<StoredPublication>, SourceError> {
        let recovery = self
            .authority
            .record_for(self.root)
            .map_err(|_| SourceError::Unavailable)?;
        let stream = stream.clone();
        self.root.try_issue_semantic_pass().map_err(transport_error)?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !current(&mut tx,&stream,recovery.recovery_generation).await? {return Err(DurabilityError::InvalidStoredState);}
            let row=sqlx::query("SELECT export_bytes,export_sha256,(export_bytes IS NOT NULL AND payload_expires_at>clock_timestamp()) AS replayable FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=1")
                .bind(stream.encode().map_err(invalid)?).fetch_optional(&mut *tx).await?;
            let publication=row.map(|row|->Result<StoredPublication>{
                if !row.try_get::<bool,_>("replayable")? {return Err(DurabilityError::InvalidStoredState);}
                let bytes:Vec<u8>=row.try_get("export_bytes")?;let hash:Vec<u8>=row.try_get("export_sha256")?;
                if Sha256::digest(&bytes).as_slice()!=hash {return Err(DurabilityError::InvalidStoredState);}
                Ok(StoredPublication{bytes,sha256:hash.try_into().map_err(invalid)?})
            }).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(publication)
        })).await.map_err(transport_error)
    }
    async fn after_cursor(
        &self,
        stream: &ReadGameJournalCursorV1,
        cursor: &ReadGameJournalCursorV1Response,
    ) -> std::result::Result<Option<StoredPublication>, SourceError> {
        let recovery = self
            .authority
            .record_for(self.root)
            .map_err(|_| SourceError::Unavailable)?;
        let stream = stream.clone();
        let cursor = cursor.clone();
        self.root.try_issue_semantic_pass().map_err(transport_error)?.run(move|holder,deadline|Box::pin(async move {
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !current(&mut tx,&stream,recovery.recovery_generation).await? {return Err(DurabilityError::InvalidStoredState);}
   cursor.encode().map_err(invalid)?;let key=stream.encode().map_err(invalid)?;
   let after:u64=cursor.after_source_revision.parse().map_err(invalid)?;
   // A nonzero remote cursor must name the exact retained local identity.
   if after>0 {
    let id:Option<String>=sqlx::query_scalar("SELECT record_id::text FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=$2::text::numeric")
     .bind(&key).bind(after.to_string()).fetch_optional(&mut *tx).await?;
    if id.as_deref()!=cursor.last_record_id.as_deref(){return Err(DurabilityError::InvalidStoredState);}
   }
   sqlx::query("SELECT game_journal_reconcile($1,$2::text::numeric,$3::text::uuid)").bind(&key).bind(after.to_string()).bind(&cursor.last_record_id).execute(&mut *tx).await?;
   let next=u128::from(after)+1;
   let row=sqlx::query("SELECT export_bytes,export_sha256,(export_bytes IS NOT NULL AND payload_expires_at>clock_timestamp()) AS replayable FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=$2::text::numeric")
    .bind(&key).bind(next.to_string()).fetch_optional(&mut *tx).await?;
   let publication=if let Some(row)=row {
    if !row.try_get::<bool,_>("replayable")? {return Err(DurabilityError::InvalidStoredState);}
                let bytes:Vec<u8>=row.try_get("export_bytes")?;let digest:Vec<u8>=row.try_get("export_sha256")?;
    if Sha256::digest(&bytes).as_slice()!=digest {return Err(DurabilityError::InvalidStoredState);}
    Some(StoredPublication{bytes,sha256:digest.try_into().map_err(invalid)?})
   }else{
    let tail:String=sqlx::query_scalar("SELECT next_revision::text FROM game_journal_streams WHERE stream_key=$1").bind(key).fetch_one(&mut *tx).await?;
    if tail!=next.to_string(){return Err(DurabilityError::InvalidStoredState);}
    None
   };
   commit_semantic_transaction(tx,deadline).await?;Ok(publication)
  })).await.map_err(transport_error)
    }
    async fn acknowledge(
        &self,
        stream: &ReadGameJournalCursorV1,
        record: &PublishGameJournalRecordV1,
    ) -> std::result::Result<(), SourceError> {
        let recovery = self
            .authority
            .record_for(self.root)
            .map_err(|_| SourceError::Unavailable)?;
        record.encode().map_err(|_| SourceError::InvalidInput)?;
        let stream = stream.clone();
        let revision = record.source_revision.clone();
        let id = record.record_id.clone();
        self.root
            .try_issue_semantic_pass()
            .map_err(transport_error)?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if !current(&mut tx, &stream, recovery.recovery_generation).await? {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let _: bool = sqlx::query_scalar(
                        "SELECT game_journal_ack($1,$2::text::numeric,$3::text::uuid)",
                    )
                    .bind(stream.encode().map_err(invalid)?)
                    .bind(revision)
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(())
                })
            })
            .await
            .map_err(transport_error)
    }
}

impl ProducerStore<'_, '_, '_> {
    /// Bounded, restartable due-queue. Rechecks every stream after at most 30s
    /// while idle; retained ACKs do not remove a stream from cursor scans.
    pub async fn next_due(&self, source: &str) -> Result<Option<(ReadGameJournalCursorV1, u64)>> {
        let recovery = self.authority.record_for(self.root).map_err(invalid)?;
        let source = source.to_owned();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT game_journal_purge()").execute(&mut *tx).await?;
            let row=sqlx::query("SELECT stream_key,retry_attempt FROM game_journal_streams s WHERE active AND family='ITEMS' AND source_authority=$1 AND next_delivery_at<=clock_timestamp() AND EXISTS(SELECT 1 FROM game_journal_outbox o WHERE o.stream_key=s.stream_key) ORDER BY next_delivery_at,stream_key LIMIT 1")
                .bind(source).fetch_optional(&mut *tx).await?;
            let due=row.map(|r|->Result<_>{let b:Vec<u8>=r.try_get("stream_key")?;Ok((ReadGameJournalCursorV1::decode(&b).map_err(invalid)?,u64::try_from(r.try_get::<i64,_>("retry_attempt")?).map_err(invalid)?))}).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(due)
        })).await
    }
    pub async fn schedule(
        &self,
        stream: &ReadGameJournalCursorV1,
        attempt: u64,
        delay: std::time::Duration,
        progress: bool,
    ) -> Result<()> {
        let recovery = self.authority.record_for(self.root).map_err(invalid)?;
        let key = stream.encode().map_err(invalid)?;
        let attempt = i64::try_from(attempt).map_err(invalid)?;
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query("SELECT game_journal_schedule($1,$2,$3,$4)")
                        .bind(key)
                        .bind(attempt)
                        .bind(delay.as_secs_f64())
                        .bind(progress)
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(())
                })
            })
            .await
    }
    pub async fn maintenance(&self) -> Result<()> {
        let recovery = self.authority.record_for(self.root).map_err(invalid)?;
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query("SELECT game_journal_purge()")
                        .execute(&mut *tx)
                        .await?;
                    let stalled: bool = sqlx::query_scalar("SELECT game_journal_stalled()")
                        .fetch_one(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    if stalled {
                        eprintln!("event=game_journal_stalled reason=no_progress");
                    }
                    Ok(())
                })
            })
            .await
    }
}
