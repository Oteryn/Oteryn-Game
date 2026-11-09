//! An admitted room recipient remains independently fenced at durable settlement.
use super::*;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterDeathContext {
    pub source: String,
    pub scope: RuntimeScopeRefV1,
    pub scope_generation: u64,
    pub character: [u8; 16],
    pub session: GameSessionId,
    pub lease_generation: u64,
    pub participant: [u8; 16],
    pub defender: [u8; 16],
}
impl EncounterDeathContext {
    pub(super) fn matches(&self, fence: CurrentCharacterGameplayFence) -> bool {
        self.scope == fence.runtime_scope
            && self.scope_generation == fence.scope_ownership_generation.get()
            && self.character == *fence.character_id.as_bytes()
            && self.session == fence.game_session_id
            && self.lease_generation == fence.character_lease_generation
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) {
        let mut digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"admitted-encounter-death-v1")
            .chain_update(self.scope.world_id().as_bytes());
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                digest.update([1]);
                digest.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                digest.update([2]);
                digest.update(instance_id);
            }
        }
        digest.update(self.scope_generation.to_be_bytes());
        digest.update(self.character);
        digest.update(self.session.as_bytes());
        digest.update(self.lease_generation.to_be_bytes());
        digest.update(self.participant);
        digest.update(self.defender);
        digest.update(self.source.as_bytes());
        binding[1..].copy_from_slice(&digest.finalize());
    }
}
