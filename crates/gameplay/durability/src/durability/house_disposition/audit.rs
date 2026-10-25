//! Foundation event 8, retained in the same transaction as the native disposition mutation.
use super::super::item_mint_audit::{EventEnvelopeV1, TransactionEventRefV1, encode_bounded};
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};

#[derive(Clone, PartialEq, Message)]
struct DispositionEvent {
    #[prost(string, tag = "2")]
    house: String,
    #[prost(bytes = "vec", tag = "4")]
    character: Vec<u8>,
    #[prost(uint32, tag = "5")]
    kind: u32,
    #[prost(bytes = "vec", tag = "21")]
    disposition: Vec<u8>,
    #[prost(uint32, tag = "22")]
    state: u32,
    #[prost(int64, tag = "23")]
    due_at_unix_ms: i64,
    #[prost(bytes = "vec", optional, tag = "24")]
    census_digest: Option<Vec<u8>>,
    #[prost(message, repeated, tag = "25")]
    transfers: Vec<Transfer>,
}
#[derive(Clone, PartialEq, Message)]
struct Transfer {
    #[prost(bytes = "vec", tag = "1")]
    item: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    reclaim_subject: Vec<u8>,
    #[prost(uint64, tag = "3")]
    ordinal: u64,
}

pub(super) async fn write(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    p: &Value,
    operation: &str,
) -> std::result::Result<(), DurabilityError> {
    let mut p = p.clone();
    p["operation"] = operation.into();
    let row = port(tx, node, "audit_snapshot", &p).await?;
    let bad = || DurabilityError::InvalidStoredState;
    let id = |key: &str| -> std::result::Result<Vec<u8>, DurabilityError> {
        let raw = row.get(key).and_then(Value::as_str).ok_or_else(bad)?;
        Ok(super::super::character_progression::uuid_text(raw)?.to_vec())
    };
    let transfers = row["transfers"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|r| {
            Ok(Transfer {
                item: super::super::character_progression::uuid_text(
                    r["item"].as_str().ok_or_else(bad)?,
                )?
                .to_vec(),
                reclaim_subject: super::super::character_progression::uuid_text(
                    r["recipient"].as_str().ok_or_else(bad)?,
                )?
                .to_vec(),
                ordinal: r["ordinal"]
                    .as_str()
                    .ok_or_else(bad)?
                    .parse()
                    .map_err(|_| bad())?,
            })
        })
        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
    let batch = transfers
        .iter()
        .map(|item| {
            Ok((
                <[u8; 16]>::try_from(item.item.as_slice()).map_err(|_| bad())?,
                <[u8; 16]>::try_from(item.reclaim_subject.as_slice()).map_err(|_| bad())?,
            ))
        })
        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
    if !oteryn_gameplay_housing::house_transfer::valid_step(&batch)
        || row["count"]
            .as_u64()
            .is_none_or(|count| count > oteryn_gameplay_housing::house_transfer::CENSUS_MAX)
    {
        return Err(bad());
    }
    let digest = row["digest"]
        .as_str()
        .map(|h| {
            (0..h.len())
                .step_by(2)
                .map(|i| {
                    h.get(i..i + 2)
                        .and_then(|b| u8::from_str_radix(b, 16).ok())
                        .ok_or_else(bad)
                })
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .transpose()?;
    let operation = id("operation")?;
    let event = id("event")?;
    let now = row["now"].as_i64().ok_or_else(bad)?;
    let payload = encode_bounded(
        &DispositionEvent {
            house: row["house"].as_str().ok_or_else(bad)?.into(),
            character: id("owner")?,
            kind: 4,
            disposition: operation.clone(),
            state: row["state"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(bad)?,
            due_at_unix_ms: row["due"].as_i64().ok_or_else(bad)?,
            census_digest: digest,
            transfers,
        },
        65_536,
    )
    .map_err(|_| bad())?;
    let envelope = encode_bounded(
        &EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: event,
            event_type_id: 8,
            event_schema_revision: 1,
            durability_class: 2,
            privacy_class: 3,
            retention_profile_id: "ECONOMY_LEDGER_RETENTION_V1".into(),
            occurred_at_unix_ms: now,
            world_id: Some(id("world")?),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: id("event")?,
                ordinal: 1,
                count: 1,
            }),
            operation_id: Some(operation),
            server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload,
            ..Default::default()
        },
        131_072,
    )
    .map_err(|_| bad())?;
    p["event"] = row["event"].clone();
    p["now"] = now.into();
    p["envelope"] = envelope
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    port(tx, node, "audit_insert", &p).await?;
    Ok(())
}
