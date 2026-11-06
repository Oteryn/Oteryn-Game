use super::{SoulpitActivityBinding, SoulpitRuntimeError};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{SoulpitAction, SoulpitStepRequest, SoulpitStoredEncounter};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) fn occurrence(
    binding: SoulpitActivityBinding,
    purpose: &[u8],
    ordinal: u64,
) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(b"oteryn.soulpit.runtime.v1")
        .chain_update(binding.handoff_id)
        .chain_update(binding.generation.get().to_be_bytes())
        .chain_update(purpose)
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value
}

pub(super) fn draw(
    root: [u8; 32],
    id: [u8; 16],
    purpose: &str,
    ordinal: u64,
    min: i64,
    max: i64,
) -> Result<i64, SoulpitRuntimeError> {
    let value = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(root),
        DecisionOccurrenceId::from_bytes(id),
        purpose,
        ordinal,
    )
    .map_err(|_| SoulpitRuntimeError::InvalidState)?;
    Ok(oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        value, min, max,
    ))
}

pub(in crate::gameplay_transport::soulpit) fn request(
    binding: SoulpitActivityBinding,
    stored: &SoulpitStoredEncounter,
    now: u64,
    action: SoulpitAction,
    survivors: Vec<CurrentCharacterGameplayFence>,
) -> Result<SoulpitStepRequest, SoulpitRuntimeError> {
    let semantic = serde_json::to_vec(&action).map_err(|_| SoulpitRuntimeError::InvalidState)?;
    Ok(SoulpitStepRequest {
        handoff_id: binding.handoff_id,
        occurrence: occurrence(binding, &semantic, stored.revision),
        runtime_scope: binding.scope,
        scope_ownership_generation: binding.generation,
        expected_revision: stored.revision,
        now_micros: now,
        action,
        survivors,
    })
}
