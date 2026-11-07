use super::*;
use oteryn_gameplay_native_admission_source::world_event_outcomes::{Publication, Watermark};

impl DurabilityRoot {
    pub async fn t94_next(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        family: &str,
    ) -> Result<Option<Publication>> {
        if !matches!(family, "world_event_outcome" | "guild_season") {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(invalid)?;
        let family = family.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(None)};
   let row=sqlx::query("SELECT publication_id::text,projection_epoch::text,projection_revision::text,wire,wire_sha256,audit,audit_event_id::text,transaction_id::text FROM game_t94_publications WHERE family=$1 AND NOT acknowledged AND projection_epoch=$2::text::numeric AND NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE (s.family,s.world_id,s.subject_id)=(game_t94_publications.family,game_t94_publications.world_id,game_t94_publications.subject_id) AND s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000) ORDER BY delivery_order LIMIT 1")
    .bind(&family).bind(c.epoch.to_string()).fetch_optional(&mut *tx).await?;
   let publication=row.map(|r|->Result<Publication>{
    let bytes:Vec<u8>=r.try_get("wire")?;let hash:Vec<u8>=r.try_get("wire_sha256")?;if hash!=Sha256::digest(&bytes).as_slice(){return Err(DurabilityError::InvalidStoredState)}
    let wire=String::from_utf8(bytes).map_err(invalid)?;Snapshot::decode(&wire).map_err(invalid)?;
    Ok(Publication{id:r.try_get("publication_id")?,delivery_token:None,family:family.clone(),epoch:wire::decimal(&r.try_get::<String,_>("projection_epoch")?,false).map_err(invalid)?,revision:wire::decimal(&r.try_get::<String,_>("projection_revision")?,false).map_err(invalid)?,wire,sha256:hash,audit:r.try_get("audit")?,audit_event_id:r.try_get("audit_event_id")?,transaction_id:r.try_get("transaction_id")?})
   }).transpose()?;
   commit_semantic_transaction(tx,deadline).await?;Ok(publication)
  })).await
    }
    pub async fn t94_ack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        publication: &Publication,
    ) -> Result<bool> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        let p = publication.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(false)};
   if c.epoch!=p.epoch{return Ok(false)}
   let changed=sqlx::query("UPDATE game_t94_publications SET acknowledged=true WHERE publication_id=$1::uuid AND family=$2 AND projection_epoch=$3::text::numeric AND projection_revision=$4::text::numeric AND wire=$5 AND wire_sha256=$6 AND NOT acknowledged AND EXISTS(SELECT 1 FROM game_t94_delivery_lanes l WHERE l.family=game_t94_publications.family AND l.token=$7::uuid AND l.lease_until>clock_timestamp())")
    .bind(p.id).bind(p.family).bind(p.epoch.to_string()).bind(p.revision.to_string()).bind(p.wire.as_bytes()).bind(p.sha256).bind(p.delivery_token).execute(&mut *tx).await?.rows_affected()==1;
   commit_semantic_transaction(tx,deadline).await?;Ok(changed)
  })).await
    }
    pub async fn t94_watermark(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        family: &str,
    ) -> Result<Option<Watermark>> {
        if !matches!(family, "world_event_outcome" | "guild_season") {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(invalid)?;
        let family = family.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(None)};
   if !c.resync||c.cut==0{return Ok(None)}
   let missing_calendar:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_world_policies w WHERE w.first_start<=floor(extract(epoch FROM statement_timestamp())) AND NOT EXISTS(SELECT 1 FROM game_t94_default_seasons s WHERE s.world_id=w.world_id AND s.starts_at<=floor(extract(epoch FROM statement_timestamp())) AND s.starts_at+w.season_seconds>floor(extract(epoch FROM statement_timestamp()))))").fetch_one(&mut *tx).await?;
   if missing_calendar{return Ok(None)}
   if family=="guild_season"{
    let private_display_pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_privacy_watch w JOIN game_t94_subjects s ON s.family='guild_season' AND s.world_id=w.world_id AND s.subject_id=w.season_id LEFT JOIN game_guilds g ON g.guild_id=w.guild_id AND g.world_id=w.world_id LEFT JOIN game_t94_bindings b ON b.family=s.family AND b.world_id=s.world_id AND b.subject_id=s.subject_id WHERE NOT s.display_withdrawn AND (s.final_at IS NULL OR s.final_at>=floor(extract(epoch FROM statement_timestamp()))::bigint-7776000) AND (g.guild_id IS NULL OR g.state=4) AND w.public_name IS DISTINCT FROM (convert_from(b.binding,'UTF8')::jsonb->>'tombstone_name'))").fetch_one(&mut *tx).await?;
    if private_display_pending{return Ok(None)}
   }
   if family=="world_event_outcome"{
    let missing_gate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_world_policies w JOIN game_world_event_epochs e ON e.world_id=w.world_id WHERE w.fury_gate_realization_available AND NOT EXISTS(SELECT 1 FROM game_t94_native_receipts n WHERE n.world_id=w.world_id AND n.kind='DAILY_WORLD_GATE' AND n.source_key='fury_gates' AND n.source_cut='reset/'||e.reset_epoch::text))").fetch_one(&mut *tx).await?;
    if missing_gate{return Ok(None)}
   }
   let staging:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_publications WHERE projection_epoch=$1::text::numeric AND resync_member AND NOT acknowledged)").bind(c.epoch.to_string()).fetch_one(&mut *tx).await?;
   if staging{return Ok(None)}
   let pending:Option<i64>=sqlx::query_scalar("SELECT min(source_observed_at) FROM game_t94_publications WHERE family=$1 AND projection_epoch=$2::text::numeric AND NOT acknowledged AND NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE (s.family,s.world_id,s.subject_id)=(game_t94_publications.family,game_t94_publications.world_id,game_t94_publications.subject_id) AND s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000)")
    .bind(&family).bind(c.epoch.to_string()).fetch_one(&mut *tx).await?;
   let unprocessed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_bindings b WHERE b.approved AND b.family=$1 AND NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE s.family=b.family AND s.world_id=b.world_id AND s.subject_id=b.subject_id AND s.final_at<floor(extract(epoch FROM statement_timestamp()))::bigint-7776000) AND NOT EXISTS(SELECT 1 FROM game_t94_publications p WHERE p.family=b.family AND p.world_id=b.world_id AND p.subject_id=b.subject_id AND p.projection_epoch=$2::text::numeric))")
    .bind(&family).bind(c.epoch.to_string()).fetch_one(&mut *tx).await?;
   if unprocessed{return Ok(None)}
   let unseen:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_bindings b JOIN game_t94_native_receipts n ON n.world_id=b.world_id WHERE b.approved AND b.family=$1 AND n.delivery_order>b.native_seen_order AND NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE s.family=b.family AND s.world_id=b.world_id AND s.subject_id=b.subject_id AND s.final_state) AND ((b.family='world_event_outcome' AND n.occurrence_id=(convert_from(b.binding,'UTF8')::jsonb->>'occurrence_id')::uuid AND n.kind<>'GUILD_SOURCE') OR (b.family='guild_season' AND n.kind='GUILD_SOURCE' AND (convert_from(b.binding,'UTF8')::jsonb->'rules' ? n.source_key))))")
    .bind(&family).fetch_one(&mut *tx).await?;
   if unseen{return Ok(None)}
   let unbound:bool=if family=="world_event_outcome"{sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_native_receipts n WHERE n.kind<>'GUILD_SOURCE' AND octet_length(n.receipt)>0 AND NOT EXISTS(SELECT 1 FROM game_t94_bindings b WHERE b.family='world_event_outcome' AND b.world_id=n.world_id AND b.approved AND convert_from(b.binding,'UTF8')::jsonb->>'occurrence_id'=n.occurrence_id::text AND convert_from(b.binding,'UTF8')::jsonb->'question'->>'kind'=n.kind))").fetch_one(&mut *tx).await?}else{sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_corrections WHERE approved AND receipt IS NULL)").fetch_one(&mut *tx).await?};
   if unbound{return Ok(None)}
   let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
   let cut=pending.map_or(c.cut,|at|c.cut.min(u64::try_from(at.saturating_sub(1)).unwrap_or(0)));
   if cut==0||cut>now as u64{return Ok(None)}
   let watermark=Watermark{contract_version:1,operation:"PublishLeagueWatermarkV1".into(),source_authority:c.authority,family,projection_epoch:c.epoch.to_string(),complete_through:cut.to_string(),observed_at:now.to_string()};
   watermark.encode(now as u64).map_err(invalid)?;commit_semantic_transaction(tx,deadline).await?;Ok(Some(watermark))
  })).await
    }
    /// Audit delivery has a separate acknowledgement; projection acknowledgement never drops it.
    pub async fn t94_next_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<Option<Publication>> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(_c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(None)};
   let row=sqlx::query("SELECT publication_id::text,family,projection_epoch::text,projection_revision::text,wire,wire_sha256,audit,audit_event_id::text,transaction_id::text FROM game_t94_publications WHERE NOT audit_acknowledged AND octet_length(audit)>0 AND occurred_at_ms>=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-7776000000 ORDER BY delivery_order LIMIT 1").fetch_optional(&mut *tx).await?;
   let p=row.map(|r|->Result<Publication>{Ok(Publication{id:r.try_get("publication_id")?,delivery_token:None,family:r.try_get("family")?,epoch:wire::decimal(&r.try_get::<String,_>("projection_epoch")?,false).map_err(invalid)?,revision:wire::decimal(&r.try_get::<String,_>("projection_revision")?,false).map_err(invalid)?,wire:String::from_utf8(r.try_get("wire")?).map_err(invalid)?,sha256:r.try_get("wire_sha256")?,audit:r.try_get("audit")?,audit_event_id:r.try_get("audit_event_id")?,transaction_id:r.try_get("transaction_id")?})}).transpose()?;
   commit_semantic_transaction(tx,deadline).await?;Ok(p)
  })).await
    }
    pub async fn t94_ack_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        event: &str,
        hash: &[u8],
    ) -> Result<bool> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        let event = event.to_owned();
        let hash = hash.to_vec();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(_c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(false)};
   let changed=sqlx::query("UPDATE game_t94_publications SET audit_acknowledged=true WHERE audit_event_id=$1::uuid AND sha256(audit)=$2 AND NOT audit_acknowledged").bind(event).bind(hash).execute(&mut *tx).await?.rows_affected()==1;
   commit_semantic_transaction(tx,deadline).await?;Ok(changed)
  })).await
    }
}

