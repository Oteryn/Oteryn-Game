//! Source lifecycle events share the actual cast owner lane without wire commands.
use super::*;
use crate::spell::companion_lifecycle::{
    LifecycleFamiliarApplyReceipt, PreparedLifecycleFamiliar, SavedLifecycleFamiliarTimer,
};

#[derive(Debug)]
pub(crate) struct LifecycleTimerReservation {
    expected_revision: u64,
    next_revision: u64,
    stamp: RuntimeWorkStamp,
    lane: OwnerTimerLane<SpellTimerFamily, OwnerSpellTimerKey>,
    payloads: Vec<LifecyclePending>,
    cast_payloads: Vec<Pending>,
    ready: VecDeque<FiredTimer<SpellTimerFamily, OwnerSpellTimerKey>>,
    added: Vec<FamiliarLifecycleOccurrence>,
}
#[derive(Debug)]
pub(crate) struct LifecycleTimerInstallPreflight {
    reservation: LifecycleTimerReservation,
    expected_binding: super::super::companion_lifecycle::FamiliarLifecycleBinding,
}
#[derive(Debug)]
pub(crate) struct ValidatedLifecycleTimerInstall {
    reservation: LifecycleTimerReservation,
}
impl LifecycleTimerInstallPreflight {
    pub(crate) fn finalize(
        self,
        receipt: &LifecycleFamiliarApplyReceipt,
    ) -> Result<ValidatedLifecycleTimerInstall, Error> {
        if receipt.binding() != &self.expected_binding {
            return Err(Error::SubstitutedBatch);
        }
        if !self.reservation.added.iter().all(|key| {
            self.reservation
                .payloads
                .iter()
                .find(|p| p.occurrence == *key)
                .is_some_and(|p| receipt.matches_saved(&p.saved))
        }) {
            return Err(Error::SubstitutedBatch);
        }
        Ok(ValidatedLifecycleTimerInstall {
            reservation: self.reservation,
        })
    }
    pub(crate) fn validate_current(
        &self,
        owner: &SpellTimerOwner,
        fence: &ScopeRuntimeFence,
    ) -> Result<(), Error> {
        owner.check_lifecycle_reservation(fence, &self.reservation)
    }
}
impl SpellTimerOwner {
    pub(crate) fn schedule_lifecycle_reservation(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        requests: Vec<LifecycleScheduleRequest>,
    ) -> Result<LifecycleTimerReservation, Error> {
        if !self.current(fence, stamp) {
            return Err(Error::StaleOwner);
        }
        if requests.len() > MAX_DUE_PER_CYCLE
            || self
                .pending_len()
                .checked_add(requests.len())
                .is_none_or(|n| n > MAX_PENDING_TOTAL)
        {
            return Err(Error::Bounds);
        }
        let next_revision = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        let mut lane = self.lane.clone();
        let mut payloads = self.lifecycle_payloads.clone();
        let mut added = Vec::new();
        payloads
            .try_reserve(requests.len())
            .map_err(|_| Error::Allocation)?;
        added
            .try_reserve(requests.len())
            .map_err(|_| Error::Allocation)?;
        for request in requests {
            let binding = &request.saved.binding;
            if request.phase >= 256
                || request.saved.lifecycle_epoch == 0
                || !fence.accepts_stamp(binding.stamp())
                || request.due < binding.at()
                || RuntimeScopeRefV1::channel(
                    binding.owner().world_id(),
                    binding.owner().channel_id(),
                ) != self.scope()
                || binding.owner().scope_generation() != self.generation()
            {
                return Err(Error::InvalidPayload);
            }
            let occurrence = FamiliarLifecycleOccurrence {
                owner: binding.owner(),
                session: binding.session(),
                epoch: request.saved.lifecycle_epoch,
                // Source event identity stays frozen across durable retries;
                // current `stamp` authorizes this scheduling owner turn only.
                stamp: binding.stamp(),
                phase: request.phase,
            };
            if payloads.iter().any(|p| p.occurrence == occurrence) {
                return Err(Error::InvalidPayload);
            }
            lane.schedule(
                fence,
                stamp,
                SpellTimerFamily::FamiliarLifecycle,
                OwnerSpellTimerKey::Lifecycle(occurrence),
                Some(request.saved.creature),
                request.due,
            )?;
            added.push(occurrence);
            payloads.push(LifecyclePending {
                occurrence,
                due: request.due,
                saved: request.saved,
            });
        }
        Ok(LifecycleTimerReservation {
            expected_revision: self.revision,
            next_revision,
            stamp,
            lane,
            payloads,
            cast_payloads: self.payloads.clone(),
            ready: self.ready.clone(),
            added,
        })
    }
    fn check_lifecycle_reservation(
        &self,
        fence: &ScopeRuntimeFence,
        reservation: &LifecycleTimerReservation,
    ) -> Result<(), Error> {
        if !self.current(fence, reservation.stamp)
            || reservation.lane.scope() != self.scope()
            || reservation.lane.generation() != self.generation()
        {
            return Err(Error::StaleOwner);
        }
        if reservation.expected_revision != self.revision {
            return Err(Error::StaleReservation);
        }
        Ok(())
    }
    pub(crate) fn preflight_lifecycle_install(
        &self,
        fence: &ScopeRuntimeFence,
        mut reservation: LifecycleTimerReservation,
        prepared: &PreparedLifecycleFamiliar,
    ) -> Result<LifecycleTimerInstallPreflight, Error> {
        self.check_lifecycle_reservation(fence, &reservation)?;
        let canonical = prepared.schedules(0).map_err(Error::Companion)?;
        if canonical.len() != reservation.added.len()
            || canonical.iter().any(|schedule| {
                reservation
                    .payloads
                    .iter()
                    .filter(|p| {
                        reservation.added.contains(&p.occurrence)
                            && p.due == schedule.due
                            && p.saved.event == schedule.saved.event
                    })
                    .count()
                    != 1
            })
        {
            return Err(Error::SubstitutedBatch);
        }
        for occurrence in &reservation.added {
            let pending = reservation
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            if !prepared.matches_saved(&pending.saved)
                || !canonical.iter().any(|schedule| {
                    schedule.saved.event == pending.saved.event && schedule.due == pending.due
                })
            {
                return Err(Error::SubstitutedBatch);
            }
        }
        if let super::super::companion_lifecycle::FamiliarLifecycleOrigin::Death { occurrence } =
            prepared.binding().origin()
        {
            let dead = occurrence.actor();
            let epoch = prepared.owner_facts().state.lifecycle_epoch;
            let owner = prepared.binding().owner();
            let mut cancelled = Vec::new();
            reservation.cast_payloads.retain(|p| {
                let remove=matches!(&p.payload,TimerPayload::Familiar(saved) if saved.creature==dead
                    && saved.lifecycle_epoch==epoch && saved.binding.caster==owner);
                if remove { let key=OwnerSpellTimerKey::Cast(p.occurrence);
                    reservation.lane.cancel(SpellTimerFamily::Familiar,key); cancelled.push(key); }
                !remove
            });
            reservation.payloads.retain(|p| {
                let remove = p.saved.creature == dead
                    && p.saved.lifecycle_epoch == epoch
                    && p.saved.binding.owner() == owner;
                if remove {
                    let key = OwnerSpellTimerKey::Lifecycle(p.occurrence);
                    reservation
                        .lane
                        .cancel(SpellTimerFamily::FamiliarLifecycle, key);
                    cancelled.push(key);
                }
                !remove
            });
            reservation
                .ready
                .retain(|p| !cancelled.contains(&p.occurrence));
        }
        Ok(LifecycleTimerInstallPreflight {
            reservation,
            expected_binding: prepared.binding().clone(),
        })
    }
    /// Called in the same uninterrupted owner turn after the genuine sealed
    /// lifecycle physical receipt has finalized the preflight.
    pub(crate) fn install_lifecycle_preflighted(&mut self, proof: ValidatedLifecycleTimerInstall) {
        self.lane = proof.reservation.lane;
        self.lifecycle_payloads = proof.reservation.payloads;
        self.payloads = proof.reservation.cast_payloads;
        self.ready = proof.reservation.ready;
        self.revision = proof.reservation.next_revision;
    }
    pub(crate) fn registered_lifecycle_familiar(
        &self,
        creature: ExactActorRef,
        epoch: u64,
    ) -> Option<&SavedLifecycleFamiliarTimer> {
        self.lifecycle_payloads
            .iter()
            .find(|p| p.saved.creature == creature && p.saved.lifecycle_epoch == epoch)
            .map(|p| &p.saved)
    }
    pub(crate) fn cancel_owner_lifecycle(
        &mut self,
        creature: ExactActorRef,
        epoch: u64,
    ) -> Result<usize, Error> {
        let keys: Vec<_> = self
            .lifecycle_payloads
            .iter()
            .filter(|p| p.saved.creature == creature && p.saved.lifecycle_epoch == epoch)
            .map(|p| p.occurrence)
            .collect();
        if keys.is_empty() {
            return Ok(0);
        }
        let next = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        for key in &keys {
            self.lane.cancel(
                SpellTimerFamily::FamiliarLifecycle,
                OwnerSpellTimerKey::Lifecycle(*key),
            );
        }
        self.lifecycle_payloads
            .retain(|p| !keys.contains(&p.occurrence));
        self.ready.retain(
            |p| !matches!(p.occurrence, OwnerSpellTimerKey::Lifecycle(k) if keys.contains(&k)),
        );
        self.revision = next;
        Ok(keys.len())
    }
}
pub(super) fn lifecycle_target_current(
    runtime: &ChannelRuntimeV1,
    saved: &SavedLifecycleFamiliarTimer,
) -> bool {
    runtime
        .player_control_facts(saved.binding.owner(), saved.binding.session())
        .is_ok()
        && runtime
            .companion_snapshot(saved.creature)
            .is_ok_and(|snapshot| {
                snapshot.state.master
                    == Some(crate::foundation::CompanionMaster {
                        actor: saved.binding.owner(),
                        session: saved.binding.session(),
                    })
                    && snapshot.state.policy.is_familiar
                    && snapshot.state.lifecycle_epoch == saved.lifecycle_epoch
            })
}
