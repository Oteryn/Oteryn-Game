//! COND-1c: one condition store in each occupied actor slot, under the existing owner fence.
//! A retry retains its prepared plan; it never replans an old occurrence at a new revision.

#[cfg(test)]
pub(crate) use super::super::exact_actor_test_ability::condition::{
    ApplicationFacts, ConditionDefinition, ConditionRefusal, ConditionSourceKind, ConditionStore,
    ConditionType, ConditionValues, SpeedRange,
};
use super::*;
#[cfg(not(test))]
pub(crate) use crate::ability::condition::{
    ApplicationFacts, ConditionDefinition, ConditionRefusal, ConditionSourceKind, ConditionStore,
    ConditionType, ConditionValues, SpeedRange,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionOwnerError {
    Actor(CarrierError),
    FactsMismatch,
    TimeMismatch,
    StalePlan,
    RevisionExhausted,
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
            ConditionSourceKind::SelfUse => source.actor == target && source.session.is_some(),
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
            ActorConditionTransition::Apply { facts, source, .. } => (*facts, Some(*source)),
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
            receipt: ConditionReceipt {
                occurrence: facts.occurrence,
                revision: current.revision,
                at: now.get(),
                source,
            },
        })
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