pub struct ProducerStore<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub world: crate::foundation::WorldId,
    pub defaults: wire::ProducerDefaults,
    pub source_authority: String,
    pub epoch_fence_file: std::path::PathBuf,
}
impl oteryn_gameplay_native_admission_source::world_event_outcomes::publisher::LeagueStore
    for ProducerStore<'_, '_, '_>
{
    async fn current(
        &self,
        family: &str,
        epoch: u64,
        token: &str,
    ) -> std::result::Result<bool, ()> {
        self.root
            .t94_delivery_current(self.authority, family, epoch, token)
            .await
            .map_err(|_| ())
    }

    async fn produce(&self) -> std::result::Result<(), ()> {
        let mut fence = wire::fence::LeagueEpochFenceFile::new(self.epoch_fence_file.clone());
        self.root
            .t94_initialize(self.authority, &self.source_authority, &mut fence)
            .await
            .map_err(|_| ())?;
        self.root
            .t94_start_world(
                self.authority,
                self.world,
                &self.defaults,
                &self.source_authority,
            )
            .await
            .map_err(|_| ())?;
        self.root
            .t94_produce(self.authority)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
    async fn next(&self, family: &str) -> std::result::Result<Option<Publication>, ()> {
        self.root
            .t94_claim(self.authority, family)
            .await
            .map_err(|_| ())
    }
    async fn failed(
        &self,
        p: &Publication,
        error: oteryn_gameplay_native_admission_source::runtime_status::NotDelivered,
    ) -> std::result::Result<(), ()> {
        self.root
            .t94_failed(
                self.authority,
                p,
                matches!(
                    error,
                    oteryn_gameplay_native_admission_source::runtime_status::NotDelivered::Conflict
                ),
            )
            .await
            .map_err(|_| ())
    }
    async fn release_watermark(&self, w: &wire::WatermarkDelivery) -> std::result::Result<(), ()> {
        let p = Publication {
            id: String::new(),
            delivery_token: Some(w.token.clone()),
            family: w.watermark.family.clone(),
            epoch: 0,
            revision: 0,
            wire: String::new(),
            sha256: Vec::new(),
            audit: Vec::new(),
            audit_event_id: String::new(),
            transaction_id: String::new(),
        };
        self.root
            .t94_release(self.authority, &p)
            .await
            .map_err(|_| ())
    }
    async fn ack(&self, p: &Publication) -> std::result::Result<(), ()> {
        if !self.root.t94_ack(self.authority, p).await.map_err(|_| ())? {
            return Err(());
        };
        self.root
            .t94_release(self.authority, p)
            .await
            .map_err(|_| ())
    }
    async fn watermark(
        &self,
        family: &str,
    ) -> std::result::Result<Option<wire::WatermarkDelivery>, ()> {
        self.root
            .t94_claim_watermark(self.authority, family)
            .await
            .map_err(|_| ())
    }
}

impl DurabilityRoot {
    pub async fn t94_claim(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        family: &str,
    ) -> Result<Option<Publication>> {
        let Some(mut p) = self.t94_next(authority, family).await? else {
            return Ok(None);
        };
        let recovery = authority.record_for(self).map_err(invalid)?;
        let family = family.to_owned();
        let id = p.id.clone();
        let epoch = p.epoch.to_string();
        let token=self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(None)};
   if c.epoch.to_string()!=epoch{return Ok(None)}
   let token:Option<String>=sqlx::query_scalar("UPDATE game_t94_delivery_lanes SET token=uuidv7(),lease_until=clock_timestamp()+interval '10 seconds' WHERE family=$1 AND (lease_until IS NULL OR lease_until<=clock_timestamp()) AND EXISTS(SELECT 1 FROM game_t94_publications p WHERE p.publication_id=$2::uuid AND NOT p.acknowledged) RETURNING token::text")
    .bind(family).bind(id).fetch_optional(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(token)
  })).await?;
        let Some(token) = token else { return Ok(None) };
        p.delivery_token = Some(token);
        Ok(Some(p))
    }
    pub async fn t94_release(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        publication: &Publication,
    ) -> Result<()> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        let p = publication.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   control(&mut tx,recovery.recovery_generation).await?.ok_or(DurabilityError::Unavailable)?;
   sqlx::query("UPDATE game_t94_delivery_lanes SET lease_until=NULL,token=NULL WHERE family=$1 AND token=$2::uuid").bind(p.family).bind(p.delivery_token).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}

