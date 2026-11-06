//! S17 P2: current-node-fenced, immutable, contiguous audit prefix commits.
//! Receipt fields are private: an IO attempt cannot acknowledge the journal.
mod bans;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_mint_audit::{EventEnvelopeV1, check_uuid_v7};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct CastAuditRecord {
    #[prost(uint64, tag = "1")]
    pub ordinal: u64,
    #[prost(uint32, tag = "2")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub target_session: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub cast_id: Vec<u8>,
    #[prost(uint64, tag = "5")]
    pub revision: u64,
    #[prost(uint64, tag = "6")]
    pub epoch: u64,
    #[prost(bytes = "vec", optional, tag = "7")]
    pub subject_account: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "8")]
    pub subject_character: Option<Vec<u8>>,
    #[prost(bool, optional, tag = "9")]
    pub consent: Option<bool>,
}
#[derive(Debug, Clone, Copy)]
pub struct CastAuditReceipt {
    incarnation: [u8; 16],
    through: u64,
}
impl CastAuditReceipt {
    pub const fn incarnation(self) -> [u8; 16] {
        self.incarnation
    }
    pub const fn through(self) -> u64 {
        self.through
    }
}

impl DurabilityRoot {
    pub async fn commit_cast_audit(
        &self,
        node: &NodeIncarnationProof,
        incarnation: [u8; 16],
        records: &[CastAuditRecord],
    ) -> Result<CastAuditReceipt, DurabilityError> {
        check_uuid_v7(&incarnation).map_err(|_| DurabilityError::InvalidStoredState)?;
        if records.len() > 64 {
            return Err(DurabilityError::InvalidStoredState);
        }
        for (i, r) in records.iter().enumerate() {
            if r.ordinal == 0
                || r.ordinal > i64::MAX as u64
                || !(1..=12).contains(&r.kind)
                || r.cast_id.len() != 16
                || r.subject_account.as_ref().is_some_and(|v| v.len() != 16)
                || r.subject_character.as_ref().is_some_and(|v| v.len() != 16)
                || (r.kind == 8 && (r.subject_character.is_none() || r.consent.is_none()))
                || (i > 0 && records[i - 1].ordinal.checked_add(1) != Some(r.ordinal))
                || r.encoded_len() > 256
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let id: [u8; 16] = r
                .target_session
                .as_slice()
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            check_uuid_v7(&id).map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        let node = node.clone();
        let records = records.to_vec();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let through = write_batch(&mut tx, &node, incarnation, records).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(CastAuditReceipt {
                        incarnation,
                        through,
                    })
                })
            })
            .await
    }
}

async fn write_batch(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    incarnation: [u8; 16],
    records: Vec<CastAuditRecord>,
) -> Result<u64, DurabilityError> {
    if !prove_current_incarnation(tx, node).await? {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("INSERT INTO game_cast_audit_streams(incarnation,node_id,registration_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric) ON CONFLICT DO NOTHING")
                .bind(incarnation.as_slice()).bind(node.fact().node_id().as_bytes().as_slice()).bind(node.fact().registration_revision().to_string()).execute(&mut **tx).await?;
    let stream=sqlx::query("SELECT uuid_send(node_id) AS node,registration_revision::text AS registration,last_ordinal FROM game_cast_audit_streams WHERE incarnation=encode($1,'hex')::uuid FOR UPDATE")
                .bind(incarnation.as_slice()).fetch_one(&mut **tx).await?;
    if stream.try_get::<Vec<u8>, _>("node")? != node.fact().node_id().as_bytes().as_slice()
        || stream.try_get::<String, _>("registration")?
            != node.fact().registration_revision().to_string()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut last: i64 = stream.try_get("last_ordinal")?;
    for record in records {
        let ordinal =
            i64::try_from(record.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?;
        let payload = record.encode_to_vec();
        if ordinal <= last {
            let stored:Option<Vec<u8>>=sqlx::query_scalar("SELECT payload FROM game_cast_audit_outbox WHERE incarnation=encode($1,'hex')::uuid AND ordinal=$2").bind(incarnation.as_slice()).bind(ordinal).fetch_optional(&mut **tx).await?;
            if stored.as_deref() != Some(payload.as_slice()) {
                return Err(DurabilityError::InvalidStoredState);
            }
            continue;
        }
        if last.checked_add(1) != Some(ordinal) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let (event,now):(Vec<u8>,i64)=sqlx::query_as("SELECT uuid_send(game_character_uuid_v7()),floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
        let envelope = EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: event.clone(),
            event_type_id: 12,
            event_schema_revision: 1,
            durability_class: 2,
            privacy_class: 4,
            retention_profile_id: "SPECTATE_AUDIT_RETENTION_V1".into(),
            occurred_at_unix_ms: now,
            node_id: Some(node.fact().node_id().as_bytes().to_vec()),
            game_session_id: Some(record.target_session),
            server_build_id: env!("CARGO_PKG_VERSION").into(),
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload: payload.clone(),
            ..Default::default()
        }
        .encode_to_vec();
        sqlx::query("INSERT INTO game_cast_audit_outbox(event_id,incarnation,ordinal,payload,envelope,envelope_sha256,occurred_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$7+2592000000)")
                    .bind(event).bind(incarnation.as_slice()).bind(ordinal).bind(payload).bind(&envelope).bind(Sha256::digest(&envelope).as_slice()).bind(now).execute(&mut **tx).await?;
        last = ordinal;
    }
    sqlx::query("UPDATE game_cast_audit_streams SET last_ordinal=$2 WHERE incarnation=encode($1,'hex')::uuid").bind(incarnation.as_slice()).bind(last).execute(&mut **tx).await?;
    Ok(last as u64)
}

/// The Character root is already locked and fenced. Singleton consent producers
/// are transactional: CAS replay/conflict emits nothing; preference and event
/// either both commit or both roll back. Zero cast_id denotes no active cast.
pub(super) async fn record_consent(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: super::character_progression::CurrentCharacterGameplayFence,
    revision: i64,
    allowed: bool,
) -> Result<(), DurabilityError> {
    let id: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&mut **tx)
        .await?;
    let incarnation: [u8; 16] = id
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    write_batch(
        tx,
        node,
        incarnation,
        vec![CastAuditRecord {
            ordinal: 1,
            kind: 8,
            target_session: fence.game_session_id.as_bytes().to_vec(),
            cast_id: vec![0; 16],
            revision: revision as u64,
            epoch: 0,
            subject_account: None,
            subject_character: Some(fence.character_id.as_bytes().to_vec()),
            consent: Some(allowed),
        }],
    )
    .await?;
    Ok(())
}
