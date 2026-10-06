//! Source soulwars_fear addEvent(2000), inside the existing generic cast owner.
//! PROJECT_BOUNDED_DELAYED_CAST1; no second clock, authority, thread or interpreter.
use super::*;
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane,
    TimerFamily,
};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Family;
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        AI01_PENDING_TIMERS_PER_ACTOR
    }
}
struct Pending {
    token: u64,
    source: SpellSource,
    proposal: ProfileAbilityProposal,
    session: GameSessionId,
}
/// Private one-use native due pulse. Cannot be cloned or fabricated by the generic caller.
pub(super) struct Pulse {
    pending: Pending,
    pub(super) stamp: RuntimeWorkStamp,
}
impl Pulse {
    pub(super) fn into_parts(
        self,
    ) -> (
        SpellSource,
        ProfileAbilityProposal,
        GameSessionId,
        RuntimeWorkStamp,
    ) {
        (
            self.pending.source,
            self.pending.proposal,
            self.pending.session,
            self.stamp,
        )
    }
}
pub(super) struct DelayedOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Family, u64>,
    pending: Vec<Pending>,
    next: u64,
    clock: Option<SemanticTimeMicros>,
}
impl DelayedOwner {
    pub(super) fn new(
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
    ) -> Result<Self, AttackError> {
        let b = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        let generation = b.scope_generation();
        if !fence.is_current_for_scope(scope, generation) {
            return Err(AttackError::StaleOwner);
        }
        let timers = OwnerTimerLane::for_generation(
            scope,
            generation,
            [(
                Family,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::DeadlineState,
                },
            )],
        )
        .map_err(|_| AttackError::InvalidPlan)?;
        Ok(Self {
            scope,
            generation,
            timers,
            pending: Vec::new(),
            next: 1,
            clock: None,
        })
    }
    fn current(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        now: SemanticTimeMicros,
    ) -> Result<(), AttackError> {
        let b = r.binding();
        if self.scope != RuntimeScopeRefV1::channel(b.world_id(), b.channel_id())
            || self.generation != b.scope_generation()
            || !f.is_current_for_scope(self.scope, self.generation)
        {
            return Err(AttackError::StaleOwner);
        }
        if self.clock.is_some_and(|old| now < old) {
            return Err(AttackError::NotDue);
        }
        Ok(())
    }
    // Keep schedule ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn schedule(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &SpellSource,
        p: &ProfileAbilityProposal,
        session: GameSessionId,
        now: SemanticTimeMicros,
        due: SemanticTimeMicros,
    ) -> Result<(), AttackError> {
        self.current(r, f, now)?;
        if !f.accepts_stamp(stamp) || !r.contains_live_creature(p.issuer) {
            return Err(AttackError::StaleOwner);
        }
        if self.pending.len() >= 64 {
            return Err(AttackError::LedgerFull);
        }
        let next = self
            .next
            .checked_add(1)
            .ok_or(AttackError::NumericOverflow)?;
        self.pending
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        let pending = Pending {
            token: self.next,
            source: source.clone(),
            proposal: p.clone(),
            session,
        };
        self.timers
            .schedule(f, stamp, Family, self.next, Some(p.issuer), due)
            .map_err(|e| match e {
                crate::foundation::owner_timer::OwnerTimerError::StaleOwnerGeneration => {
                    AttackError::StaleOwner
                }
                _ => AttackError::LedgerFull,
            })?;
        self.pending.push(pending);
        self.next = next;
        self.clock = Some(now);
        Ok(())
    }
    pub(super) fn drain(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
    ) -> PreparedDueCasts {
        let now = clock.now();
        self.current(r, f, now)?;
        let mut pulses = Vec::new();
        let mut cancellations = Vec::new();
        pulses
            .try_reserve(self.pending.len())
            .map_err(|_| AttackError::LedgerFull)?;
        cancellations
            .try_reserve(self.pending.len())
            .map_err(|_| AttackError::LedgerFull)?;
        // Obtain one genuine batch work ordinal before draining any pending timer.
        // No later ordinal failure can discard earlier committed due-cast outcomes.
        let ordinal = f
            .accept_input(self.generation)
            .map_err(|_| AttackError::StaleOwner)?;
        let stamp = f.stamp(ordinal);
        let mut index = 0;
        while index < self.pending.len() {
            let p = &self.pending[index];
            let error = if !r.contains_live_creature(p.proposal.issuer) {
                Some(AttackError::StaleIssuer)
            } else if r.content_pin().server_artifact_digest() != p.source.content_digest {
                Some(AttackError::ContentChanged)
            } else if r
                .player_control_facts(p.proposal.target, p.session)
                .is_err()
            {
                Some(AttackError::StaleTarget)
            } else {
                None
            };
            if let Some(error) = error {
                self.timers.cancel(Family, p.token);
                let p = self.pending.remove(index);
                cancellations.push((p.source.ability, error));
            } else {
                index += 1;
            }
        }
        let fired = self
            .timers
            .drain_due(clock, f, |a| r.contains_live_creature(a));
        for timer in fired {
            if let Some(index) = self
                .pending
                .iter()
                .position(|p| p.token == timer.occurrence)
            {
                pulses.push(Pulse {
                    pending: self.pending.remove(index),
                    stamp,
                });
            }
        }
        self.clock = Some(now);
        Ok((pulses, cancellations))
    }
    #[cfg(test)]
    pub(super) fn pending_len(&self) -> usize {
        self.timers.pending_len()
    }
}

type PreparedDueCasts = Result<(Vec<Pulse>, Vec<(Ref, AttackError)>), AttackError>;