impl DurabilityRoot {
    pub async fn t94_claim_watermark(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        family: &str,
    ) -> Result<Option<wire::WatermarkDelivery>> {
        let Some(watermark) = self.t94_watermark(authority, family).await? else {
            return Ok(None);
        };
        let recovery = authority.record_for(self).map_err(invalid)?;
        let family = family.to_owned();
        let epoch = watermark.projection_epoch.clone();
        let token=self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(None)};
   if c.epoch.to_string()!=epoch{return Ok(None)}
   let token:Option<String>=sqlx::query_scalar("UPDATE game_t94_delivery_lanes SET token=uuidv7(),lease_until=clock_timestamp()+interval '10 seconds' WHERE family=$1 AND (lease_until IS NULL OR lease_until<=clock_timestamp()) RETURNING token::text").bind(family).fetch_optional(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(token)
  })).await?;
        Ok(token.map(|token| wire::WatermarkDelivery { watermark, token }))
    }
    pub async fn t94_failed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        p: &Publication,
        conflict: bool,
    ) -> Result<()> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        let p = p.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   control(&mut tx,recovery.recovery_generation).await?.ok_or(DurabilityError::Unavailable)?;
   if conflict{
    sqlx::query("UPDATE game_t94_control SET quarantined=true WHERE singleton").execute(&mut *tx).await?;
    eprintln!("family=t94 operation=publish result=PLATFORM_CONFLICT timing_ms=0");
   }
   sqlx::query("UPDATE game_t94_delivery_lanes SET token=NULL,lease_until=NULL WHERE family=$1 AND token=$2::uuid").bind(p.family).bind(p.delivery_token).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}

impl DurabilityRoot {
    pub async fn t94_delivery_current(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        family: &str,
        epoch: u64,
        token: &str,
    ) -> Result<bool> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        let family = family.to_owned();
        let token = token.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
 let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
 let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(false)};
 if c.epoch!=epoch{return Ok(false)}
 let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_delivery_lanes WHERE family=$1 AND token=$2::uuid AND lease_until>=clock_timestamp()+interval '6 seconds')").bind(family).bind(token).fetch_one(&mut *tx).await?;
 commit_semantic_transaction(tx,deadline).await?;Ok(current)
 })).await
    }
}
