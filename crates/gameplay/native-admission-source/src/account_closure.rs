//! Private v2 account-closure transport, purpose-bound to Character commands.
use super::{SourceError, descriptor::Operation};
use serde::{Deserialize, Serialize};
pub const PATH: &str = "/internal/v1/game-auth/account-closure";
pub const WATERMARK_PATH: &str = "/internal/v1/game-auth/account-closure/watermark";
pub const SNAPSHOT_BYTES: usize = 2048;
pub const RECEIPT_BYTES: usize = 2048;
pub const WATERMARK_BYTES: usize = 512;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub operation: Operation,
    pub body: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    contract_version: u8,
    result: String,
}
pub fn decode_ack(raw: &[u8]) -> Result<(), SourceError> {
    let ack: Ack = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if raw.len() > 256
        || ack.contract_version != 2
        || !matches!(ack.result.as_str(), "accepted" | "superseded")
        || serde_json::to_vec(&ack).map_err(|_| SourceError::InvalidInput)? != raw
    {
        return Err(SourceError::InvalidInput);
    }
    Ok(())
}

pub const LIFECYCLE_PATH: &str = "/internal/v1/game-auth/account-closure/lifecycle/read";
#[derive(Debug, Clone, Serialize)]
pub struct LifecycleRequest {
    pub contract_version: u8,
    pub operation: &'static str,
    pub source_authority: String,
    pub account_id: String,
    pub termination_id: String,
    pub recovery_generation: String,
    pub audience: &'static str,
}
impl LifecycleRequest {
    pub fn from_intent(source: &str, raw: &[u8], recovery: u64) -> Result<Self, SourceError> {
        let intent: serde_json::Value =
            serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
        let account = intent["account_id"]
            .as_str()
            .ok_or(SourceError::InvalidInput)?;
        let termination = intent["termination_id"]
            .as_str()
            .ok_or(SourceError::InvalidInput)?;
        if !super::character_commands::uuid_v7(account)
            || !super::character_commands::uuid_v7(termination)
            || recovery == 0
        {
            return Err(SourceError::InvalidInput);
        }
        Ok(Self {
            contract_version: 2,
            operation: "ReadAccountClosureLifecycleV1",
            source_authority: source.into(),
            account_id: account.into(),
            termination_id: termination.into(),
            recovery_generation: recovery.to_string(),
            audience: super::character_commands::ISSUER_AUTHORITY,
        })
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleObservation {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub termination_id: String,
    pub lifecycle_generation: String,
    pub source_revision: String,
    pub phase: String,
    pub financial_obligations_clear: bool,
    pub recovery_generation: String,
    pub observed_at: String,
    pub valid_until: String,
}
pub fn decode_lifecycle(
    request: &LifecycleRequest,
    raw: &[u8],
) -> Result<LifecycleObservation, SourceError> {
    use super::character_commands::counter;
    if request.contract_version != 2
        || request.operation != "ReadAccountClosureLifecycleV1"
        || request.audience != super::character_commands::ISSUER_AUTHORITY
        || !super::character_commands::source_authority_valid(&request.source_authority)
        || !super::character_commands::uuid_v7(&request.account_id)
        || !super::character_commands::uuid_v7(&request.termination_id)
    {
        return Err(SourceError::InvalidInput);
    }

    if raw.len() > 2048 {
        return Err(SourceError::CapacityExceeded);
    }
    let o: LifecycleObservation =
        serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if serde_json::to_vec(&o).map_err(|_| SourceError::InvalidInput)? != raw
        || raw.iter().any(|b| *b <= 0x20 || *b == b'\\')
        || o.contract_version != 2
        || o.operation != "ReadAccountClosureLifecycleV1"
        || o.source_authority != super::character_commands::ISSUER_AUTHORITY
        || o.account_id != request.account_id
        || o.termination_id != request.termination_id
        || o.recovery_generation != request.recovery_generation
        || !matches!(o.phase.as_str(), "CLOSE" | "CANCEL" | "FINALIZE")
        || [
            &o.lifecycle_generation,
            &o.source_revision,
            &o.observed_at,
            &o.valid_until,
            &o.recovery_generation,
        ]
        .iter()
        .any(|s| counter(s).is_none())
    {
        return Err(SourceError::InvalidInput);
    }
    let observed = counter(&o.observed_at).ok_or(SourceError::InvalidInput)?;
    let until = counter(&o.valid_until).ok_or(SourceError::InvalidInput)?;
    if until <= observed || until - observed > 5 || i64::try_from(until).is_err() {
        return Err(SourceError::InvalidInput);
    }
    Ok(o)
}

pub fn decode_receipt_ack(raw: &[u8]) -> Result<(), SourceError> {
    if raw != br#"{"contract_version":2,"result":"accepted"}"# {
        return Err(SourceError::InvalidInput);
    }
    Ok(())
}
#[cfg(test)]
#[path = "account_closure_tests.rs"]
mod tests;
