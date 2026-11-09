//! Concrete adapter of the existing Instance kernel and shared durable activity owner.
use super::super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use super::runtime::*;
use super::scene::NativeScene;
use crate::durability::soulpit::{
    SoulpitScopeBinding, SoulpitStepOutcome, SoulpitStepRequest, SoulpitStoredEncounter,
};
use crate::foundation::ChannelRuntimeV1;
use tokio::sync::MutexGuard;

pub(super) struct NativeOwner<'p, 'a, 'f, 's> {
    pub(super) view: &'p ComposedFreshAdmission<'a, 'f, 's>,
    pub(super) bound: SoulpitScopeBinding,
    pub(super) binding: SoulpitActivityBinding,
    pub(super) scene: NativeScene,
    pub(super) survivors:
        Vec<crate::durability::character_progression::CurrentCharacterGameplayFence>,
    pub(super) permit: Option<crate::durability::spell_owner_commit::SpellLanePermit>,
    pub(super) runtime: Option<MutexGuard<'p, ChannelRuntimeV1>>,
    pub(super) spells: Option<MutexGuard<'p, ChannelSpellStates>>,
}
impl<'p, 'a, 'f, 's> NativeOwner<'p, 'a, 'f, 's> {
    fn release_kernel(&mut self) {
        self.spells.take();
        self.runtime.take();
    }
    async fn acquire_kernel(&mut self) -> Result<(), SoulpitRuntimeError> {
        self.runtime = Some(self.view.runtime.lock().await);
        self.spells = Some(self.view.states.spell_states.lock().await);
        if self
            .runtime
            .as_ref()
            .ok_or(SoulpitRuntimeError::InvalidState)?
            .owner_fence()
            .is_err()
        {
            self.release_kernel();
            return Err(SoulpitRuntimeError::InvalidScope);
        }
        Ok(())
    }
}
impl<'p, 'a, 'f, 's> ActivityHandoffOwnerRuntime for NativeOwner<'p, 'a, 'f, 's> {
    fn binding(&self) -> SoulpitActivityBinding {
        self.binding
    }
    fn kernel(&mut self) -> Result<SoulpitKernel<'_>, SoulpitRuntimeError> {
        let runtime = self
            .runtime
            .as_mut()
            .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
        let (carrier, continuity, content) = runtime.soulpit_kernel_parts()?;
        let native = self
            .view
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(SoulpitRuntimeError::MissingContent)?;
        Ok(SoulpitKernel {
            carrier,
            continuity,
            content,
            native,
            scene: &self.scene,
        })
    }
    fn survivors(
        &self,
    ) -> Vec<crate::durability::character_progression::CurrentCharacterGameplayFence> {
        self.survivors.clone()
    }
    fn read_encounter(&mut self) -> ActivityFuture<'_, SoulpitStoredEncounter> {
        Box::pin(async move {
            self.release_kernel();
            let mut value = self
                .view
                .root
                .read_soulpit_encounter(
                    self.view.character,
                    self.view.holder,
                    self.binding.handoff_id,
                    self.binding.scope,
                    self.binding.generation,
                )
                .await?
                .ok_or(SoulpitRuntimeError::InvalidState)?;
            self.survivors = self.view.current_soulpit_survivors(&self.bound).await?;
            for member in &self.bound.members {
                let snapshot = value.encounter.snapshot();
                if snapshot.phase.is_terminal() {
                    break;
                }
                if snapshot.surviving_slots.contains(&member.slot)
                    && !self
                        .survivors
                        .iter()
                        .any(|f| f.game_session_id == member.destination_session)
                {
                    let request = super::runtime::request(
                        self.binding,
                        &value,
                        self.view
                            .owner_now()
                            .get()
                            .max(snapshot.last_observed_at_micros),
                        crate::durability::soulpit::SoulpitAction::Leave { slot: member.slot },
                        Vec::new(),
                    )?;
                    let outcome = self
                        .view
                        .root
                        .advance_soulpit(self.view.character, self.view.holder, request)
                        .await?;
                    value.revision = outcome.revision;
                    value.encounter = outcome.encounter;
                }
            }
            self.acquire_kernel().await?;
            Ok(value)
        })
    }
    fn commit_progress(
        &mut self,
        mut request: SoulpitStepRequest,
    ) -> ActivityFuture<'_, SoulpitStepOutcome> {
        Box::pin(async move {
            self.release_kernel();
            request
                .survivors
                .sort_by_key(|f| *f.character_id.as_bytes());
            let mut slots = Vec::with_capacity(request.survivors.len());
            for fence in &mut request.survivors {
                let mut slot = self
                    .view
                    .revision_sequencer
                    .acquire(fence.character_id)
                    .await;
                fence.expected_character_revision = slot
                    .cursor(self.view.root, self.view.character)
                    .await
                    .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
                slots.push(slot);
            }
            self.survivors = request.survivors.clone();
            let result = self
                .view
                .root
                .advance_soulpit(self.view.character, self.view.holder, request)
                .await;
            #[cfg(test)]
            eprintln!("SRX3_SOULPIT PROGRESS_RESULT {result:?}");
            // Completion owns Animus and may have advanced every survivor's root.
            // Unknown outcomes also invalidate cursors before another descendant writer.
            for slot in &mut slots {
                slot.observe_endgame_progression();
            }
            drop(slots);
            self.acquire_kernel().await?;
            result.map_err(Into::into)
        })
    }
    fn register_creatures(
        &mut self,
        actors: &[oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveActor],
    ) -> Result<(), SoulpitRuntimeError> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
        let spells = self
            .spells
            .as_mut()
            .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
        let native = self
            .view
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(SoulpitRuntimeError::MissingContent)?;
        for actor in actors {
            if spells
                .monster_melee
                .creature_ai
                .state(actor.actor)
                .is_some()
            {
                continue;
            }
            let snapshot = runtime.companion_snapshot(actor.actor)?;
            let creature = crate::content::ProjectV2DefinitionRef {
                family: crate::content::ProjectV2Family::Creature,
                key: snapshot.state.policy.definition_key.clone(),
                revision: snapshot.state.policy.definition_revision.clone(),
            };
            spells
                .monster_melee
                .creature_ai
                .admit_native_creature(
                    runtime,
                    native,
                    actor.actor,
                    &creature,
                    self.view.owner_now().get(),
                )
                .map_err(|_| SoulpitRuntimeError::InvalidState)?;
        }
        Ok(())
    }
    fn settle_boss_rewards(&mut self, reward: SoulpitBossReward) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            self.release_kernel();
            // The lethal native callback freezes equipment, contributors and party
            // dependencies into this same queue before any cadence can retire the body.
            let _ = reward;
            #[cfg(test)]
            eprintln!("SRX3_SOULPIT TERMINAL_STAGE drain_boss_rewards");
            self.permit.take();
            for member in &self.bound.members {
                if self
                    .view
                    .drain_kill_rewards(member.destination_session)
                    .await
                    == super::super::kill_reward::DrainOutcome::Deferred
                {
                    return Err(SoulpitRuntimeError::AdapterUnavailable);
                }
            }
            let permit = self
                .view
                .spell_lane_permit()
                .await
                .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            self.scene = self
                .view
                .prepare_soulpit_scene(&permit)
                .await
                .map_err(|_| {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT TERMINAL_REFUSAL reward_scene");
                    SoulpitRuntimeError::AdapterUnavailable
                })?;
            self.permit = Some(permit);
            self.acquire_kernel().await
        })
    }
    fn return_participants(
        &mut self,
        _phase: oteryn_gameplay_domain::domain::soulpit::EncounterPhase,
    ) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            self.release_kernel();
            self.permit.take();
            #[cfg(test)]
            eprintln!("SRX3_SOULPIT TERMINAL_STAGE return_participants");
            for member in &self.bound.members {
                let Some(admitted) = self
                    .view
                    .activity_runtimes
                    .connection(member.destination_session)
                else {
                    continue;
                };
                let admitted = self.view.activity_runtimes.current_connection(admitted);
                if admitted.item_fence.is_none_or(|f| {
                    f.runtime_scope != self.binding.scope
                        || f.scope_ownership_generation != self.binding.generation
                }) {
                    continue;
                }
                match self
                    .view
                    .return_due_activity(
                        admitted,
                        crate::durability::activity_handoff::ActivityReturnReason::Defeated,
                    )
                    .await
                    .map_err(|_| {
                        #[cfg(test)]
                        eprintln!("SRX3_SOULPIT TERMINAL_REFUSAL return_due_activity");
                        SoulpitRuntimeError::AdapterUnavailable
                    })? {
                    super::super::native_activity::NativeActivityOutcome::Committed(commit) => self
                        .view
                        .publish_activity_commit(&commit)
                        .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?,
                    super::super::native_activity::NativeActivityOutcome::Refused(_) => {
                        return Err(SoulpitRuntimeError::InvalidState);
                    }
                }
            }
            self.acquire_kernel().await
        })
    }
}
