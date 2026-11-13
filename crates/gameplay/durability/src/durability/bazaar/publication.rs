//! Bounded durable receipt and full-subject projection publication. Empty feeds
//! never certify completeness: a retained enumeration cursor must finish first.
use super::*;
use crate::native_admission_source::{
    bazaar::transport::{
        DeliveryKey, PublicationDelivery, ReceiptDelivery, RemoteState, TransportBatch,
    },
    descriptor::Operation,
};

/// A historical signature cannot certify a replacement projection floor.
fn snapshot_projection_current(raw: &[u8], policy: &Policy, source: &str, epoch: u64) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(raw) else {
        return false;
    };
    let snapshot = &value["snapshot"];
    policy.snapshot_source_authority.as_deref() == Some(source)
        && snapshot["source_authority"].as_str() == Some(source)
        && snapshot["projection_epoch"].as_str() == Some(epoch.to_string().as_str())
}

fn receipt_signed_snapshot(raw: &[u8]) -> Result<Option<Vec<u8>>, DurabilityError> {
    #[derive(Deserialize)]
    struct ReceiptData {
        result_data: Option<Box<RawValue>>,
    }
    #[derive(Deserialize)]
    struct LockData {
        signed_snapshot: Box<RawValue>,
    }
    let receipt: ReceiptData =
        serde_json::from_slice(raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    receipt
        .result_data
        .map(|data| {
            let data: LockData = serde_json::from_str(data.get())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            Ok(data.signed_snapshot.get().as_bytes().to_vec())
        })
        .transpose()
}

pub(super) async fn listing_changed(
    tx: &mut Tx<'_>,
    r: &Request,
    v: &serde_json::Value,
    now: i64,
) -> Result<(), DurabilityError> {
    if matches!(
        r.command.as_str(),
        "HoldBazaarGold" | "ReleaseBazaarGold" | "PayoutBazaarGold"
    ) {
        return Ok(());
    }
    let listing = if let Some(listing) = v.get("listing_id").and_then(serde_json::Value::as_str) {
        Some(listing.to_owned())
    } else if let Some(character) = v.get("character_id").and_then(serde_json::Value::as_str) {
        sqlx::query_scalar(
            "SELECT listing_id::text FROM game_bazaar_locks WHERE character_id=$1::uuid AND state IN('locked','INVALID') LIMIT 1",
        )
        .bind(character)
        .fetch_optional(&mut **tx)
        .await?
    } else {
        None
    };
    if let Some(listing) = listing {
        sqlx::query("INSERT INTO game_bazaar_publication_outbox(subject,epoch,revision,body,committed_at,operation,enumeration_version) VALUES('listing:'||$1,$2::numeric,1,convert_to('{}','UTF8'),$3,'PublishBazaarListingStatusV1',coalesce((SELECT version FROM game_bazaar_projection_state WHERE singleton),0)) ON CONFLICT(subject) DO UPDATE SET enumeration_version=coalesce((SELECT version FROM game_bazaar_projection_state WHERE singleton),0),epoch=EXCLUDED.epoch,revision=game_bazaar_publication_outbox.revision+1,body=EXCLUDED.body,committed_at=EXCLUDED.committed_at,acknowledged=false")
            .bind(listing).bind(r.generation.to_string()).bind(now).execute(&mut **tx).await?;
    }
    Ok(())
}

#[derive(Serialize)]
struct Status<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    listing_id: &'a str,
    seller_account_id: String,
    world_id: String,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    asset_kind: &'static str,
    asset_id: String,
    state: String,
    custody_id: String,
    custody_revision: String,
    snapshot_hash: Option<String>,
    last_operation_id: Option<String>,
    reason_code: Option<&'static str>,
}
#[derive(Serialize)]
struct Transferability<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    eligibility_revision: String,
    definitions: &'a RawValue,
}
#[derive(Serialize)]
struct Watermark<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}

