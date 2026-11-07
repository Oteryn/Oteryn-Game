use super::*;
use crate::durability::npc_travel::{
    NpcTravelGate, PaidTravelRequest, PendingArrival, TravelRefusal,
};

impl RevisionSlot {
    /// Confirm travel under the same Character slot as NPC purchases, including zero fares.
    pub async fn commit_npc_travel<G: NpcTravelGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut fence: CurrentCharacterGameplayFence,
        request: PaidTravelRequest,
        gate: &G,
    ) -> Result<Result<Option<PendingArrival>, TravelRefusal>, CharacterProgressionError> {
        if self.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        fence.expected_character_revision = self
            .cursor(root, authority)
            .await
            .map_err(CharacterProgressionError::from_authority)?;
        let result = root
            .commit_npc_travel(authority, node, fence, request, gate)
            .await;
        match &result {
            // Travel returns an arrival rather than a revision receipt. Reload the root
            // before the next write, also after a replay or an unknown outcome.
            Ok(Ok(_)) | Err(_) => *self.cursor = None,
            Ok(Err(_)) => (),
        }
        result
    }
}
