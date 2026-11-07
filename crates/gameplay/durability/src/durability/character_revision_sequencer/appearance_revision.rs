//! Appearance retains its original expected revision across commit and exact replay.
use super::*;
use crate::domain::appearance::AppearanceContent;
use crate::durability::character_appearance::{
    AppearanceChangeOutcome, AppearanceChangeRequest, AppearanceValidationContext,
    AppearanceWriteError,
};

impl RevisionSlot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_appearance<C: AppearanceContent<String> + Send + Sync + Clone + 'static>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AppearanceChangeRequest,
        validation: AppearanceValidationContext<C>,
    ) -> Result<AppearanceChangeOutcome, AppearanceWriteError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Replay(fence.expected_character_revision),
            |fence| {
                root.commit_character_appearance(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    AppearanceValidationContext {
                        sex: validation.sex,
                        content: validation.content.clone(),
                        content_revision: validation.content_revision.clone(),
                    },
                )
            },
        )
        .await
    }
}
impl SequencedOutcome for AppearanceChangeOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}
impl SequencedError for AppearanceWriteError {
    fn is_mismatch(&self) -> bool {
        matches!(
            self,
            Self::Fence(CharacterProgressionError::CharacterRevisionMismatch)
        )
    }
    fn is_unknown(&self) -> bool {
        matches!(
            self,
            Self::Unavailable(_) | Self::Fence(CharacterProgressionError::Unavailable(_))
        )
    }
    fn rejected() -> Self {
        Self::Fence(CharacterProgressionError::AuthorityRejected)
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
