use super::*;
use crate::native_admission_source::{account_closure::Publication, descriptor::Operation};
use wire::digest;
fn publication_identity(p: &Publication) -> Result<Option<String>, DurabilityError> {
    let field = match p.operation {
        Operation::PublishAccountClosureCommandReceiptV2 => "operation_id",
        Operation::PublishAccountClosureV1 => "account_id",
        Operation::PublishAccountClosureWatermarkV1 if p.body.len() <= 512 => return Ok(None),
        _ => return Err(DurabilityError::Unavailable),
    };
    if p.body.len() > 2048 {
        return Err(DurabilityError::Unavailable);
    }
    let body: serde_json::Value =
        serde_json::from_str(&p.body).map_err(|_| DurabilityError::Unavailable)?;
    let identity = body[field]
        .as_str()
        .filter(|s| uuid_v7(s))
        .ok_or(DurabilityError::Unavailable)?;
    Ok(Some(identity.to_owned()))
}
#[derive(Serialize)]
struct Closure<'a> {
    termination_id: &'a str,
    operation_id: &'a str,
    lifecycle_generation: &'a str,
    closure_state: ClosureState,
    receipt_digest: String,
    proof_sha256: &'a str,
}
#[derive(Serialize)]
struct Snapshot<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    account_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    closures: Vec<Closure<'a>>,
}
pub(super) async fn enqueue(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    d: &ResultData,
    receipt: &[u8],
    now: u64,
) -> Result<(), DurabilityError> {
    let epoch:i64=sqlx::query_scalar("SELECT projection_epoch FROM game_account_closure_publication_clock WHERE singleton FOR SHARE").fetch_one(&mut **tx).await?;
    let revision:i64=sqlx::query_scalar("SELECT coalesce((SELECT projection_revision FROM game_account_closure_publications WHERE account_id=$1::uuid),0)+1").bind(&d.account_id).fetch_one(&mut **tx).await?;
    let body = serde_json::to_vec(&Snapshot {
        contract_version: 2,
        operation: "PublishAccountClosureV1",
        source_authority: r.source_authority,
        account_id: &d.account_id,
        projection_epoch: epoch.to_string(),
        projection_revision: revision.to_string(),
        source_observed_at: now.to_string(),
        closures: vec![Closure {
            termination_id: &d.termination_id,
            operation_id: r.operation_id,
            lifecycle_generation: &d.lifecycle_generation,
            closure_state: d.closure_state,
            receipt_digest: digest(receipt),
            proof_sha256: d
                .proof_sha256
                .as_deref()
                .ok_or(DurabilityError::InvalidStoredState)?,
        }],
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    if body.len() > 2048 {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("INSERT INTO game_account_closure_publications(account_id,projection_epoch,projection_revision,receipt_operation_id,body,created_at) VALUES($1::uuid,$2,$3,$4::uuid,$5,$6) ON CONFLICT(account_id) DO UPDATE SET projection_epoch=EXCLUDED.projection_epoch,projection_revision=EXCLUDED.projection_revision,receipt_operation_id=EXCLUDED.receipt_operation_id,body=EXCLUDED.body,created_at=EXCLUDED.created_at,acknowledged=false,quarantined=false")
 .bind(&d.account_id).bind(epoch).bind(revision).bind(r.operation_id).bind(body).bind(now as i64).execute(&mut **tx).await?;
    Ok(())
}

#[derive(Serialize)]
struct Watermark<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    family: &'static str,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}
impl DurabilityRoot {
    pub async fn next_account_closure_publication(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        source: &str,
    ) -> Result<Option<Publication>, DurabilityError> {
        if !source_authority_valid(source) {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        let source = source.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}

   sqlx::query("DELETE FROM game_account_closure_audit_outbox WHERE event_id IN (SELECT event_id FROM game_account_closure_audit_outbox WHERE NOT legal_hold AND expires_at<=game_account_closure_now()*1000 ORDER BY expires_at,event_id LIMIT 64 FOR UPDATE SKIP LOCKED)").execute(&mut *tx).await?;
   let clock=sqlx::query("SELECT qualified_recovery_generation::text,projection_epoch,resync_after::text,resync_pending FROM game_account_closure_publication_clock WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
   if clock.try_get::<String,_>("qualified_recovery_generation")?!=recovery.recovery_generation.to_string() {commit_semantic_transaction(tx,deadline).await?;return Ok(None);}
   if clock.try_get::<bool,_>("resync_pending")? {
    let epoch:i64=clock.try_get("projection_epoch")?;
    let accounts:Vec<String>=sqlx::query_scalar("SELECT account_id::text FROM game_account_closure_publications WHERE ($1::text IS NULL OR account_id>$1::uuid) ORDER BY account_id LIMIT 32")
     .bind(clock.try_get::<Option<String>,_>("resync_after")?).fetch_all(&mut *tx).await?;
    for account in &accounts {
     sqlx::query("UPDATE game_account_closure_jobs SET receipt_acknowledged=false,receipt_quarantined=false WHERE operation_id IN(SELECT receipt_operation_id FROM game_account_closure_publications WHERE account_id=$1::uuid)").bind(account).execute(&mut *tx).await?;
     sqlx::query(r#"UPDATE game_account_closure_publications SET projection_epoch=$2,acknowledged=false,quarantined=false,body=convert_to(regexp_replace(convert_from(body,'UTF8'),'"projection_epoch":"[0-9]+"','"projection_epoch":"'||$2::text||'"'),'UTF8') WHERE account_id=$1::uuid"#)
      .bind(account).bind(epoch).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE game_account_closure_publication_clock SET resync_after=$1::uuid,resync_pending=$2 WHERE singleton")
     .bind(accounts.last()).bind(accounts.len()==32).execute(&mut *tx).await?;
    commit_semantic_transaction(tx,deadline).await?;return Ok(None);
   }
   // Historical receipts remain byte-identical, including across recovery. They
   // do not reacquire admission authority or by themselves certify current proof.
   if let Some(receipt)=sqlx::query_scalar::<_,Vec<u8>>("SELECT receipt FROM game_account_closure_jobs WHERE receipt IS NOT NULL AND NOT receipt_acknowledged AND NOT receipt_quarantined ORDER BY decided_at,operation_id LIMIT 1").fetch_optional(&mut *tx).await? {
    let body=String::from_utf8(receipt).map_err(|_|DurabilityError::InvalidStoredState)?;
    commit_semantic_transaction(tx,deadline).await?;return Ok(Some(Publication{operation:Operation::PublishAccountClosureCommandReceiptV2,body}));
   }
   let next=sqlx::query("SELECT p.body FROM game_account_closure_publications p JOIN game_account_closure_jobs j ON j.operation_id=p.receipt_operation_id WHERE NOT p.acknowledged AND NOT p.quarantined AND j.receipt_acknowledged AND j.recovery_generation=$1::numeric ORDER BY p.created_at,p.account_id LIMIT 1")
    .bind(recovery.recovery_generation.to_string()).fetch_optional(&mut *tx).await?;
   if let Some(row)=next{
    let raw:Vec<u8>=row.try_get("body")?;let body=String::from_utf8(raw).map_err(|_|DurabilityError::InvalidStoredState)?;
    commit_semantic_transaction(tx,deadline).await?;return Ok(Some(Publication{operation:Operation::PublishAccountClosureV1,body}));
   }
   let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_jobs WHERE receipt IS NOT NULL AND NOT receipt_acknowledged) OR EXISTS(SELECT 1 FROM game_account_closure_publications WHERE NOT acknowledged) OR EXISTS(SELECT 1 FROM game_character_account_projection_outbox) OR EXISTS(SELECT 1 FROM game_public_character_profile_outbox) OR EXISTS(SELECT 1 FROM game_public_highscore_outbox)")
    .fetch_one(&mut *tx).await?;
   if blocked{commit_semantic_transaction(tx,deadline).await?;return Ok(None);}
   let clock=sqlx::query("SELECT projection_epoch,watermark_at,quarantined,game_account_closure_now() AS now FROM game_account_closure_publication_clock WHERE singleton").fetch_one(&mut *tx).await?;
   if clock.try_get::<bool,_>("quarantined")? {commit_semantic_transaction(tx,deadline).await?;return Ok(None);}
   let now:i64=clock.try_get("now")?;if now-clock.try_get::<i64,_>("watermark_at")?<5{commit_semantic_transaction(tx,deadline).await?;return Ok(None);}
   let cut=now.saturating_sub(super::super::super::account_characters_projection::MAX_CHARACTER_TRANSACTION.as_secs() as i64);
   let body=serde_json::to_string(&Watermark{contract_version:2,operation:"PublishProjectionWatermarkV1",source_authority:&source,family:"account_closure",projection_epoch:clock.try_get::<i64,_>("projection_epoch")?.to_string(),complete_through:cut.to_string(),observed_at:now.to_string()}).map_err(|_|DurabilityError::Unavailable)?;
   if body.len()>512{return Err(DurabilityError::Unavailable);}
   commit_semantic_transaction(tx,deadline).await?;Ok(Some(Publication{operation:Operation::PublishAccountClosureWatermarkV1,body}))
  })).await
    }
    pub async fn acknowledge_account_closure(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        receipt_operation: Option<&str>,
        publication: Option<&Publication>,
    ) -> Result<(), DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        let operation = receipt_operation.map(str::to_owned);
        let identity = publication.map(publication_identity).transpose()?.flatten();
        let publication = publication.cloned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   if let Some(operation)=operation {if !uuid_v7(&operation){return Err(DurabilityError::Unavailable);}
    sqlx::query("UPDATE game_account_closure_jobs SET receipt_acknowledged=true WHERE operation_id=$1::uuid AND receipt IS NOT NULL").bind(operation).execute(&mut *tx).await?;
   }
   if let Some(publication)=publication{match publication.operation{
    Operation::PublishAccountClosureCommandReceiptV2=>{
     sqlx::query("UPDATE game_account_closure_jobs SET receipt_acknowledged=true WHERE operation_id=$2::uuid AND receipt=$1").bind(publication.body.as_bytes()).bind(identity.as_deref().ok_or(DurabilityError::Unavailable)?).execute(&mut *tx).await?;
    }
    Operation::PublishAccountClosureV1=>{sqlx::query("UPDATE game_account_closure_publications SET acknowledged=true WHERE account_id=$2::uuid AND body=$1").bind(publication.body.as_bytes()).bind(identity.as_deref().ok_or(DurabilityError::Unavailable)?).execute(&mut *tx).await?;}
    Operation::PublishAccountClosureWatermarkV1=>{
     let v:serde_json::Value=serde_json::from_str(&publication.body).map_err(|_|DurabilityError::Unavailable)?;
     let now=v["observed_at"].as_str().and_then(counter).ok_or(DurabilityError::Unavailable)?;
     let epoch=v["projection_epoch"].as_str().and_then(counter).ok_or(DurabilityError::Unavailable)?;
     sqlx::query("UPDATE game_account_closure_publication_clock SET watermark_at=greatest(watermark_at,$1) WHERE singleton AND projection_epoch=$2 AND $1<=floor(extract(epoch FROM statement_timestamp()))::bigint").bind(now as i64).bind(epoch as i64).execute(&mut *tx).await?;
    }
    _=>return Err(DurabilityError::Unavailable),
   }}
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }

    /// The actor owner calls this only after exact runtime retirement. Durable
    /// claims are independently checked again here; a kick request is never proof.
    pub async fn acknowledge_account_closure_offline(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        absence: oteryn_gameplay_entities::carrier::RuntimeSessionAbsence,
    ) -> Result<(), DurabilityError> {
        let session = *absence.session().as_bytes();
        let generation = absence.generation();
        let (kind, world, channel, instance) = match absence.scope() {
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => (
                1i16,
                *world_id.as_bytes(),
                Some(*channel_id.as_bytes()),
                None,
            ),
            crate::foundation::RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } => (2i16, *world_id.as_bytes(), None, Some(instance_id)),
        };
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   let relevant:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_account_closures c ON c.account_id=s.account_id WHERE s.game_session_id=encode($1,'hex')::uuid AND c.state IN('CLOSING','CANCELLING'))").bind(session.as_slice()).fetch_one(&mut *tx).await?;
   if !relevant {commit_semantic_transaction(tx,deadline).await?;return Ok(());}
   let already:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_offline o JOIN game_durability_reconnect_sessions s USING(game_session_id) WHERE o.game_session_id=encode($1,'hex')::uuid AND o.character_id=s.character_id AND o.lease_generation=s.character_lease_generation AND s.session_state=3)").bind(session.as_slice()).fetch_one(&mut *tx).await?;
   if !already {
    let inserted=sqlx::query("INSERT INTO game_account_closure_offline(game_session_id,character_id,node_id,lease_generation,acknowledged_at) SELECT s.game_session_id,s.character_id,$2::uuid,s.character_lease_generation,floor(extract(epoch FROM statement_timestamp()))::bigint FROM game_durability_reconnect_sessions s JOIN game_runtime_scope_assignments r ON r.scope_kind=$3 AND r.world_id=encode($4,'hex')::uuid AND r.channel_id IS NOT DISTINCT FROM (CASE WHEN $5::bytea IS NULL THEN NULL ELSE encode($5,'hex')::uuid END) AND (CASE WHEN r.scope_kind=2 THEN game_house_scope_instance_id(r.world_id,r.house_key) ELSE NULL END) IS NOT DISTINCT FROM (CASE WHEN $6::bytea IS NULL THEN NULL ELSE encode($6,'hex')::uuid END) AND r.state=1 AND r.holder_node_id=$2::uuid AND r.holder_registration_revision=$7::numeric AND r.ownership_generation=$8::numeric WHERE s.game_session_id=encode($1,'hex')::uuid AND s.session_state=3 AND s.runtime_scope_kind=r.scope_kind AND s.runtime_scope_world_id=r.world_id AND s.runtime_scope_channel_id IS NOT DISTINCT FROM r.channel_id AND s.runtime_scope_instance_id IS NOT DISTINCT FROM (CASE WHEN r.scope_kind=2 THEN game_house_scope_instance_id(r.world_id,r.house_key) ELSE NULL END) AND NOT EXISTS(SELECT 1 FROM game_durability_admission_character_guards c WHERE c.character_id=s.character_id AND c.holder_game_session_id IS NOT NULL) AND NOT EXISTS(SELECT 1 FROM game_durability_admission_account_guards c WHERE c.account_id=s.account_id AND c.presence_character_id=s.character_id) ON CONFLICT(game_session_id) DO NOTHING")
     .bind(session.as_slice()).bind(uuid_string(p.fact().node_id().as_bytes())).bind(kind).bind(world.as_slice()).bind(channel.as_ref().map(|v|v.as_slice())).bind(instance.as_ref().map(|v|v.as_slice())).bind(p.fact().registration_revision().to_string()).bind(generation.to_string()).execute(&mut *tx).await?;
    if inserted.rows_affected()!=1{return Err(DurabilityError::Unavailable);}
   }
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
    pub async fn account_closure_requested(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let closed = sqlx::query_scalar(
                        "SELECT game_account_closure_closed(encode($1,'hex')::uuid)",
                    )
                    .bind(account.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(closed)
                })
            })
            .await
    }
}
impl DurabilityRoot {
    pub async fn observe_account_closure_lifecycle(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        request: &crate::native_admission_source::account_closure::LifecycleRequest,
        raw: &[u8],
    ) -> Result<(), DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        if request.recovery_generation != recovery.recovery_generation.to_string() {
            return Err(DurabilityError::Unavailable);
        }
        let o = crate::native_admission_source::account_closure::decode_lifecycle(request, raw)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   let now:i64=sqlx::query_scalar("SELECT game_account_closure_now()").fetch_one(&mut *tx).await?;
   let observed=counter(&o.observed_at).ok_or(DurabilityError::Unavailable)? as i64;
   let until=counter(&o.valid_until).ok_or(DurabilityError::Unavailable)? as i64;
   if observed>now || until<=now{return Err(DurabilityError::Unavailable);}
   sqlx::query("INSERT INTO game_account_closure_lifecycle(account_id,termination_id,lifecycle_generation,source_revision,phase,recovery_generation,observed_at,valid_until,financial_obligations_clear) VALUES($1::uuid,$2::uuid,$3::numeric,$4::numeric,$5,$6::numeric,$7,$8,$9) ON CONFLICT(account_id) DO UPDATE SET usable=true,financial_obligations_clear=EXCLUDED.financial_obligations_clear,termination_id=EXCLUDED.termination_id,lifecycle_generation=EXCLUDED.lifecycle_generation,source_revision=EXCLUDED.source_revision,phase=EXCLUDED.phase,recovery_generation=EXCLUDED.recovery_generation,observed_at=EXCLUDED.observed_at,valid_until=EXCLUDED.valid_until WHERE EXCLUDED.source_revision>game_account_closure_lifecycle.source_revision OR (EXCLUDED.source_revision=game_account_closure_lifecycle.source_revision AND (EXCLUDED.termination_id,EXCLUDED.lifecycle_generation,EXCLUDED.phase,EXCLUDED.recovery_generation,EXCLUDED.observed_at,EXCLUDED.valid_until,EXCLUDED.financial_obligations_clear) IS NOT DISTINCT FROM (game_account_closure_lifecycle.termination_id,game_account_closure_lifecycle.lifecycle_generation,game_account_closure_lifecycle.phase,game_account_closure_lifecycle.recovery_generation,game_account_closure_lifecycle.observed_at,game_account_closure_lifecycle.valid_until,game_account_closure_lifecycle.financial_obligations_clear))")
    .bind(o.account_id).bind(o.termination_id).bind(o.lifecycle_generation).bind(o.source_revision).bind(o.phase).bind(o.recovery_generation).bind(observed).bind(until).bind(o.financial_obligations_clear).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}
