//! Registered #3010 payload tags and EventEnvelope; no new event ids.
use super::*;
use crate::durability::item_mint_audit::{EventEnvelopeV1, TransactionEventRefV1};
use prost::Message;
#[derive(Clone, PartialEq, Message)]
pub struct WorldEventOutcomeDecidedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub record: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub record_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub source_evidence_sha256: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub recovery_generation: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub subject_event_id: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
pub struct GuildSeasonStandingsDecidedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub record_sha256: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub recovery_generation: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub season_id: Vec<u8>,
}
pub(super) fn hex(raw: &str) -> Result<Vec<u8>> {
    if !raw.len().is_multiple_of(2) {
        return Err(DurabilityError::InvalidStoredState);
    }
    raw.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            std::str::from_utf8(pair)
                .map_err(invalid)
                .and_then(|p| u8::from_str_radix(p, 16).map_err(invalid))
        })
        .collect()
}
pub(super) fn uuid(raw: &str) -> Result<Vec<u8>> {
    hex(&raw.replace('-', ""))
}
pub(super) fn encode(
    s: &Snapshot,
    record: &[u8],
    event: &str,
    transaction: &str,
    generation: u64,
    at: i64,
) -> Result<Vec<u8>> {
    let (event_type, payload) = match s {
        Snapshot::Event(s) => {
            let Some(e) = s.event.first() else {
                return Ok(Vec::new());
            };
            // SCHEDULED/LOCKED are retained immutable publication audit, never a
            // fabricated WORLD_EVENT_OUTCOME_DECIDED with absent terminal evidence.
            if !matches!(e.state.as_str(), "SETTLED" | "VOID" | "CLOSED") {
                return Ok(Vec::new());
            }
            let evidence = e
                .evidence_sha256
                .first()
                .ok_or(DurabilityError::InvalidStoredState)?;
            (
                10,
                WorldEventOutcomeDecidedV1 {
                    record: record.to_vec(),
                    record_sha256: Sha256::digest(record).to_vec(),
                    source_evidence_sha256: hex(evidence)?,
                    recovery_generation: generation,
                    subject_event_id: uuid(&s.event_id)?,
                }
                .encode_to_vec(),
            )
        }
        Snapshot::Season(s) => {
            if s.season.is_empty() {
                return Ok(Vec::new());
            }
            (
                11,
                GuildSeasonStandingsDecidedV1 {
                    record_sha256: Sha256::digest(record).to_vec(),
                    recovery_generation: generation,
                    season_id: uuid(&s.season_id)?,
                }
                .encode_to_vec(),
            )
        }
    };
    if payload.len() > 196608 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let envelope = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: uuid(event)?,
        event_type_id: event_type,
        event_schema_revision: 1,
        durability_class: 2,
        privacy_class: 3,
        retention_profile_id: "PLAT3_AUTHORITY_AUDIT_RETENTION_V1".into(),
        occurred_at_unix_ms: at,
        world_id: Some(uuid(fields(s).1)?),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: uuid(transaction)?,
            ordinal: 1,
            count: 1,
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let raw = envelope.encode_to_vec();
    if raw.len() > 262144 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(raw)
}
