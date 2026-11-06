//! Source waves and physical rewards below #2769's ActivityHandoffOwner. This adapter
//! receives an already allocated, accepted Instance; it cannot create a scope or session.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{
    SoulpitAction, SoulpitStepOutcome, SoulpitStepRequest, SoulpitStoredEncounter,
    SoulpitWriteError,
};
use crate::foundation::{
    ChannelContentPin, ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::soulpit::{
    BossAbility, EncounterEffect, EncounterPhase, MONSTERS_PER_WAVE, SPAWN_DELAY_MICROS,
    SoulpitStack, TIME_LIMIT_MICROS, wave_stacks,
};
use oteryn_gameplay_entities::carrier::soulpit::{
    SoulpitDeathFacts, SoulpitOppressorCast, SoulpitWaveActor, SoulpitWaveRequest,
};
use oteryn_gameplay_entities::carrier::{
    CarrierError, ChannelActorCarrier, NamespaceContinuityGuard,
};
use std::{future::Future, pin::Pin};

pub type ActivityFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, SoulpitRuntimeError>> + Send + 'a>>;

#[derive(Debug)]
pub enum SoulpitRuntimeError {
    Carrier(CarrierError),
    Durable(SoulpitWriteError),
    InvalidScope,
    MissingContent,
    InvalidState,
    AdapterUnavailable,
}
impl From<CarrierError> for SoulpitRuntimeError {
    fn from(value: CarrierError) -> Self {
        Self::Carrier(value)
    }
}
impl From<SoulpitWriteError> for SoulpitRuntimeError {
    fn from(value: SoulpitWriteError) -> Self {
        Self::Durable(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SoulpitActivityBinding {
    pub handoff_id: [u8; 16],
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
}

mod decisions;
use decisions::draw;
pub(in crate::gameplay_transport) use decisions::occurrence;
pub(super) use decisions::request;
mod spawn_plan;
pub(in crate::gameplay_transport) use spawn_plan::{source_spawn_tile, source_wave_positions};

/// Current native map and AI observations from the shared runtime, not client coordinates.
pub trait ActivityHandoffScene: Sync {
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool;
    fn attack_target(&self, source: ExactActorRef) -> Option<ExactActorRef>;
    fn permits_condition(
        &self,
        source: ExactActorRef,
        target: ExactActorRef,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> bool;
    /// Existing PlayerSpellState's current saved Attributes delta, read under the same
    /// kernel lock. Do not include canonical Hex: refresh removes its old change first.
    fn player_attribute_damage_delta(
        &self,
        target: ExactActorRef,
        session: GameSessionId,
        now_micros: u64,
    ) -> Option<i32>;
}

/// A borrow of the existing shared physical owner. Its continuity and artifact pins remain
/// authoritative; callers must release this borrow before any durable await.
pub struct SoulpitKernel<'a> {
    pub carrier: &'a mut ChannelActorCarrier,
    pub continuity: &'a NamespaceContinuityGuard,
    pub content: &'a ChannelContentPin,
    pub native: &'a NativeGameplayState,
    pub scene: &'a dyn ActivityHandoffScene,
}

/// The shared activity integration port uses the existing atomic Soulpit entry,
/// ActivityHandoffOwner::return_participant/recover, Instance runtime, XP sequencer
/// and WeaponProficiencyDispatch. No method here grants authority from a handoff UUID.
pub trait ActivityHandoffOwnerRuntime: Send {
    fn binding(&self) -> SoulpitActivityBinding;
    fn kernel(&mut self) -> Result<SoulpitKernel<'_>, SoulpitRuntimeError> {
        Err(SoulpitRuntimeError::AdapterUnavailable)
    }
    /// The production adapter locks the physical runtime only for this synchronous turn.
    fn with_kernel<'a, T: Send + 'a>(
        &'a mut self,
        operation: impl for<'k> FnOnce(SoulpitKernel<'k>) -> Result<T, SoulpitRuntimeError> + Send + 'a,
    ) -> ActivityFuture<'a, T> {
        Box::pin(async move { operation(self.kernel()?) })
    }
    fn survivors(&self) -> Vec<CurrentCharacterGameplayFence>;
    fn read_encounter(&mut self) -> ActivityFuture<'_, SoulpitStoredEncounter>;
    /// Calls DurabilityRoot::advance_soulpit with current reconciled authority/node and
    /// all survivor revision slots held. The writer independently proves the Instance.
    fn commit_progress(
        &mut self,
        request: SoulpitStepRequest,
    ) -> ActivityFuture<'_, SoulpitStepOutcome>;
    /// Register each successful exact actor in the already existing native AI/profile owner.
    /// Repeating these identities reconciles the same registration, never creates another actor.
    fn register_creatures<'a>(
        &'a mut self,
        actors: &'a [SoulpitWaveActor],
    ) -> ActivityFuture<'a, ()>;
    /// The existing XP/party and proficiency owners consume this physical death. Equipment
    /// and contributor facts are frozen by their lethal callback; ordinary loot, Bestiary,
    /// Task Board, Forge and ordinary per-kill proficiency are deliberately absent.
    fn settle_boss_rewards(&mut self, reward: SoulpitBossReward) -> ActivityFuture<'_, ()>;
    /// Retains each remaining participant's owned shared-return intention. The world
    /// cadence executes the generic owner after this dynamic turn. Completion has
    /// already credited Animus in the lifecycle transaction; timeout/abort never does.
    fn return_participants(&mut self, phase: EncounterPhase) -> ActivityFuture<'_, ()>;
}

