use super::*;
use crate::durability::arena_match::MatchTransition;

impl RevisionSlot {
    /// Commit arena transitions and their entry fee under the Character revision slot.
    pub async fn commit_arena_match(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut fence: CurrentCharacterGameplayFence,
        request: MatchTransition,
    ) -> Result<Vec<u8>, CharacterProgressionError> {
        if self.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        fence.expected_character_revision = self
            .cursor(root, authority)
            .await
            .map_err(CharacterProgressionError::from_authority)?;
        // Arena returns wire bytes rather than a revision receipt. Reload after
        // success, replay or an unknown outcome before the next queued write.
        *self.cursor = None;
        root.commit_arena_match(authority, node, fence, request)
            .await
    }
}
