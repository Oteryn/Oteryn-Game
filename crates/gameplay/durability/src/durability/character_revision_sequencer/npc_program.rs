//! Source NPC storage writes use the existing Character cursor and retry contract.
use super::super::quest_state::npc_program::{NpcProgramCommit, NpcProgramRequest};
use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the sequencer passes authority, fence, catalogue and the item owner independently"
    )]
    pub async fn commit_npc_program<C: super::super::item_loop_move::ItemLoopContent>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_npc_program(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    items,
                )
            },
        )
        .await
    }
}

impl SequencedOutcome for NpcProgramCommit {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        self.transition.committed_revision()
    }
}
//! Source NPC storage writes use the existing Character cursor and retry contract.
use super::super::quest_state::npc_program::{NpcProgramCommit, NpcProgramRequest};
use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "The sequenced NPC writer keeps its root, authority, node, fence and native content explicit, matching the underlying durable writer."
    )]
    pub async fn commit_npc_program<C: super::super::item_loop_move::ItemLoopContent>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.commit_npc_program_with_premium(
            root,
            authority,
            node,
            fence,
            request,
            catalogue,
            items,
            &super::super::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
    }

    /// Serialize an NPC write with a transaction-bound native Premium participant.
    #[allow(
        clippy::too_many_arguments,
        reason = "The sequenced NPC transaction names its native Premium participant alongside existing authority."
    )]
    pub async fn commit_npc_program_with_premium<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        premium: &G,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_npc_program_with_premium(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    items,
                    premium,
                )
            },
        )
        .await
    }
}

impl SequencedOutcome for NpcProgramCommit {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        self.transition.committed_revision()
    }
}
