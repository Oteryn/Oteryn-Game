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
        request: AtelierRequest<'_>,
        ruleset: Arc<WheelRuleset>,
        gate: &G,
    ) -> Result<AtelierOutcome, CharacterProgressionError> {
        self.atelier_with_admission(root, authority, node, fence, request, ruleset, gate, &|| true)
            .await
    }

    /// Reuses the caller's one fresh-admission decision if the Character cursor retries.
    #[allow(clippy::too_many_arguments)]
    pub async fn atelier_with_admission<G: super::super::character_wheel::WheelPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AtelierRequest<'_>,
        ruleset: Arc<WheelRuleset>,
        gate: &G,
        admit: &(impl Fn() -> bool + Sync),
    ) -> Result<AtelierOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_atelier_with_admission(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&ruleset),
                    gate,
                    admit,
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
