//! COND-1c: one condition store in each occupied actor slot, under the existing owner fence.
//! A retry retains its prepared plan; it never replans an old occurrence at a new revision.

#[cfg(test)]
pub(crate) use super::super::exact_actor_test_ability::condition::{
    ApplicationFacts, AttributeModifier, AttributeModifiers, CombatSkill, ConditionDefinition,
    ConditionRefusal, ConditionSourceKind, ConditionStore, ConditionTick, ConditionType,
    ConditionValues, DamageSchedule, DamageSegment, DotElement, ExactSpeedRatio,
    RationalSpeedRange, SpeedRange, StatusKind, TickFacts, TickKind,
};
use super::*;
#[cfg(not(test))]
pub(crate) use crate::ability::condition::{
    ApplicationFacts, AttributeModifier, AttributeModifiers, CombatSkill, ConditionDefinition,
    ConditionRefusal, ConditionSourceKind, ConditionStore, ConditionTick, ConditionType,
    ConditionValues, DamageSchedule, DamageSegment, DotElement, ExactSpeedRatio,
    RationalSpeedRange, SpeedRange, StatusKind, TickFacts, TickKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, SemanticTimeMicros};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ActorConditionState {
    store: ConditionStore<ExactActorRef>,
    revision: u64,
    at: u64,
    receipt: Option<ConditionReceipt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PlayerRuntimeState {
    pub(super) control_loss: Option<ControlLossMark>,
    pub(super) conditions: ActorConditionState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CreatureCommitState {
    damage: DamageReceipts,
    pub(super) conditions: ActorConditionState,
}
impl std::ops::Deref for CreatureCommitState {
    type Target = DamageReceipts;
    fn deref(&self) -> &Self::Target {
        &self.damage
    }
}
impl std::ops::DerefMut for CreatureCommitState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.damage
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionSource {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: Option<GameSessionId>,
    pub(crate) kind: ConditionSourceKind,
}

pub(crate) enum ActorConditionTransition<'a> {
    Apply {
        definition: &'a ConditionDefinition,
        source: ConditionSource,
        immunities: &'a [ConditionType],
        facts: ApplicationFacts<'a>,
    },
    /// One source-bound compound effect, at most the canonical per-actor limit.
    ApplyBatch {
        definitions: &'a [ConditionDefinition],
        source: ConditionSource,
        immunities: &'a [ConditionType],
        facts: ApplicationFacts<'a>,
    },
    /// Exact type, not its shared conflict key (curing paralysis must retain haste).
    RemoveType {
        kind: ConditionType,
        source: ConditionSource,
        facts: ApplicationFacts<'a>,
    },
    Cleanse(ApplicationFacts<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConditionReceipt {
    occurrence: DecisionOccurrenceId,
    revision: u64,
    at: u64,
    source: Option<ConditionSource>,
}

/// Immutable value plan, never current actor/session authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActorConditionPlan {
    actor: ExactActorRef,
    session: Option<GameSessionId>,
    receipt: ConditionReceipt,
    next: ConditionStore<ExactActorRef>,
    applications: Vec<Result<u32, ConditionRefusal>>,
}
impl ActorConditionPlan {
    /// Retained source-order outcomes; policy refusals are explicit no-effects.
    pub(crate) fn applications(&self) -> &[Result<u32, ConditionRefusal>] {
        &self.applications
    }
}

/// Frozen projection only. The owning HP consumer must preflight its whole pass and publish
/// this slot plan with HP/death in one owner-held transaction, never commit it before HP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActorConditionTickPlan {
    condition: ActorConditionPlan,
    ticks: Vec<ConditionTick<ExactActorRef>>,
}
impl ActorConditionTickPlan {
    pub(crate) fn condition_plan(&self) -> &ActorConditionPlan {
        &self.condition
    }
    pub(crate) fn ticks(&self) -> &[ConditionTick<ExactActorRef>] {
        &self.ticks
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionOwnerError {
    Actor(CarrierError),
    FactsMismatch,
    TimeMismatch,
    StalePlan,
    RevisionExhausted,
    BatchSize,
    Condition(ConditionRefusal),
}

impl ActorConditionState {
    pub(super) fn die(&mut self) {
        self.store.clear_on_death();
        self.receipt = None;
        // A dead actor cannot reacquire mutation authority; a fresh slot gets fresh state.
        self.revision = self.revision.saturating_add(1);
    }
}

impl ChannelRuntimeV1 {
    fn condition_index(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
    ) -> Result<usize, ConditionOwnerError> {
        self.carrier
            .lookup(&self.continuity, actor.0)
            .map_err(ConditionOwnerError::Actor)?;
        match session {
            Some(session) => self
                .carrier
                .player_slot_index(&self.continuity, actor.0, session)
                .map_err(ConditionOwnerError::Actor),
            None => {
                let index = self
                    .carrier
                    .validate_ref(&self.continuity, actor.0)
                    .map_err(ConditionOwnerError::Actor)?;
                if matches!(self.carrier.slots[index], Slot::CreatureOccupied { .. }) {
                    Ok(index)
                } else {
                    Err(ConditionOwnerError::Actor(
                        CarrierError::PlayerReservationMismatch,
                    ))
                }
            }
        }
    }

    fn condition_state(&self, index: usize) -> &ActorConditionState {
        match &self.carrier.slots[index] {
            Slot::Occupied { lifecycle, .. } => &lifecycle.conditions,
            Slot::CreatureOccupied { committed, .. } => &committed.conditions,
            _ => unreachable!("validated occupied actor"),
        }
    }

    fn condition_source(
        &self,
        source: ConditionSource,
        target: ExactActorRef,
    ) -> Result<(), ConditionOwnerError> {
        self.condition_index(source.actor, source.session)?;
        let valid = match source.kind {
            ConditionSourceKind::Player => source.session.is_some(),
            ConditionSourceKind::Creature => source.session.is_none(),
            ConditionSourceKind::SelfUse => source.actor == target,
            // Spatial field provenance needs its owning continuation; this actor-source API
            // cannot manufacture it from a caller-selected actor.
            ConditionSourceKind::Field => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ConditionOwnerError::FactsMismatch)
        }
    }

    pub(crate) fn actor_conditions(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
    ) -> Result<&ConditionStore<ExactActorRef>, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        Ok(&self.condition_state(index).store)
    }

    /// Current source-store read for merged Creature buffs; stale owner time
    /// cannot turn a newer buff into a guessed expired/absent modifier.
    pub(crate) fn actor_conditions_at(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        now: u64,
    ) -> Result<&ConditionStore<ExactActorRef>, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        let current = self.condition_state(index);
        if now < current.at || !current.store.accepts_time(now) {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        Ok(&current.store)
    }
    /// Read the current actor's active contribution, never an expired stored speed.
    pub(crate) fn actor_active_speed_delta(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        now: SemanticTimeMicros,
    ) -> Result<i64, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        let current = self.condition_state(index);
        if now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        Ok(current.store.active_speed_delta(now.get()))
    }

    /// Prepare only non-ticking expiry under the same actor revision as other transitions.
    /// The owner retains this original plan for replay; no timer or tick consumer is added.
    pub(crate) fn prepare_actor_condition_expiry(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        occurrence: DecisionOccurrenceId,
        now: SemanticTimeMicros,
    ) -> Result<ActorConditionPlan, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        let current = self.condition_state(index);
        if now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        if current
            .receipt
            .is_some_and(|receipt| receipt.occurrence == occurrence)
        {
            return Err(ConditionOwnerError::StalePlan);
        }
        let mut next = current.store.clone();
        next.expire_non_ticking(now.get());
        Ok(ActorConditionPlan {
            actor,
            session,
            next,
            applications: Vec::new(),
            receipt: ConditionReceipt {
                occurrence,
                revision: current.revision,
                at: now.get(),
                source: None,
            },
        })
    }

    /// Project the canonical slot's due work; retained provenance remains valid after a
    /// caster dies. Target generation/session/clock/revision are independently current.
    /// Preparing or abandoning the projection never consumes ticks in the live store.
    pub(crate) fn prepare_actor_condition_ticks(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        occurrence: DecisionOccurrenceId,
        facts: TickFacts,
        now: SemanticTimeMicros,
    ) -> Result<ActorConditionTickPlan, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        let current = self.condition_state(index);
        if now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        if current
            .receipt
            .is_some_and(|receipt| receipt.occurrence == occurrence)
        {
            return Err(ConditionOwnerError::StalePlan);
        }
        let mut next = current.store.clone();
        let ticks = next.take_due(now.get(), facts);
        Ok(ActorConditionTickPlan {
            ticks,
            condition: ActorConditionPlan {
                actor,
                session,
                next,
                applications: Vec::new(),
                receipt: ConditionReceipt {
                    occurrence,
                    revision: current.revision,
                    at: now.get(),
                    source: None,
                },
            },
        })
    }

    /// Read/resolve one transition while holding the Channel owner. No slot changes yet.
    pub(crate) fn prepare_actor_condition(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        transition: ActorConditionTransition<'_>,
        now: SemanticTimeMicros,
    ) -> Result<ActorConditionPlan, ConditionOwnerError> {
        let index = self.condition_index(actor, session)?;
        let current = self.condition_state(index);
        let (facts, source) = match &transition {
            ActorConditionTransition::Apply { facts, source, .. }
            | ActorConditionTransition::ApplyBatch { facts, source, .. }
            | ActorConditionTransition::RemoveType { facts, source, .. } => (*facts, Some(*source)),
            ActorConditionTransition::Cleanse(facts) => (*facts, None),
        };
        if facts.target_is_player != session.is_some() || (source.is_none() && session.is_none()) {
            return Err(ConditionOwnerError::FactsMismatch);
        }
        if facts.now != now.get() || now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        if current
            .receipt
            .is_some_and(|receipt| receipt.occurrence == facts.occurrence)
        {
            return Err(ConditionOwnerError::StalePlan);
        }
        if let Some(source) = source {
            self.condition_source(source, actor)?;
        }
        let mut next = current.store.clone();
        next.expire_non_ticking(now.get());
        let mut applications = Vec::new();
        match transition {
            ActorConditionTransition::Apply {
                definition,
                source,
                immunities,
                ..
            } => {
                next.apply(
                    definition,
                    Some(source.actor),
                    source.kind,
                    immunities,
                    &facts,
                )
                .map_err(ConditionOwnerError::Condition)?;
            }
            ActorConditionTransition::ApplyBatch {
                definitions,
                source,
                immunities,
                ..
            } => {
                if definitions.is_empty() || definitions.len() > 16 {
                    return Err(ConditionOwnerError::BatchSize);
                }
                for definition in definitions {
                    let result = next
                        .apply(
                            definition,
                            Some(source.actor),
                            source.kind,
                            immunities,
                            &facts,
                        )
                        .map(|applied| applied.sequence);
                    match result {
                        Ok(_)
                        | Err(
                            ConditionRefusal::Immune
                            | ConditionRefusal::ReentryProtected
                            | ConditionRefusal::KeptCurrent
                            | ConditionRefusal::ZeroDamage,
                        ) => applications.push(result),
                        Err(error) => return Err(ConditionOwnerError::Condition(error)),
                    }
                }
            }
            ActorConditionTransition::RemoveType { kind, .. } => {
                next.remove_type(kind);
            }
            ActorConditionTransition::Cleanse(_) => {
                if let Some(plan) = next
                    .prepare_cleanse(&facts)
                    .map_err(ConditionOwnerError::Condition)?
                {
                    next.commit_cleanse(plan);
                }
            }
        }
        Ok(ActorConditionPlan {
            actor,
            session,
            next,
            applications,
            receipt: ConditionReceipt {
                occurrence: facts.occurrence,
                revision: current.revision,
                at: now.get(),
                source,
            },
        })
    }

    /// The immutable callback cannot alter slot authority. All validation and allocation
    /// precede it; a refusal never consumes the canonical condition projection.
    pub(crate) fn commit_actor_condition_with_vitals<R>(
        &mut self,
        plan: &ActorConditionPlan,
        now: SemanticTimeMicros,
        commit: impl FnOnce(&Self) -> Option<R>,
    ) -> Result<Option<R>, ConditionOwnerError> {
        self.commit_actor_condition_with_life_result(plan, now, |runtime| {
            commit(runtime).map(|receipt| (receipt, false))
        })
    }

    pub(crate) fn commit_actor_condition_with_life_result<R>(
        &mut self,
        plan: &ActorConditionPlan,
        now: SemanticTimeMicros,
        commit_vitals: impl FnOnce(&Self) -> Option<(R, bool)>,
    ) -> Result<Option<R>, ConditionOwnerError> {
        let index = self.condition_index(plan.actor, plan.session)?;
        let current = self.condition_state(index);
        if plan.receipt.at > now.get() || now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        if current.receipt == Some(plan.receipt) && current.store == plan.next {
            return Ok(None);
        }
        if current.revision != plan.receipt.revision || plan.receipt.at != now.get() {
            return Err(ConditionOwnerError::StalePlan);
        }
        if let Some(source) = plan.receipt.source {
            self.condition_source(source, plan.actor)?;
        }
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(ConditionOwnerError::RevisionExhausted)?;
        let mut next = ActorConditionState {
            store: plan.next.clone(),
            revision,
            at: now.get(),
            receipt: Some(plan.receipt),
        };
        let Some((receipt, lethal)) = commit_vitals(self) else {
            return Ok(None);
        };
        if lethal {
            next.store.clear_on_death();
            next.receipt = None;
        }
        // The callback had only &Self. The exact validated occupied slot cannot retire
        // or change generation between preflight and this allocation-free publication.
        match &mut self.carrier.slots[index] {
            Slot::Occupied { lifecycle, .. } => lifecycle.conditions = next,
            Slot::CreatureOccupied { committed, .. } => committed.conditions = next,
            _ => unreachable!("immutable callback preserves validated occupied slot"),
        }
        Ok(Some(receipt))
    }

    /// Couple native player vitals to its canonical condition slot. The callback has no
    /// mutable runtime authority. Checked revision is prepared before any HP/mint write.
    pub(crate) fn commit_player_vitals_with_death_clear<R>(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        commit: impl FnOnce(&Self) -> Option<(R, bool)>,
    ) -> Result<Option<R>, ConditionOwnerError> {
        let index = self.condition_index(actor, Some(session))?;
        let revision = self
            .condition_state(index)
            .revision
            .checked_add(1)
            .ok_or(ConditionOwnerError::RevisionExhausted)?;
        let Some((receipt, lethal)) = commit(self) else {
            return Ok(None);
        };
        if lethal {
            let Slot::Occupied { lifecycle, .. } = &mut self.carrier.slots[index] else {
                unreachable!("immutable callback preserves validated player")
            };
            lifecycle.conditions.store.clear_on_death();
            lifecycle.conditions.revision = revision;
            lifecycle.conditions.receipt = None;
        }
        Ok(Some(receipt))
    }

    /// Existing respawn owner calls this after its successful position preflight/commit.
    /// Exact native player slot/session is the only scope. This cannot affect other actors.
    pub(super) fn clear_respawn_player_conditions(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let Ok(index) = self.condition_index(actor, Some(session)) else {
            return;
        };
        if let Slot::Occupied { lifecycle, .. } = &mut self.carrier.slots[index] {
            lifecycle.conditions.die();
        }
    }

    /// Revalidate independently current scope, slot generation and session at the sole write.
    /// Exactly one receipt is retained; old prepared revisions can never apply again.
    pub(crate) fn commit_actor_condition(
        &mut self,
        plan: &ActorConditionPlan,
        now: SemanticTimeMicros,
    ) -> Result<bool, ConditionOwnerError> {
        let index = self.condition_index(plan.actor, plan.session)?;
        let current = self.condition_state(index);
        if plan.receipt.at > now.get() || now.get() < current.at {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        if current.receipt == Some(plan.receipt) && current.store == plan.next {
            return Ok(false);
        }
        if current.revision != plan.receipt.revision || plan.receipt.at != now.get() {
            return Err(ConditionOwnerError::StalePlan);
        }
        if let Some(source) = plan.receipt.source {
            self.condition_source(source, plan.actor)?;
        }
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(ConditionOwnerError::RevisionExhausted)?;
        let state = match &mut self.carrier.slots[index] {
            Slot::Occupied { lifecycle, .. } => &mut lifecycle.conditions,
            Slot::CreatureOccupied { committed, .. } => &mut committed.conditions,
            _ => unreachable!("validated occupied actor"),
        };
        state.store = plan.next.clone();
        state.revision = revision;
        state.at = now.get();
        state.receipt = Some(plan.receipt);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "runtime_actor_conditions_tests.rs"]
mod tests;

impl ChannelRuntimeV1 {
    // Removal is an existing accepted owner operation, not a fabricated applying caster.
    // Parent source consumers prove source registration, Combat and external fence first.
    fn stage_creature_paralysis_removal(
        &self,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<(usize, ActorConditionState), CarrierError> {
        if !self.contains_live_creature(actor) {
            return Err(CarrierError::StaleActorGeneration);
        }
        let index = self
            .condition_index(actor, None)
            .map_err(|_| CarrierError::StaleActorGeneration)?;
        let current = self.condition_state(index);
        if now < current.at {
            return Err(CarrierError::StaleAttackerSequence);
        }
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(CarrierError::PlanConflict)?;
        let mut store = current.store.clone();
        store.remove_type(ConditionType::Paralysis);
        Ok((
            index,
            ActorConditionState {
                store,
                revision,
                at: now,
                receipt: None,
            },
        ))
    }
    fn publish_creature_condition_removal(&mut self, index: usize, next: ActorConditionState) {
        // Exact native positive-HP heal methods never retire or replace physical slots.
        if let Slot::CreatureOccupied { committed, .. } = &mut self.carrier.slots[index] {
            committed.conditions = next;
        } else {
            unreachable!("native positive HP heal preserves preflighted slot")
        }
    }
    pub(crate) fn commit_source_creature_heal_and_cure(
        &mut self,
        content: [u8; 32],
        heal: &[(ExactActorRef, String, u64, u64)],
        cure: &[ExactActorRef],
        now: u64,
    ) -> Result<Vec<OwnerDamageResult>, CarrierError> {
        if content != self.content_pin().server_artifact_digest() {
            return Err(CarrierError::PositionContextMismatch);
        }
        if cure.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        let mut prepared = Vec::new();
        prepared
            .try_reserve(cure.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (n, actor) in cure.iter().enumerate() {
            if cure[..n].contains(actor) {
                return Err(CarrierError::PlanConflict);
            }
            prepared.push(self.stage_creature_paralysis_removal(*actor, now)?);
        }
        // Native all-target preflight includes physical generation, source-world and Bone
        // shared-HP fanout. A failure publishes neither HP nor any canonical condition slot.
        let receipts = self.commit_source_creature_heal_batch(content, heal)?;
        for (index, next) in prepared {
            self.publish_creature_condition_removal(index, next);
        }
        Ok(receipts)
    }
    pub(crate) fn commit_creature_self_heal_and_cure(
        &mut self,
        ledger: &mut CreatureSelfHealLedger,
        registration: &CreatureSelfHealRegistration,
        actor: ExactActorRef,
        sequence: u64,
        draw: i64,
        now: u64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let (index, next) = self.stage_creature_paralysis_removal(actor, now)?;
        let result = self.commit_creature_self_heal(ledger, registration, actor, sequence, draw)?;
        // Registration owns source highwater; A+B+retryA never reacquires a new cure.
        if result.applied {
            self.publish_creature_condition_removal(index, next)
        }
        Ok(result)
    }
}

impl ChannelRuntimeV1 {
    /// Typed removal after source consumer admission, under existing owner revision/clock.
    /// It grants no applying actor or new immunity and never invokes a DoT tick.
    pub(crate) fn prepare_actor_condition_removal(
        &self,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        kind: ConditionType,
        occurrence: DecisionOccurrenceId,
        now: SemanticTimeMicros,
    ) -> Result<ActorConditionPlan, ConditionOwnerError> {
        let mut plan = self.prepare_actor_condition_expiry(actor, session, occurrence, now)?;
        plan.next.remove_type(kind);
        Ok(plan)
    }
}

impl ChannelRuntimeV1 {
    /// Existing canonical condition owner: preflight every distinct physical slot before
    /// the immutable Player owner callback. No authority/condition side collection.
    pub(crate) fn commit_actor_condition_batch_with_vitals<R>(
        &mut self,
        plans: &[ActorConditionPlan],
        now: SemanticTimeMicros,
        commit: impl FnOnce(&Self) -> Option<R>,
    ) -> Result<Option<R>, ConditionOwnerError> {
        if plans.len() > 64 {
            return Err(ConditionOwnerError::FactsMismatch);
        }
        let mut successors: Vec<(usize, ActorConditionState)> = Vec::new();
        successors
            .try_reserve(plans.len())
            .map_err(|_| ConditionOwnerError::FactsMismatch)?;
        for plan in plans {
            if plan.session.is_some() {
                return Err(ConditionOwnerError::FactsMismatch);
            }
            let index = self.condition_index(plan.actor, None)?;
            if successors.iter().any(|(old, _)| *old == index) {
                return Err(ConditionOwnerError::FactsMismatch);
            }
            let current = self.condition_state(index);
            if now.get() < current.at || plan.receipt.at != now.get() {
                return Err(ConditionOwnerError::TimeMismatch);
            }
            if current.revision != plan.receipt.revision
                || current
                    .receipt
                    .is_some_and(|r| r.occurrence == plan.receipt.occurrence)
            {
                return Err(ConditionOwnerError::StalePlan);
            }
            if let Some(source) = plan.receipt.source {
                self.condition_source(source, plan.actor)?;
            }
            let revision = current
                .revision
                .checked_add(1)
                .ok_or(ConditionOwnerError::RevisionExhausted)?;
            successors.push((
                index,
                ActorConditionState {
                    store: plan.next.clone(),
                    revision,
                    at: now.get(),
                    receipt: Some(plan.receipt),
                },
            ));
        }
        let Some(receipt) = commit(self) else {
            return Ok(None);
        };
        for (index, next) in successors {
            let Slot::CreatureOccupied { committed, .. } = &mut self.carrier.slots[index] else {
                unreachable!("immutable callback preserves preflighted exact Creature slots");
            };
            committed.conditions = next;
        }
        Ok(Some(receipt))
    }
}
