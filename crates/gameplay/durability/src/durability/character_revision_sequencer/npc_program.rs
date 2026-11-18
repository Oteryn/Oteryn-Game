//! Source NPC storage writes use the existing Character cursor and retry contract.
use super::super::quest_state::npc_program::{NpcProgramCommit, NpcProgramRequest};
use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the sequencer passes authority, fence, catalogue and the item owner independently"
    )]
    pub async fn commit_npc_program<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        premium: &G,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.commit_npc_program_with_spawn(root, authority, node, permit, fence, request, catalogue, items, premium, None).await
    }

    #[allow(clippy::too_many_arguments, reason = "the revision cursor forwards the exact NPC spawn admission alongside the item participant")]
    pub async fn commit_npc_program_with_spawn<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        premium: &G,
        spawn: Option<&[oteryn_gameplay_quests::quest::world_callbacks::CreatureSpawn]>,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_npc_program_with_spawn(
                    authority,
                    node,
                    permit,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    items,
                    premium,
                    spawn,
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
