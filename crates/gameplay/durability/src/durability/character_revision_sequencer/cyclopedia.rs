use super::*;
use crate::durability::character_cyclopedia::{CyclopediaCommit, CyclopediaPremiumGate, MapRequest};

impl RevisionSlot {
    pub async fn cyclopedia<G: CyclopediaPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut fence: CurrentCharacterGameplayFence,
        request: MapRequest,
        gate: &G,
    ) -> Result<CyclopediaCommit, CharacterProgressionError> {
        if self.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        fence.expected_character_revision = self
            .cursor(root, authority)
            .await
            .map_err(CharacterProgressionError::from_authority)?;
        match root
            .commit_cyclopedia(authority, node, fence, request, gate)
            .await
        {
            Ok((revision, bytes, discovered_now)) => {
                *self.cursor = Some(
                    CharacterRevision::new(revision)
                        .map_err(|_| CharacterProgressionError::InvalidInput)?,
                );
                Ok(CyclopediaCommit { bytes, discovered_now })
            }
            Err(error) => {
                *self.cursor = None;
                Err(error)
            }
        }
    }
}