async fn build(
    tx: &mut Tx<'_>,
    p: &Policy,
    source: &str,
    subject: &str,
    epoch: u64,
    revision: &str,
    observed: i64,
) -> Result<Option<Vec<u8>>, DurabilityError> {
    if let Some(world) = subject.strip_prefix("policy:") {
        let Some(eligibility) = p.obligations_revision.filter(|r| *r > 0) else {
            return Ok(None);
        };
        let Some(definitions) = items::transferability(tx, p, world).await? else {
            return Ok(None);
        };
        let definitions =
            RawValue::from_string(definitions).map_err(|_| DurabilityError::InvalidStoredState)?;
        return serde_json::to_vec(&Transferability {
            contract_version: 2,
            operation: "PublishBazaarTransferabilityV1",
            source_authority: source,
            world_id: world,
            projection_epoch: epoch.to_string(),
            projection_revision: revision.to_owned(),
            source_observed_at: observed.to_string(),
            eligibility_revision: eligibility.to_string(),
            definitions: &definitions,
        })
        .map(Some)
        .map_err(|_| DurabilityError::Unavailable);
    }
    let listing = subject
        .strip_prefix("listing:")
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (row,character)=if let Some(row)=sqlx::query("SELECT seller_account_id::text,world_id::text,character_id::text AS asset_id,state,lock_id::text AS custody_id,revision::text AS custody_revision,snapshot_hash,signed_snapshot,last_operation_id::text FROM game_bazaar_locks WHERE listing_id=$1::uuid FOR SHARE").bind(listing).fetch_optional(&mut **tx).await?{(row,true)}
        else if let Some(row)=sqlx::query("SELECT seller_account_id::text,world_id::text,item_instance_id::text AS asset_id,state,escrow_id::text AS custody_id,revision::text AS custody_revision,NULL::bytea AS snapshot_hash,NULL::bytea AS signed_snapshot,last_operation_id::text FROM game_bazaar_escrows WHERE listing_id=$1::uuid FOR SHARE").bind(listing).fetch_optional(&mut **tx).await?{(row,false)}else{return Ok(None);};
    let mut state: String = row.try_get("state")?;
    if character && state == "locked" {
        let last: String = row.try_get("last_operation_id")?;
        if let Some(purpose)=sqlx::query_scalar::<_,String>("SELECT convert_from(intent,'UTF8')::jsonb->>'purpose' FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid AND command='TransferCharacterOwnership' AND result_code='CHAR_CMD_OK'").bind(CHARACTER_ISSUER).bind(last).fetch_optional(&mut **tx).await? {
            state=match purpose.as_str(){"BAZAAR_SETTLEMENT"=>"DELIVERED".into(),"BAZAAR_ESCROW_RETURN"=>"RETURNED".into(),_=>state};
        }
    }
    let mut reason = (state == "INVALID").then_some("STATE_CHANGED");
    if character && !matches!(state.as_str(), "UNLOCKED" | "DELIVERED" | "RETURNED") {
        let signed = row.try_get::<Vec<u8>, _>("signed_snapshot")?;
        let value: serde_json::Value =
            serde_json::from_slice(&signed).map_err(|_| DurabilityError::InvalidStoredState)?;
        let snapshot = &value["snapshot"];
        if snapshot["projection_epoch"].as_str() != Some(epoch.to_string().as_str()) {
            state = "INVALID".into();
            reason = Some("RESTORE_RECONCILING");
        } else if !snapshot_projection_current(&signed, p, source, epoch)
            || snapshot["eligibility_revision"].as_str()
                != p.obligations_revision.map(|v| v.to_string()).as_deref()
        {
            state = "INVALID".into();
            reason = Some("POLICY_CHANGED");
        } else if !characters::signed_snapshot_trusted(tx, p, &signed).await? {
            state = "INVALID".into();
            reason = Some("AUTHORITY_UNAVAILABLE");
        }
    }
    let snapshot_hash: Option<Vec<u8>> = row.try_get("snapshot_hash")?;
    serde_json::to_vec(&Status {
        contract_version: 2,
        operation: "PublishBazaarListingStatusV1",
        source_authority: source,
        listing_id: listing,
        seller_account_id: row.try_get("seller_account_id")?,
        world_id: row.try_get("world_id")?,
        projection_epoch: epoch.to_string(),
        projection_revision: revision.to_owned(),
        source_observed_at: observed.to_string(),
        asset_kind: if character { "CHARACTER" } else { "ITEM" },
        asset_id: row.try_get("asset_id")?,
        state,
        custody_id: row.try_get("custody_id")?,
        custody_revision: row.try_get("custody_revision")?,
        snapshot_hash: snapshot_hash.as_deref().map(hex),
        last_operation_id: row.try_get("last_operation_id")?,
        reason_code: reason,
    })
    .map(Some)
    .map_err(|_| DurabilityError::Unavailable)
}