#[derive(Debug, Clone)]
pub struct SoulpitBossReward {
    pub occurrence: [u8; 16],
    pub death_at_ms: u64,
    pub species: String,
    pub experience: u64,
    pub death: SoulpitDeathFacts,
}

impl SoulpitBossReward {
    /// Current-instance callers pass the existing proficiency owner's frozen contributor
    /// facts. This uses its normal duplicate/window/weapon checks and checkpoint ledger.
    pub fn credit_weapon(
        &self,
        owner: &mut super::super::weapon_proficiency::WeaponProficiencyDispatch,
        session: crate::foundation::GameSessionId,
        contributor: crate::domain::weapon_proficiency::ProficiencyKillContributor<'_>,
        daily_streak: u32,
    ) -> Option<u32> {
        super::super::kill_reward::credit_soulpit_weapon_proficiency(
            owner,
            self,
            session,
            contributor,
            daily_streak,
        )
    }
}

fn update(stored: &mut SoulpitStoredEncounter, outcome: &SoulpitStepOutcome) {
    stored.revision = outcome.revision;
    stored.encounter = outcome.encounter.clone();
}

/// The source grants stack40 XP/weapon points at the physical kill, independently of
/// the wave-clearance tick or the survival reward. A precise timeout/recovery pass must
/// drain these frozen lethal receipts before it removes actors. The existing reward
/// owner deduplicates this same occurrence if a later durable step/return is retried.
pub(super) async fn settle_lethal_bosses(
    owner: &mut impl ActivityHandoffOwnerRuntime,
    binding: SoulpitActivityBinding,
) -> Result<(), SoulpitRuntimeError> {
    let rewards = owner
        .with_kernel(|kernel| {
            let mut rewards = Vec::new();
            for actor in kernel
                .carrier
                .soulpit_actors(kernel.continuity, binding.handoff_id)?
            {
                if actor.health != 0 || actor.tag.stack != SoulpitStack::Boss || actor.tag.is_summon
                {
                    continue;
                }
                let death = kernel
                    .carrier
                    .capture_soulpit_death(kernel.continuity, actor.actor)?;
                let base = kernel
                    .native
                    .reward_table()
                    .encounter_facts(&actor.species)
                    .and_then(|facts| facts.experience)
                    .ok_or(SoulpitRuntimeError::MissingContent)?;
                rewards.push(SoulpitBossReward {
                    occurrence: occurrence(binding, b"boss-kill", death.tag.monster_id),
                    death_at_ms: death.death_at_ms.ok_or(SoulpitRuntimeError::InvalidState)?,
                    species: actor.species,
                    experience: death.tag.experience(base)?,
                    death,
                });
            }
            Ok(rewards)
        })
        .await?;
    for reward in rewards {
        owner.settle_boss_rewards(reward).await?;
    }
    Ok(())
}

/// One source global encounter tick (one second in encounters.lua). Its async work calls
/// only the shared owner's durable seams; no physical borrow survives an await.
pub async fn drive_soulpit_activity(
    owner: &mut impl ActivityHandoffOwnerRuntime,
    now_micros: u64,
) -> Result<Option<u64>, SoulpitRuntimeError> {
    drive(owner, now_micros, true).await
}