impl DurabilityRoot {
    pub async fn next_account_closure_terminal(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        scope: ([u8; 16], [u8; 16]),
    ) -> Result<Vec<([u8; 16], crate::foundation::RuntimeScopeRefV1)>, DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   let rows=sqlx::query("SELECT s.game_session_id::text,s.runtime_scope_kind,s.runtime_scope_world_id::text,s.runtime_scope_channel_id::text,s.runtime_scope_instance_id::text FROM game_durability_reconnect_sessions s JOIN game_account_closure_members m ON m.character_id=s.character_id JOIN game_account_closures c ON c.account_id=m.account_id AND c.close_operation_id=m.close_operation_id JOIN game_runtime_scope_assignments r ON r.scope_kind=s.runtime_scope_kind AND r.world_id=s.runtime_scope_world_id AND r.channel_id IS NOT DISTINCT FROM s.runtime_scope_channel_id AND (CASE WHEN r.scope_kind=2 THEN game_house_scope_instance_id(r.world_id,r.house_key) ELSE NULL END) IS NOT DISTINCT FROM s.runtime_scope_instance_id AND r.state=1 WHERE c.state IN('CLOSING','CANCELLING') AND m.step=0 AND s.session_state=3 AND s.character_lease_generation=(SELECT max(s2.character_lease_generation) FROM game_durability_reconnect_sessions s2 WHERE s2.character_id=s.character_id) AND r.world_id=encode($1,'hex')::uuid AND (r.scope_kind=2 OR r.channel_id=encode($2,'hex')::uuid) AND r.holder_node_id=$3::uuid AND r.holder_registration_revision=$4::numeric AND NOT EXISTS(SELECT 1 FROM game_account_closure_offline o WHERE o.game_session_id=s.game_session_id) ORDER BY s.game_session_id LIMIT 32")
    .bind(scope.0.as_slice()).bind(scope.1.as_slice()).bind(uuid_string(p.fact().node_id().as_bytes())).bind(p.fact().registration_revision().to_string()).fetch_all(&mut *tx).await?;
   let mut sessions=Vec::with_capacity(rows.len());
   for row in rows {
    let world=crate::foundation::WorldId::decode(&uuid_text(&row.try_get::<String,_>("runtime_scope_world_id")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
    let scope=match row.try_get::<i16,_>("runtime_scope_kind")? {
     1=>crate::foundation::RuntimeScopeRefV1::Channel{world_id:world,channel_id:crate::foundation::ChannelId::decode(&uuid_text(&row.try_get::<String,_>("runtime_scope_channel_id")?)?).map_err(|_|DurabilityError::InvalidStoredState)?},
     2=>crate::foundation::RuntimeScopeRefV1::Instance{world_id:world,instance_id:uuid_text(&row.try_get::<String,_>("runtime_scope_instance_id")?)?},
     _=>return Err(DurabilityError::InvalidStoredState),
    };
    sessions.push((uuid_text(&row.try_get::<String,_>("game_session_id")?)?,scope));
   }
   commit_semantic_transaction(tx,deadline).await?;Ok(sessions)
  })).await
    }
}

