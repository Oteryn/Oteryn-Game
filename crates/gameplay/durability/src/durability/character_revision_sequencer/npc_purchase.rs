use super::*;
use crate::durability::npc_purchase::{NpcPurchaseGate, NpcPurchaseRequest, PurchaseOutcome};

impl RevisionSlot {
    pub async fn commit_npc_purchase<G: NpcPurchaseGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut fence: CurrentCharacterGameplayFence,
        request: NpcPurchaseRequest,
        gate: &G,
    ) -> Result<PurchaseOutcome, CharacterProgressionError> {
        if self.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        fence.expected_character_revision = self
            .cursor(root, authority)
            .await
            .map_err(CharacterProgressionError::from_authority)?;
        let result = root
            .commit_npc_purchase(authority, node, fence, request, gate)
            .await;
        match &result {
            Ok(PurchaseOutcome::Committed(receipt)) => {
                *self.cursor = Some(
                    self.cursor
                        .as_ref()
                        .copied()
                        .filter(|current| current.get() > receipt.committed_revision.get())
                        .unwrap_or(receipt.committed_revision),
                )
            }
            Err(_) => *self.cursor = None,
            Ok(PurchaseOutcome::Refused(_)) => (),
        }
        result
    }
}
