use super::super::character_atelier::{AtelierOutcome, AtelierRequest};
use super::*;

impl RevisionSlot {
    /// Atelier changes share the Character cursor and retry a revision race once.
    #[allow(clippy::too_many_arguments)]
    pub async fn atelier<G: super::super::character_wheel::WheelPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AtelierRequest,
        ruleset: Arc<WheelRuleset>,
        gate: &G,
    ) -> Result<AtelierOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_atelier(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&ruleset),
                    gate,
                )
            },
        )
        .await
    }
}
impl SequencedOutcome for AtelierOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed { revision, .. } | Self::AlreadyCommitted { revision, .. } => {
                Some(*revision)
            }
            Self::Refused => None,
        }
    }
}
