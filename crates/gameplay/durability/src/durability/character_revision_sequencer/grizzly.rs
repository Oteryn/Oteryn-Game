use super::*;
use oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction;
use super::super::quest_state::grizzly::GrizzlyPremiumGate;
use super::super::quest_state::QuestCause;

impl RevisionSlot {
    /// The existing native progression policy supplies only the point-cap projection;
    /// XP remains an obligation consumed by the existing XP owner after this batch.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_grizzly_transition_with_progression<G: GrizzlyPremiumGate, const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        gate: &G,
        policy: Option<&crate::domain::progression::FiniteProgressionPolicy<String, N>>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(root_revision(root, authority), fence, Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_character_grizzly_transition_with_progression(authority, node, fence,
                cause, action.clone(), Arc::clone(&catalogue), gate, policy)).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn commit_grizzly_boss_entry(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_state::grizzly_boss::GrizzlyBossEntryContext,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(root_revision(root, authority), fence, Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_character_grizzly_boss_entry(authority, node, fence, cause,
                Arc::clone(&catalogue), permit, context.clone())).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn commit_grizzly_kill(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        source: crate::foundation::PartySnapshot,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(root_revision(root, authority), fence, Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_character_grizzly_kill(authority, node, fence, cause,
                action.clone(), Arc::clone(&catalogue), source.clone())).await
    }

    /// A Grizzly action shares the quest revision cursor, including a retry after a
    /// revision race. The locked writer evaluates all conditions again on that retry.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_grizzly_transition<G: GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_grizzly_transition(
                    authority,
                    node,
                    fence,
                    cause,
                    action.clone(),
                    Arc::clone(&catalogue),
                    gate,
                )
            },
        )
        .await
    }
}