/// Independent addEvent deadlines never run the source's global wave-clearance tick.
pub async fn drive_soulpit_deadlines(
    owner: &mut impl ActivityHandoffOwnerRuntime,
    now_micros: u64,
) -> Result<Option<u64>, SoulpitRuntimeError> {
    drive(owner, now_micros, false).await
}

async fn drive(
    owner: &mut impl ActivityHandoffOwnerRuntime,
    now_micros: u64,
    global_tick: bool,
) -> Result<Option<u64>, SoulpitRuntimeError> {
    let binding = owner.binding();
    if !matches!(binding.scope, RuntimeScopeRefV1::Instance { .. }) || binding.handoff_id == [0; 16]
    {
        return Err(SoulpitRuntimeError::InvalidScope);
    }
    owner
        .with_kernel(|kernel| {
            if kernel.carrier.encounter_scope() != (binding.scope, binding.generation)
                || kernel.native.policies().source_digest()
                    != kernel.content.server_artifact_digest()
            {
                return Err(SoulpitRuntimeError::InvalidScope);
            }
            Ok(())
        })
        .await?;
    let mut stored = owner.read_encounter().await?;
    settle_lethal_bosses(owner, binding).await?;
    let mut effect = EncounterEffect::NoChange;
    if global_tick && !stored.encounter.snapshot().phase.is_terminal() {
        // A failed Spawned acknowledgement leaves the physical actors registered while
        // the durable wave is still Preparing. Keep their deaths and summons intact until
        // the same spawn receipt is acknowledged; reconciliation requires Fighting.
        let mut actors = if stored.encounter.snapshot().phase == EncounterPhase::Fighting {
            owner
                .with_kernel(|kernel| {
                    kernel
                        .carrier
                        .cascade_soulpit_summon_deaths(kernel.continuity, binding.handoff_id)?;
                    Ok(kernel
                        .carrier
                        .soulpit_actors(kernel.continuity, binding.handoff_id)?)
                })
                .await?
        } else {
            Vec::new()
        };
        actors.sort_by_key(|actor| !actor.tag.is_summon);
        // Source summons count for clearance. A summon receives its original stack, no
        // boss ability/XP/proficiency and no extra ordinary progression callback.
        for actor in &actors {
            if stored.encounter.snapshot().phase.is_terminal() {
                break;
            }
            if actor.tag.wave != stored.encounter.snapshot().wave {
                continue;
            }
            if actor.tag.is_summon && stored.encounter.monster(actor.tag.monster_id).is_none() {
                let action = SoulpitAction::Summoned {
                    wave: actor.tag.wave,
                    monster_id: actor.tag.monster_id,
                    stack: actor.tag.stack.get(),
                };
                let step = request(binding, &stored, now_micros, action, owner.survivors())?;
                let outcome = owner.commit_progress(step).await?;
                update(&mut stored, &outcome);
                effect = outcome.effect;
            }
        }
        for actor in actors {
            if stored.encounter.snapshot().phase.is_terminal() {
                break;
            }
            if actor.health != 0 {
                continue;
            }
            if actor.tag.wave < stored.encounter.snapshot().wave
                || stored.encounter.monster(actor.tag.monster_id).is_none()
            {
                owner
                    .with_kernel(|kernel| {
                        Ok(kernel
                            .carrier
                            .retire_soulpit_defeated(kernel.continuity, actor.actor)?)
                    })
                    .await?;
                continue;
            }
            let action = SoulpitAction::Defeated {
                wave: actor.tag.wave,
                monster_id: actor.tag.monster_id,
            };
            let step = request(binding, &stored, now_micros, action, owner.survivors())?;
            let outcome = owner.commit_progress(step).await?;
            update(&mut stored, &outcome);
            effect = outcome.effect;
            owner
                .with_kernel(|kernel| {
                    Ok(kernel
                        .carrier
                        .retire_soulpit_defeated(kernel.continuity, actor.actor)?)
                })
                .await?;
        }
        if !stored.encounter.snapshot().phase.is_terminal()
            && matches!(effect, EncounterEffect::NoChange)
        {
            let step = request(
                binding,
                &stored,
                now_micros,
                SoulpitAction::Advance,
                owner.survivors(),
            )?;
            let outcome = owner.commit_progress(step).await?;
            update(&mut stored, &outcome);
            effect = outcome.effect;
        }
    }
    if !global_tick && !stored.encounter.snapshot().phase.is_terminal() {
        let snapshot = stored.encounter.snapshot();
        let spawn_due = snapshot.phase == EncounterPhase::Preparing
            && now_micros.saturating_sub(snapshot.wave_started_at_micros) >= SPAWN_DELAY_MICROS;
        let timeout_due =
            now_micros.saturating_sub(snapshot.started_at_micros) >= TIME_LIMIT_MICROS;
        if spawn_due || timeout_due {
            let step = request(
                binding,
                &stored,
                now_micros,
                SoulpitAction::Advance,
                owner.survivors(),
            )?;
            let outcome = owner.commit_progress(step).await?;
            update(&mut stored, &outcome);
            effect = outcome.effect;
        }
    }
    if let EncounterEffect::SpawnWave { wave, stacks } = effect {
        if stacks != wave_stacks(wave).map_err(|_| SoulpitRuntimeError::InvalidState)? {
            return Err(SoulpitRuntimeError::InvalidState);
        }
        let actors = owner
            .with_kernel(|kernel| {
                let root = kernel.content.server_artifact_digest();
                let id = occurrence(binding, b"wave", u64::from(wave));
                let retained = kernel.carrier.retained_soulpit_wave_request(
                    kernel.continuity,
                    binding.handoff_id,
                    wave,
                    root,
                )?;
                let request = if let Some(prior) = retained {
                    if prior.species != stored.species {
                        return Err(SoulpitRuntimeError::InvalidState);
                    }
                    prior
                } else {
                    SoulpitWaveRequest {
                        handoff_id: binding.handoff_id,
                        wave,
                        species: stored.species.clone(),
                        positions: source_wave_positions(root, binding, wave, |at| {
                            kernel.scene.permits_spawn(at)
                        })?,
                        boss_ability: BossAbility::from_roll(draw(
                            root,
                            id,
                            "soulpit_boss_ability",
                            6,
                            1,
                            3,
                        )? as u8)
                        .map_err(|_| SoulpitRuntimeError::InvalidState)?,
                        now_micros,
                    }
                };
                Ok(kernel.carrier.spawn_soulpit_wave(
                    kernel.continuity,
                    kernel.content,
                    kernel.native.policies(),
                    request,
                    |position| kernel.scene.permits_spawn(position),
                )?)
            })
            .await?;
        owner
            .register_creatures(&actors.iter().flatten().copied().collect::<Vec<_>>())
            .await?;
        let action = SoulpitAction::Spawned {
            wave,
            monster_ids: actors.map(|actor| actor.map(|actor| actor.monster_id)),
        };
        let step = request(binding, &stored, now_micros, action, owner.survivors())?;
        let outcome = owner.commit_progress(step).await?;
        update(&mut stored, &outcome);
    }
    let phase = stored.encounter.snapshot().phase;
    if phase.is_terminal() {
        owner
            .with_kernel(|kernel| {
                Ok(kernel
                    .carrier
                    .retire_soulpit(kernel.continuity, binding.handoff_id)?)
            })
            .await?;
        owner.return_participants(phase).await?;
        return Ok(None);
    }
    // Oppressor's own two-second cadence is in its actual actor tag. The source AI selects
    // its target; the shared map supplies current wall, life and combat legality.
    owner
        .with_kernel(|kernel| {
            for actor in kernel
                .carrier
                .soulpit_actors(kernel.continuity, binding.handoff_id)?
            {
                let Some(boss) = actor
                    .tag
                    .boss
                    .filter(|boss| boss.ability == BossAbility::Oppressor)
                else {
                    continue;
                };
                if actor.health <= 0 || now_micros < boss.next_oppressor_at_micros {
                    continue;
                }
                let root = kernel.content.server_artifact_digest();
                let id = occurrence(binding, b"oppressor", boss.ordinal);
                let mut draws = [0; 3];
                for (ordinal, value) in draws.iter_mut().enumerate() {
                    *value = draw(root, id, "soulpit_oppressor", ordinal as u64, 1, 100)? as u8;
                }
                let cast = SoulpitOppressorCast {
                    source: actor.actor,
                    current_target: kernel.scene.attack_target(actor.actor),
                    now_micros,
                    draws,
                    decision_root: root,
                };
                kernel.carrier.drive_soulpit_oppressor(
                    kernel.continuity,
                    cast,
                    |target, _, from, to| {
                        kernel
                            .scene
                            .permits_condition(actor.actor, target, from, to)
                    },
                    |target, session| {
                        kernel
                            .scene
                            .player_attribute_damage_delta(target, session, now_micros)
                            .ok_or(CarrierError::InvalidCommitBinding)
                    },
                )?;
            }
            next_soulpit_deadline(binding, &stored, &kernel)
        })
        .await
}

