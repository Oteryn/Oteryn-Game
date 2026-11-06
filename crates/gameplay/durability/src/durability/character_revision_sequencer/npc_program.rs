//! Source NPC storage writes use the existing Character cursor and retry contract.
use super::super::quest_state::npc_program::{NpcProgramCommit, NpcProgramRequest};
use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the sequenced NPC transaction carries distinct authority, fence, catalogue and item owners"
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
