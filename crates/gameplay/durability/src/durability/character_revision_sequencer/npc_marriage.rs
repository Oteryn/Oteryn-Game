use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::npc_marriage::{MarriageOutcome, MarriageRequest};

impl RevisionSlot {
    pub async fn commit_npc_marriage<C: ItemLoopContent>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut fence: CurrentCharacterGameplayFence,
        request: MarriageRequest,
        content: &C,
    ) -> Result<MarriageOutcome, CharacterProgressionError> {
        if self.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        fence.expected_character_revision = self
            .cursor(root, authority)
            .await
            .map_err(CharacterProgressionError::from_authority)?;
        let result = root
            .commit_npc_marriage(authority, node, fence, request, content)
            .await;
        match &result {
            Ok(MarriageOutcome::Committed(receipt)) => {
                *self.cursor = Some(
                    self.cursor
                        .as_ref()
                        .copied()
                        .filter(|current| current.get() > receipt.committed_revision.get())
                        .unwrap_or(receipt.committed_revision),
                )
            }
            Err(_) => *self.cursor = None,
            Ok(MarriageOutcome::Refused(_)) => (),
        }
        result
    }
}
