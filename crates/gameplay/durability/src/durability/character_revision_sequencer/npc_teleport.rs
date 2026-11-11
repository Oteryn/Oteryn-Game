//! Source NPC teleport and progress share the ordinary Character revision cursor.
use super::*;
use crate::durability::quest_state::npc_program::{
    NpcProgramCommit, NpcProgramRequest, teleport::NpcTeleportAdmission,
};
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent current authority, item lane, source and destination admission"
    )]
    pub async fn commit_npc_teleport<
        C: crate::durability::item_loop_move::ItemLoopContent,
        G: crate::durability::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        premium: &G,
        admission: NpcTeleportAdmission,
    ) -> Result<NpcProgramCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_npc_teleport(
                    authority,
                    node,
                    permit,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    items,
                    premium,
                    admission,
                )
            },
        )
        .await
    }
}