#[derive(Debug, Clone)]
pub struct AuditDelivery {
    pub event_id: [u8; 16],
    pub envelope: Vec<u8>,
}
impl DurabilityRoot {
    pub async fn pending_account_closure_audit(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        limit: u16,
    ) -> Result<Vec<AuditDelivery>, DurabilityError> {
        if limit == 0 || limit > 64 {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   use prost::Message;
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let rows=sqlx::query("SELECT event_id::text,envelope,payload_sha256 FROM game_account_closure_audit_outbox WHERE publication_state=1 ORDER BY occurred_at,event_id LIMIT $1").bind(i64::from(limit)).fetch_all(&mut *tx).await?;
   let mut events=Vec::with_capacity(rows.len());
   for row in rows {
    let event_id=uuid_text(row.try_get("event_id")?)?;let envelope:Vec<u8>=row.try_get("envelope")?;
    let decoded=crate::durability::item_mint_audit::EventEnvelopeV1::decode(envelope.as_slice()).map_err(|_|DurabilityError::InvalidStoredState)?;
    let hash:Vec<u8>=row.try_get("payload_sha256")?;
    if decoded.event_id!=event_id||decoded.event_type_id!=7||decoded.event_schema_revision!=1||decoded.payload_sha256!=hash||Sha256::digest(&decoded.payload).as_slice()!=hash {return Err(DurabilityError::InvalidStoredState);}
    events.push(AuditDelivery{event_id,envelope});
   }
   commit_semantic_transaction(tx,deadline).await?;Ok(events)
  })).await
    }
    pub async fn acknowledge_account_closure_audit(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        event: [u8; 16],
    ) -> Result<(), DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   sqlx::query("UPDATE game_account_closure_audit_outbox SET publication_state=2 WHERE event_id=encode($1,'hex')::uuid").bind(event.as_slice()).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
    pub async fn quarantine_account_closure_publication(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        publication: &Publication,
    ) -> Result<(), DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        let identity = publication_identity(publication)?;
        let operation = publication.operation;
        let body = publication.body.as_bytes().to_vec();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   match operation {
    Operation::PublishAccountClosureCommandReceiptV2=>{sqlx::query("UPDATE game_account_closure_jobs SET receipt_quarantined=true WHERE operation_id=$2::uuid AND receipt=$1 AND NOT receipt_acknowledged").bind(&body).bind(identity.as_deref().ok_or(DurabilityError::Unavailable)?).execute(&mut *tx).await?;}
    Operation::PublishAccountClosureV1=>{sqlx::query("UPDATE game_account_closure_publications SET quarantined=true WHERE account_id=$2::uuid AND body=$1 AND NOT acknowledged").bind(&body).bind(identity.as_deref().ok_or(DurabilityError::Unavailable)?).execute(&mut *tx).await?;}
    Operation::PublishAccountClosureWatermarkV1=>{
     let watermark:serde_json::Value=serde_json::from_slice(&body).map_err(|_|DurabilityError::Unavailable)?;
     let epoch=watermark["projection_epoch"].as_str().and_then(counter).and_then(|v|i64::try_from(v).ok()).ok_or(DurabilityError::Unavailable)?;
     sqlx::query("UPDATE game_account_closure_publication_clock SET quarantined=true WHERE singleton AND projection_epoch=$1").bind(epoch).execute(&mut *tx).await?;
    }
    _=>return Err(DurabilityError::Unavailable),
   }
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}
impl DurabilityRoot {
    pub async fn invalidate_account_closure_lifecycle(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
        p: &NodeIncarnationProof,
        account: &str,
        termination: &str,
    ) -> Result<(), DurabilityError> {
        if !uuid_v7(account) || !uuid_v7(termination) {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let p = p.clone();
        let account = account.to_owned();
        let termination = termination.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&p).await?{return Err(DurabilityError::Unavailable);}
   sqlx::query("UPDATE game_account_closure_lifecycle SET usable=false WHERE account_id=$1::uuid AND termination_id=$2::uuid")
    .bind(account).bind(termination).execute(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;Ok(())
  })).await
    }
}

impl DurabilityRoot {
    /// P90D expiry runs from the existing node audit owner even if Platform is
    /// unavailable. Legal holds and authoritative replay state remain separate.
    pub async fn expire_account_closure_audit(
        &self,
        a: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<u64, DurabilityError> {
        let recovery = a
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let result=sqlx::query("DELETE FROM game_account_closure_audit_outbox WHERE event_id IN (SELECT event_id FROM game_account_closure_audit_outbox WHERE NOT legal_hold AND expires_at<=game_account_closure_now()*1000 ORDER BY expires_at,event_id LIMIT 64 FOR UPDATE SKIP LOCKED)").execute(&mut *tx).await?;
            let deleted=result.rows_affected();
            commit_semantic_transaction(tx,deadline).await?;
            Ok(deleted)
        })).await
    }
}
