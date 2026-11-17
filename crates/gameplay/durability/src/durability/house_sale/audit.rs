//! Registered Foundation event 8, kind 5, in the same physical transfer transaction.
use super::super::item_mint_audit::{EventEnvelopeV1, TransactionEventRefV1, encode_bounded};
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};
#[derive(Clone, PartialEq, Message)]
struct SaleEvent {
    #[prost(string, tag = "2")]
    house: String,
    #[prost(uint32, tag = "5")]
    kind: u32,
    #[prost(message, optional, tag = "26")]
    ownership_transfer: Option<Transfer>,
}
#[derive(Clone, PartialEq, Message)]
struct Transfer {
    #[prost(bytes = "vec", tag = "1")]
    offer: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    seller: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    recipient: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    seller_account: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    recipient_account: Vec<u8>,
    #[prost(uint32, tag = "6")]
    action: u32,
    #[prost(uint32, tag = "7")]
    state: u32,
    #[prost(uint64, tag = "8")]
    price: u64,
    #[prost(uint64, tag = "9")]
    rent: u64,
    #[prost(int64, tag = "10")]
    due_at_unix_ms: i64,
    #[prost(uint64, tag = "11")]
    reserved_gold: u64,
    #[prost(uint64, tag = "12")]
    refunded_gold: u64,
    #[prost(uint64, tag = "13")]
    seller_payment: u64,
    #[prost(uint64, tag = "14")]
    rent_burn: u64,
}
pub(in crate::durability) async fn write(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    scope: &Value,
    event: &str,
) -> std::result::Result<(), DurabilityError> {
    let mut p = json!({"world":scope["world"],"channel":scope["channel"],"generation":scope["generation"],"event":event});
    for key in ["instance","session","character","connection","lease"] {
        if let Some(value)=scope.get(key) {p[key]=value.clone();}
    }
    let row = audit_port(tx, node, "snapshot", &p).await?;
    let bad = || DurabilityError::InvalidStoredState;
    let id = |key: &str| -> std::result::Result<Vec<u8>, DurabilityError> {
        Ok(
            super::super::character_progression::uuid_text(row[key].as_str().ok_or_else(bad)?)?
                .to_vec(),
        )
    };
    let number = |key: &str| row[key].as_u64().ok_or_else(bad);
    let action = number("action")?;
    let price = number("price")?;
    let rent = number("rent")?;
    let total = price
        .checked_add(rent)
        .filter(|v| *v <= 999_999_999_999)
        .ok_or_else(bad)?;
    let state = number("state")?;
    let before = number("before_state")?;
    if !(3..=7).contains(&action) || !(1..=5).contains(&state) {
        return Err(bad());
    }
    let payload = encode_bounded(
        &SaleEvent {
            house: row["house"].as_str().ok_or_else(bad)?.into(),
            kind: 5,
            ownership_transfer: Some(Transfer {
                offer: id("offer")?,
                seller: id("seller")?,
                recipient: id("recipient")?,
                seller_account: id("seller_account")?,
                recipient_account: id("recipient_account")?,
                action: action as u32,
                state: state as u32,
                price,
                rent,
                due_at_unix_ms: row["due"].as_i64().ok_or_else(bad)?,
                reserved_gold: if action == 5 { total } else { 0 },
                refunded_gold: if matches!(action, 4 | 6) && before == 2 {
                    total
                } else {
                    0
                },
                seller_payment: if action == 7 { price } else { 0 },
                rent_burn: if action == 7 { rent } else { 0 },
            }),
        },
        8192,
    )
    .map_err(|_| bad())?;
    let now = row["now"].as_i64().ok_or_else(bad)?;
    let envelope = encode_bounded(
        &EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: id("event")?,
            event_type_id: 8,
            event_schema_revision: 1,
            durability_class: 2,
            privacy_class: 3,
            retention_profile_id: "ECONOMY_LEDGER_RETENTION_V1".into(),
            occurred_at_unix_ms: now,
            world_id: Some(id("world")?),
            operation_id: Some(id("offer")?),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: id("event")?,
                ordinal: 1,
                count: 1,
            }),
            server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload,
            ..Default::default()
        },
        16384,
    )
    .map_err(|_| bad())?;
    p["now"] = now.into();
    p["envelope"] = envelope
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    if audit_port(tx, node, "insert", &p).await? != Value::Bool(true) {
        return Err(bad());
    }
    Ok(())
}
async fn audit_port(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    mode: &str,
    p: &Value,
) -> std::result::Result<Value, DurabilityError> {
    let value: Option<Value> = sqlx::query_scalar(
        "SELECT game_house_sale_audit_port($1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5)",
    )
    .bind(mode)
    .bind(p)
    .bind(node.fact().node_id().as_bytes().as_slice())
    .bind(node.fact().registration_revision().to_string())
    .bind(node.sql_possession_secret())
    .fetch_one(&mut **tx)
    .await?;
    value.ok_or(DurabilityError::InvalidStoredState)
}