impl DurabilityRoot {
    pub async fn bazaar_transport_snapshot(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        limit: u16,
    ) -> Result<TransportBatch, DurabilityError> {
        if limit == 0 || limit > TRANSPORT_BATCH_LIMIT || !source_authority_valid(source) {
            return Err(DurabilityError::Unavailable);
        }
        let content = self.prepare_bazaar_content(authority, proof).await;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let source = source.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let config=sqlx::query("SELECT enabled,policy FROM game_character_service_bazaar_policy()").fetch_one(&mut *tx).await?;
            let mut policy:Policy=serde_json::from_str(config.try_get("policy")?).map_err(|_|DurabilityError::InvalidStoredState)?;
            if let Some(content)=&content {content.bind(&mut policy);}
            // Acquire canonical content/admission serialization before any
            // projection state, outbox or signing-trust row lock.
            sqlx::query("LOCK TABLE game_content_activations IN SHARE MODE").execute(&mut *tx).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let epoch=recovery.recovery_generation;
            sqlx::query("INSERT INTO game_bazaar_projection_state(singleton,source,epoch,enumeration_cursor,enumeration_done,started_at) VALUES(true,$1,$2::numeric,'',false,$3) ON CONFLICT(singleton) DO NOTHING").bind(&source).bind(epoch.to_string()).bind(now).execute(&mut *tx).await?;
            let state=sqlx::query("SELECT source,epoch::text,enumeration_cursor,enumeration_done,version,policy_digest,last_observed FROM game_bazaar_projection_state WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
            if state.try_get::<String,_>("source")?!=source{return Err(DurabilityError::Unavailable);}
            let old_epoch:u64=state.try_get::<String,_>("epoch")?.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
            if old_epoch>epoch || now<state.try_get::<i64,_>("last_observed")? {return Err(DurabilityError::Unavailable);}
            let policy_digest=Sha256::digest(serde_json::to_vec(&policy).map_err(|_|DurabilityError::InvalidStoredState)?);
            let dirty=old_epoch<epoch || state.try_get::<Option<Vec<u8>>,_>("policy_digest")?.as_deref()!=Some(policy_digest.as_slice());
            if dirty {
                sqlx::query("UPDATE game_bazaar_projection_state SET epoch=$1::numeric,enumeration_cursor='',enumeration_done=false,started_at=$2,version=version+1,policy_digest=$3 WHERE singleton").bind(epoch.to_string()).bind(now).bind(policy_digest.as_slice()).execute(&mut *tx).await?;
            }
            let version:i64=sqlx::query_scalar("SELECT version FROM game_bazaar_projection_state WHERE singleton").fetch_one(&mut *tx).await?;
            let cursor=if dirty{String::new()}else{state.try_get("enumeration_cursor")?};
            if dirty||!state.try_get::<bool,_>("enumeration_done")? {
                let subjects:Vec<String>=sqlx::query_scalar("WITH subjects AS(SELECT 'policy:'||world_id::text AS subject FROM game_runtime_scope_assignments UNION SELECT 'policy:'||world_id::text FROM game_character_roots UNION SELECT 'listing:'||listing_id::text FROM game_bazaar_locks UNION SELECT 'listing:'||listing_id::text FROM game_bazaar_escrows) SELECT subject FROM subjects WHERE subject>$1 ORDER BY subject LIMIT $2").bind(&cursor).bind(i64::from(limit)).fetch_all(&mut *tx).await?;
                for subject in &subjects {
                    sqlx::query("INSERT INTO game_bazaar_publication_outbox(subject,epoch,revision,body,committed_at,operation,enumeration_version) VALUES($1,$2::numeric,1,convert_to('{}','UTF8'),$3,$4,$5) ON CONFLICT(subject) DO UPDATE SET epoch=EXCLUDED.epoch,revision=game_bazaar_publication_outbox.revision+CASE WHEN game_bazaar_publication_outbox.body<>convert_to('{}','UTF8') THEN 1 ELSE 0 END,body=convert_to('{}','UTF8'),committed_at=EXCLUDED.committed_at,acknowledged=false,enumeration_version=EXCLUDED.enumeration_version")
                        .bind(subject).bind(epoch.to_string()).bind(now).bind(if subject.starts_with("policy:"){"PublishBazaarTransferabilityV1"}else{"PublishBazaarListingStatusV1"}).bind(version).execute(&mut *tx).await?;
                }
                sqlx::query("UPDATE game_bazaar_projection_state SET enumeration_cursor=$1,enumeration_done=$2 WHERE singleton").bind(subjects.last().unwrap_or(&cursor)).bind(subjects.len()<usize::from(limit)).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE game_bazaar_projection_state SET last_observed=$1 WHERE singleton").bind(now).execute(&mut *tx).await?;
            let mut receipts=Vec::new();
            for row in sqlx::query("SELECT d.issuer,d.operation_id::text,d.command,d.receipt FROM game_bazaar_outbox o JOIN game_bazaar_decisions d USING(issuer,operation_id) WHERE NOT o.receipt_acknowledged AND NOT o.stopped ORDER BY d.decided_at,d.issuer,d.operation_id LIMIT $1 FOR UPDATE OF o SKIP LOCKED").bind(i64::from(limit)).fetch_all(&mut *tx).await?{
                let receipt:Vec<u8>=row.try_get("receipt")?;
                let command:String=row.try_get("command")?;
                if command=="LockBazaarCharacter"
                    && let Some(bytes)=receipt_signed_snapshot(&receipt)?
                    && !characters::signed_snapshot_provenance(&mut tx,&bytes).await? {
                    continue;
                }
                receipts.push(ReceiptDelivery{issuer:row.try_get("issuer")?,operation:row.try_get("operation_id")?,command,body:String::from_utf8(receipt).map_err(|_|DurabilityError::InvalidStoredState)?});
            }
            let mut publications=Vec::new();
            for row in sqlx::query("SELECT subject,epoch::text,revision::text,body,committed_at,operation FROM game_bazaar_publication_outbox WHERE NOT acknowledged AND enumeration_version=$2 ORDER BY subject LIMIT $1 FOR UPDATE SKIP LOCKED").bind(i64::from(limit)).bind(version).fetch_all(&mut *tx).await?{
                let subject:String=row.try_get("subject")?;let mut revision:String=row.try_get("revision")?;
                let old:Vec<u8>=row.try_get("body")?;
                let Some(mut body)=build(&mut tx,&policy,&source,&subject,epoch,&revision,row.try_get("committed_at")?).await? else {continue;};
                if old!=b"{}" && old!=body {
                    let next=revision.parse::<u64>().map_err(|_|DurabilityError::InvalidStoredState)?.checked_add(1).ok_or(DurabilityError::Unavailable)?;
                    revision=next.to_string();
                    let Some(rebuilt)=build(&mut tx,&policy,&source,&subject,epoch,&revision,row.try_get("committed_at")?).await? else {continue;};body=rebuilt;
                }
                if body.len()>if subject.starts_with("policy:"){262144}else{2048}{return Err(DurabilityError::Unavailable);}
                sqlx::query("UPDATE game_bazaar_publication_outbox SET body=$2,revision=$3::numeric WHERE subject=$1").bind(&subject).bind(&body).bind(&revision).execute(&mut *tx).await?;
                publications.push(PublicationDelivery{subject,epoch,revision:revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?,operation:if row.try_get::<String,_>("operation")?=="PublishBazaarTransferabilityV1"{Operation::PublishBazaarTransferabilityV1}else{Operation::PublishBazaarListingStatusV1},body:String::from_utf8(body).map_err(|_|DurabilityError::InvalidStoredState)?});
            }
            let complete:bool=sqlx::query_scalar("SELECT enumeration_done AND NOT EXISTS(SELECT 1 FROM game_bazaar_publication_outbox WHERE NOT acknowledged OR enumeration_version<> (SELECT version FROM game_bazaar_projection_state WHERE singleton)) AND NOT EXISTS(SELECT 1 FROM game_bazaar_outbox WHERE NOT receipt_acknowledged) FROM game_bazaar_projection_state WHERE singleton").fetch_one(&mut *tx).await?;
            let watermark=if complete && policy.common_restore_scope && now>2 {
                Some(serde_json::to_string(&Watermark{contract_version:2,operation:"PublishBazaarWatermarkV1",source_authority:&source,projection_epoch:epoch.to_string(),complete_through:(now-2).to_string(),observed_at:now.to_string()}).map_err(|_|DurabilityError::Unavailable)?)
            }else{None};
            let batch=TransportBatch{enabled:config.try_get("enabled")?,receipts,publications,watermark};
            commit_semantic_transaction(tx,deadline).await?;Ok(batch)
        })).await
    }

    pub async fn bazaar_transport_ack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        key: DeliveryKey,
        digest: [u8; 32],
        status: u16,
    ) -> Result<(), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            match key {
                DeliveryKey::Receipt{issuer,operation}=>{
                    sqlx::query("UPDATE game_bazaar_outbox o SET receipt_acknowledged=$3,stopped=$4 FROM game_bazaar_decisions d WHERE o.issuer=$1 AND o.operation_id=$2::uuid AND d.issuer=o.issuer AND d.operation_id=o.operation_id AND d.receipt_digest=$5")
                        .bind(issuer).bind(operation).bind(status==200).bind(matches!(status,400|401|404|409|413)).bind(digest.as_slice()).execute(&mut *tx).await?;
                },
                DeliveryKey::Publication{subject,epoch,revision}=>{
                    if epoch != recovery.recovery_generation { return Err(DurabilityError::Unavailable); }
                    if status==200 {sqlx::query("UPDATE game_bazaar_publication_outbox SET acknowledged=true WHERE subject=$1 AND epoch=$2::numeric AND revision=$3::numeric AND sha256(body)=$4").bind(subject).bind(epoch.to_string()).bind(revision.to_string()).bind(digest.as_slice()).execute(&mut *tx).await?;}
                },
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(())
        })).await
    }

    pub async fn bazaar_reconcile_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        remote: &RemoteState,
    ) -> Result<(), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let remote = remote.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            if let Some(row)=sqlx::query("SELECT command,receipt_digest,recovery_generation::text,convert_from(receipt,'UTF8')::jsonb->>'outcome' AS outcome FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid FOR SHARE").bind(wire::ISSUER).bind(&remote.operation_id).fetch_optional(&mut *tx).await?{
                let same=row.try_get::<String,_>("command")?==remote.command;
                let pending=matches!(remote.state.as_str(),"PENDING"|"REOPENED");
                let matching=row.try_get::<String,_>("outcome")?==remote.state && remote.receipt_digest.as_ref().is_some_and(|hash|hex(&row.get::<Vec<u8>,_>("receipt_digest"))==hash.as_str())
                    && remote.recovery_generation.as_ref().is_some_and(|g|g.get().to_string()==row.get::<String,_>("recovery_generation"));
                sqlx::query("UPDATE game_bazaar_outbox SET receipt_acknowledged=$3,stopped=stopped OR $4 WHERE issuer=$1 AND operation_id=$2::uuid").bind(wire::ISSUER).bind(&remote.operation_id).bind(!pending&&same&&matching).bind(!same||(!pending&&!matching)).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(())
        })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_receipt_extracts_exact_signed_snapshot_bytes() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../docs/contracts/fixtures/bazaar/decisions.json"
        ))?;
        let snapshot = fixtures["vectors"]
            .as_array()
            .ok_or("missing vectors")?
            .iter()
            .find(|v| v["name"] == "locked_character_snapshot")
            .and_then(|v| v["wire"].as_str())
            .ok_or("missing snapshot")?;
        let receipt = format!(r#"{{"result_data":{{"signed_snapshot":{snapshot}}}}}"#);
        assert_eq!(
            receipt_signed_snapshot(receipt.as_bytes())?.as_deref(),
            Some(snapshot.as_bytes())
        );
        assert!(receipt_signed_snapshot(br#"{"result_data":null}"#)?.is_none());
        Ok(())
    }

    #[test]
    fn delivered_status_fixture_matches_production_serializer_exactly()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../docs/contracts/fixtures/bazaar/decisions.json"
        ))?;
        let vector = fixtures["vectors"]
            .as_array()
            .ok_or("missing vectors")?
            .iter()
            .find(|v| v["name"] == "delivered_status")
            .ok_or("missing status fixture")?;
        let raw = vector["wire"].as_str().ok_or("missing wire")?;
        let value: serde_json::Value = serde_json::from_str(raw)?;
        let text = |name: &str| -> Result<String, Box<dyn std::error::Error>> {
            Ok(value[name]
                .as_str()
                .ok_or("missing status member")?
                .to_owned())
        };
        // These constants select the same closed production record shape. Every
        // scalar and nullable member comes from the normative vector itself.
        let status = Status {
            contract_version: 2,
            operation: "PublishBazaarListingStatusV1",
            source_authority: value["source_authority"].as_str().ok_or("missing source")?,
            listing_id: value["listing_id"].as_str().ok_or("missing listing")?,
            seller_account_id: text("seller_account_id")?,
            world_id: text("world_id")?,
            projection_epoch: text("projection_epoch")?,
            projection_revision: text("projection_revision")?,
            source_observed_at: text("source_observed_at")?,
            asset_kind: "ITEM",
            asset_id: text("asset_id")?,
            state: text("state")?,
            custody_id: text("custody_id")?,
            custody_revision: text("custody_revision")?,
            snapshot_hash: value["snapshot_hash"].as_str().map(str::to_owned),
            last_operation_id: value["last_operation_id"].as_str().map(str::to_owned),
            reason_code: None,
        };
        assert!(value["reason_code"].is_null());
        let encoded = serde_json::to_vec(&status)?;
        assert_eq!(encoded, raw.as_bytes());
        assert_eq!(
            hex(&Sha256::digest(&encoded)),
            vector["sha256"].as_str().ok_or("missing hash")?
        );
        Ok(())
    }

    #[test]
    fn signed_snapshot_cannot_be_relabelled_with_new_epoch_or_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../docs/contracts/fixtures/bazaar/decisions.json"
        ))?;
        let raw = fixtures["vectors"]
            .as_array()
            .ok_or("missing vectors")?
            .iter()
            .find(|v| v["name"] == "locked_character_snapshot")
            .and_then(|v| v["wire"].as_str())
            .ok_or("missing snapshot")?;
        let mut policy = Policy {
            snapshot_source_authority: Some("oteryn:game:primary".into()),
            ..Policy::default()
        };
        assert!(snapshot_projection_current(
            raw.as_bytes(),
            &policy,
            "oteryn:game:primary",
            3
        ));
        assert!(!snapshot_projection_current(
            raw.as_bytes(),
            &policy,
            "oteryn:game:primary",
            4
        ));
        assert!(!snapshot_projection_current(
            raw.as_bytes(),
            &policy,
            "oteryn:game:other",
            3
        ));
        policy.snapshot_source_authority = None;
        assert!(!snapshot_projection_current(
            raw.as_bytes(),
            &policy,
            "oteryn:game:primary",
            3
        ));
        Ok(())
    }
}