/// A restart never recreates the source's process-local timers or monsters. The shared
/// recovery owner uses this same fenced writer then its existing return/recovery protocol.
pub async fn interrupt_soulpit_activity(
    owner: &mut impl ActivityHandoffOwnerRuntime,
    now_micros: u64,
) -> Result<(), SoulpitRuntimeError> {
    let binding = owner.binding();
    if !matches!(binding.scope, RuntimeScopeRefV1::Instance { .. }) || binding.handoff_id == [0; 16]
    {
        return Err(SoulpitRuntimeError::InvalidScope);
    }
    owner
        .with_kernel(|kernel| {
            if kernel.carrier.encounter_scope() != (binding.scope, binding.generation)
                || kernel.native.policies().source_digest()
                    != kernel.content.server_artifact_digest()
            {
                return Err(SoulpitRuntimeError::InvalidScope);
            }
            Ok(())
        })
        .await?;
    let stored = owner.read_encounter().await?;
    settle_lethal_bosses(owner, binding).await?;
    if stored.encounter.snapshot().phase.is_terminal() {
        owner
            .with_kernel(|kernel| {
                Ok(kernel
                    .carrier
                    .retire_soulpit(kernel.continuity, binding.handoff_id)?)
            })
            .await?;
        return owner
            .return_participants(stored.encounter.snapshot().phase)
            .await;
    }
    let step = request(
        binding,
        &stored,
        now_micros,
        SoulpitAction::Interrupt,
        owner.survivors(),
    )?;
    let outcome = owner.commit_progress(step).await?;
    owner
        .with_kernel(|kernel| {
            Ok(kernel
                .carrier
                .retire_soulpit(kernel.continuity, binding.handoff_id)?)
        })
        .await?;
    owner
        .return_participants(outcome.encounter.snapshot().phase)
        .await
}

