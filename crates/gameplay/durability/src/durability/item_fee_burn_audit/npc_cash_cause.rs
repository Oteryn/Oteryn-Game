//! NPC cash has its own closed proof; no Gem/Atelier mutation is manufactured.
use super::*;
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    kind: String,
    character: Vec<u8>,
    occurrence: Vec<u8>,
    coin_inputs: Vec<Value>,
}
pub(super) fn cause(value: &OneItemFeeBurnV1) -> Option<&OneItemNpcPurchaseV1> {
    match value.cause.as_ref()?.cause.as_ref()? {
        FeeBurnCauseV1::HorseRental(c) | FeeBurnCauseV1::NpcCosmeticPurchase(c) => Some(c),
        _ => None,
    }
}
pub(super) fn check(value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    let c = cause(value).ok_or(AuditError::InvalidInput)?;
    if c.atelier_proof.len() > atelier_cause::PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let proof: Proof =
        serde_json::from_slice(&c.atelier_proof).map_err(|_| AuditError::InvalidInput)?;
    if proof.kind != "npc_carried_cash_v1"
        || proof.character != value.character_id
        || proof.occurrence != c.occurrence
        || proof.coin_inputs.len() != value.lines.len()
        || proof.coin_inputs.len() > atelier_cause::COIN_INPUTS_MAX
    {
        return Err(AuditError::InvalidInput);
    }
    for source in &proof.coin_inputs {
        let object = source.as_object().ok_or(AuditError::InvalidInput)?;
        if object.len() != 6
            || ![
                "item",
                "surface",
                "parent",
                "ordinal",
                "slot",
                "selection_order",
            ]
            .iter()
            .all(|key| object.contains_key(*key))
        {
            return Err(AuditError::InvalidInput);
        }
    }
    coin_sources(value)?.ok_or(AuditError::InvalidInput)?;
    Ok(())
}
pub(super) fn coin_sources(
    value: &OneItemFeeBurnV1,
) -> Result<Option<Vec<atelier_cause::CoinSource>>, AuditError> {
    let Some(c) = cause(value) else {
        return Ok(None);
    };
    let proof: Value =
        serde_json::from_slice(&c.atelier_proof).map_err(|_| AuditError::InvalidInput)?;
    atelier_cause::coin_sources_from_proof(value, &proof)
}
