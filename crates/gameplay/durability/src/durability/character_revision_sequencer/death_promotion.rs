//! The death writer uses the same cursor while consuming per-use Premium evidence.
use super::*;

impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing death writer and its per-use Premium input"
    )]
    pub(in crate::durability) async fn commit_premium_death<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        formula: Option<&dyn BuildFormula>,
        original: Option<CharacterRevision>,
        premium: crate::durability::character_death::promotion::DeathPremiumState,
    ) -> Result<CharacterDeathOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_character_death_inner(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    formula,
                    premium,
                )
            },
        )
        .await
    }
}