/// Precise source addEvent deadlines, independently of the source one-second global tick.
/// The shared owner returns their minimum to its existing world cadence after every pass.
pub fn next_soulpit_deadline(
    binding: SoulpitActivityBinding,
    stored: &SoulpitStoredEncounter,
    kernel: &SoulpitKernel<'_>,
) -> Result<Option<u64>, SoulpitRuntimeError> {
    if kernel.carrier.encounter_scope() != (binding.scope, binding.generation) {
        return Err(SoulpitRuntimeError::InvalidScope);
    }
    let snapshot = stored.encounter.snapshot();
    if snapshot.phase.is_terminal() {
        return Ok(None);
    }
    let timeout = snapshot
        .started_at_micros
        .checked_add(TIME_LIMIT_MICROS)
        .ok_or(SoulpitRuntimeError::InvalidState)?;
    let mut due = timeout;
    if snapshot.phase == EncounterPhase::Preparing {
        due = due.min(
            snapshot
                .wave_started_at_micros
                .checked_add(SPAWN_DELAY_MICROS)
                .ok_or(SoulpitRuntimeError::InvalidState)?,
        );
    }
    // A boss without a target has no active attack deadline. The next source global/AI
    // pass observes a newly selected target without creating a busy overdue timer.
    for actor in kernel
        .carrier
        .soulpit_actors(kernel.continuity, binding.handoff_id)?
    {
        let Some(boss) = actor
            .tag
            .boss
            .filter(|boss| boss.ability == BossAbility::Oppressor)
        else {
            continue;
        };
        if actor.health > 0 && kernel.scene.attack_target(actor.actor).is_some() {
            due = due.min(boss.next_oppressor_at_micros);
        }
    }
    Ok(Some(due))
}

#[cfg(test)]
#[path = "runtime/tests.rs"]
mod tests;
