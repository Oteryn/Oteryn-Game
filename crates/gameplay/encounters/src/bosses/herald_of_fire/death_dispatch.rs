//! Source-specific physical-death consumer using the existing Character QuestState owner.
//! No runtime/party cache or inbox is a durable quest writer.
use crate::durability::quest_state::QuestTransitionOutcome;
use oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::QualifiedCrystalDeaths;
use oteryn_gameplay_encounters::bosses::herald_of_fire::crystal_herald_quest::HeraldQuestError;
use oteryn_gameplay_foundation::foundation::{
    ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};

use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
    character_progression::CurrentCharacterGameplayFence,
    character_revision_sequencer::CharacterRevisionSequencer,
    runtime_scope_assignment::NodeIncarnationProof,
};
use oteryn_gameplay_entities::ChannelRuntimeV1;
use std::future::Future;

/// Existing Character and Channel owners borrowed for a source-qualified death handoff.
pub struct HeraldDeathContext<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
    pub runtime: &'a tokio::sync::Mutex<ChannelRuntimeV1>,
    pub revision_sequencer: &'a CharacterRevisionSequencer,
}

impl HeraldDeathContext<'_, '_, '_> {
    /// Consume a source-qualified committed Herald death for the current top-damage session.
    /// PROJECT: party fan-out is deliberately not claimed. The SQL outcome is authoritative;
    /// no caller declares that quest progress or the separate escape portal was applied.
    pub async fn consume_herald_committed_death(
        &self,
        qualified: &QualifiedCrystalDeaths,
        source_fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        current_quest_fence: impl Future<Output = Result<Option<CurrentCharacterGameplayFence>, ()>>,
    ) -> Result<Option<QuestTransitionOutcome>, HeraldQuestError> {
        let current = current_quest_fence
            .await
            .map_err(|_| HeraldQuestError::FenceChanged)?
            .ok_or(HeraldQuestError::FenceChanged)?;
        let copy = self
            .root
            .read_character_quest_state(self.character, current.character_id)
            .await
            .map_err(HeraldQuestError::Writer)?;
        // A stable explicit source catalogue revision; unrelated monster stat edits do not
        // change existing Targuna quest pins. Existing incompatible definitions still refuse
        // in the original writer; this consumer never migrates or overwrites them.
        let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state(
            "source-herald-crystal00ce-r1",
        )
        .map_err(|_| HeraldQuestError::InvalidCatalogue)?;
        let request = {
            let mut runtime = self.runtime.lock().await;
            qualified
                .prepare_herald_quest_request(
                    &mut runtime,
                    source_fence,
                    stamp,
                    actor,
                    current,
                    &copy,
                    std::sync::Arc::new(catalogue.catalogue().clone()),
                )
                .map_err(|_| HeraldQuestError::SourceChanged)?
        }; // Runtime mutex released before CharacterRevisionSequencer acquisition and SQL.
        match request {
            Some(request) => request
                .commit(
                    self.revision_sequencer,
                    self.root,
                    self.character,
                    self.holder,
                    current,
                    source_fence,
                    self.runtime,
                    qualified,
                )
                .await
                .map(Some),
            None => Ok(None),
        }
    }
}

/// Result preserves a successfully projected physical death even when the independent
/// durable descendant refuses. No postprojection SQL error claims physical rollback.
#[derive(Debug)]
pub struct CrystalDeathQuestOutcome {
    pub projected: oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::CrystalProjectedDeath,
    pub herald: Option<Result<Option<QuestTransitionOutcome>, HeraldQuestError>>,
}
impl HeraldDeathContext<'_, '_, '_> {
    /// Canonical async owner entry: actual sealed projection, native callback routing,
    /// then the existing Character SQL writer BEFORE returning the death response.
    /// Ordinary death projector APIs currently have no shipping transport caller;
    /// this extends that owner boundary without inventing an alternate combat action.
    // Keep project_and_consume_crystal_death ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub async fn project_and_consume_crystal_death(
        &self,
        owner: &mut oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::CrystalDeathOwner,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
        current_quest_fence: impl Future<Output = Result<Option<CurrentCharacterGameplayFence>, ()>>,
    ) -> Result<
        CrystalDeathQuestOutcome,
        oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::CrystalDeathCompositionError,
    >{
        let (projected, is_herald) = {
            let mut runtime = self.runtime.lock().await;
            // Identity belongs to the actual native slot, not a supplied callback name.
            let is_herald = runtime
                .borrow_combat_death()
                .creature_target_identity(actor)
                .map_err(oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::CrystalDeathCompositionError::Owner)?
                == b"oteryn:creature.herald_of_fire";
            let projected = owner.project_and_dispatch(
                &mut runtime,
                current,
                stamp,
                actor,
                listeners,
                server_rum_draw,
            )?;
            (projected, is_herald)
        };
        let herald = if is_herald {
            Some(
                self.consume_herald_committed_death(
                    owner.qualified_registry(),
                    current,
                    stamp,
                    actor,
                    current_quest_fence,
                )
                .await,
            )
        } else {
            None
        };
        Ok(CrystalDeathQuestOutcome { projected, herald })
    }
}
