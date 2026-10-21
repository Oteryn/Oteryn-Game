//! Transport adapters for the source-qualified Herald encounter owner.
use super::ComposedFreshAdmission;
use crate::crystal_death_composition::{
    CrystalDeathCompositionError, CrystalDeathOwner, QualifiedCrystalDeaths,
};
use crate::crystal_herald_quest::HeraldQuestError;
use crate::durability::quest_state::QuestTransitionOutcome;
use crate::foundation::{ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence};
pub use oteryn_gameplay_encounters::bosses::herald_of_fire::death_dispatch::CrystalDeathQuestOutcome;
use oteryn_gameplay_encounters::bosses::herald_of_fire::death_dispatch::HeraldDeathContext;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub async fn consume_herald_committed_death(
        &self,
        qualified: &QualifiedCrystalDeaths,
        source_fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Option<QuestTransitionOutcome>, HeraldQuestError> {
        HeraldDeathContext {
            root: self.root,
            character: self.character,
            holder: self.holder,
            runtime: self.runtime,
            revision_sequencer: &self.revision_sequencer,
        }
        .consume_herald_committed_death(
            qualified,
            source_fence,
            stamp,
            actor,
            self.current_quest_fence(session),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn project_and_consume_crystal_death(
        &self,
        owner: &mut CrystalDeathOwner,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
        current_recipient_session: GameSessionId,
    ) -> Result<CrystalDeathQuestOutcome, CrystalDeathCompositionError> {
        HeraldDeathContext {
            root: self.root,
            character: self.character,
            holder: self.holder,
            runtime: self.runtime,
            revision_sequencer: &self.revision_sequencer,
        }
        .project_and_consume_crystal_death(
            owner,
            current,
            stamp,
            actor,
            listeners,
            server_rum_draw,
            self.current_quest_fence(current_recipient_session),
        )
        .await
    }
}
