//! Delayed spell inputs on the existing Channel-owner timer lane.
//!
//! Reservations clone scheduling data, never the live authority fence. Installation
//! occurs only after the physical owner's staged atomic batch has committed under
//! the same exclusive owner turn. Fresh owner facts, lease and session authority
//! still come from the real composition; immutable timer payloads are not authority.

use super::chain::TilePosition;
use super::combat_batch::{CombatBatchReceipt, OwnerCombatBatch};
use super::native::CompiledNativeSpell;
use super::native_delayed::{CasterSnapshot, Element, ScheduledStrike};
use crate::ability::AbilityOccurrence;
pub(crate) use crate::foundation::DeferredCommitAuthority;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, FiredTimer, OwnerClock, OwnerTimerError, OwnerTimerLane,
    SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CommandRef, ExactActorRef, GameSessionId, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use std::collections::{BTreeSet, VecDeque};

/// Explicit local candidate bounds; registration is the owning root's responsibility.
pub(crate) const PENDING_PER_KEY: usize = 32;
pub(crate) const MAX_PENDING_TOTAL: usize = 1024;
pub(crate) const MAX_DUE_PER_CYCLE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellTimerFamily {
    Strike,
    ChainHit,
    MarkerExpiry,
    FieldEvaluate,
    FieldExpiry,
    Familiar,
    NativeOwnerEffect,
    FamiliarLifecycle,
}
impl TimerFamily for SpellTimerFamily {
    fn registered_maximum(self) -> usize {
        PENDING_PER_KEY
    }
}
impl SpellTimerFamily {
    fn catch_up(self) -> CatchUpPolicy {
        match self {
            Self::FieldEvaluate => CatchUpPolicy::SkipToLatest,
            _ => CatchUpPolicy::DeadlineState,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellTimerOccurrence {
    pub(crate) command: CommandRef,
    /// Global phase within this cast, distinct from each target's effect sub-ordinal.
    pub(crate) phase: u16,
}

/// Internal owner input identity. Lifecycle events are not wire commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FamiliarLifecycleOccurrence {
    owner: ExactActorRef,
    session: GameSessionId,
    epoch: u64,
    stamp: RuntimeWorkStamp,
    phase: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnerSpellTimerKey {
    Cast(SpellTimerOccurrence),
    Lifecycle(FamiliarLifecycleOccurrence),
}
#[derive(Debug, Clone)]
struct LifecyclePending {
    occurrence: FamiliarLifecycleOccurrence,
    saved: super::companion_lifecycle::SavedLifecycleFamiliarTimer,
    due: SemanticTimeMicros,
}
#[derive(Debug, Clone)]
pub(crate) struct LifecycleScheduleRequest {
    pub(crate) phase: u16,
    pub(crate) due: SemanticTimeMicros,
    pub(crate) saved: super::companion_lifecycle::SavedLifecycleFamiliarTimer,
}
#[derive(Debug, Clone)]
pub(crate) struct CastBinding {
    pub(crate) spell: CompiledNativeSpell,
    pub(crate) caster: ExactActorRef,
    pub(crate) attacker: CharacterId,
    pub(crate) command: CommandRef,
    pub(crate) occurrence: AbilityOccurrence,
    pub(crate) parent_binding: Vec<u8>,
    pub(crate) cast_at: SemanticTimeMicros,
    pub(crate) cast_position: TilePosition,
    pub(crate) cast_snapshot: Option<CasterSnapshot>,
}
/// Borrowed common metadata; the actual source carrier remains native or ordinary.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CastMetadata<'a> {
    pub(crate) caster: ExactActorRef,
    pub(crate) attacker: CharacterId,
    pub(crate) command: CommandRef,
    pub(crate) occurrence: &'a AbilityOccurrence,
    pub(crate) parent_binding: &'a Vec<u8>,
    pub(crate) cast_at: SemanticTimeMicros,
    pub(crate) cast_position: TilePosition,
    pub(crate) cast_snapshot: Option<&'a CasterSnapshot>,
}
impl CastBinding {
    fn metadata(&self) -> CastMetadata<'_> {
        CastMetadata {
            caster: self.caster,
            attacker: self.attacker,
            command: self.command,
            occurrence: &self.occurrence,
            parent_binding: &self.parent_binding,
            cast_at: self.cast_at,
            cast_position: self.cast_position,
            cast_snapshot: self.cast_snapshot.as_ref(),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct SavedStrike {
    pub(crate) binding: CastBinding,
    pub(crate) scheduled: Box<ScheduledStrike>,
}
#[derive(Debug, Clone)]
pub(crate) struct SavedChainHit {
    pub(crate) binding: CastBinding,
    pub(crate) target: ExactActorRef,
    pub(crate) magnitude: i64,
    pub(crate) element: Element,
    pub(crate) effect_ordinal: u16,
}
#[derive(Debug, Clone)]
pub(crate) struct SavedMarker {
    pub(crate) binding: CastBinding,
    pub(crate) position: TilePosition,
    pub(crate) asset_binding: String,
    /// Identity allocated by the actual presentation/world owner, never a tile/type match.
    pub(crate) instance: [u8; 16],
}
#[derive(Debug, Clone)]
pub(crate) struct SavedField {
    pub(crate) binding: CastBinding,
    pub(crate) expires_at: SemanticTimeMicros,
    /// Exact instances from the Item owner's successful creation receipt.
    pub(crate) instances: Vec<[u8; 16]>,
    pub(crate) positions: Vec<TilePosition>,
}
#[derive(Debug, Clone)]
pub(crate) struct SavedNativeOwnerEffect {
    pub(crate) binding: CastBinding,
    pub(crate) effect: super::combat_batch::OwnerCombatEffect,
}
impl SavedNativeOwnerEffect {
    /// Source delayed callbacks start their condition lifetime on application.
    pub(crate) fn at_due(
        &self,
        now: SemanticTimeMicros,
    ) -> Result<super::combat_batch::OwnerCombatEffect, Error> {
        let mut effect = self.effect.clone();
        let super::combat_batch::OwnerCombatChange::MonsterAi(state) = &mut effect.change else {
            return Err(Error::InvalidPayload);
        };
        let cast_ms = self.binding.cast_at.get() / 1000;
        let due_ms = now.get() / 1000;
        if due_ms < cast_ms {
            return Err(Error::Bounds);
        }
        for expiry in state
            .forced_distance
            .iter_mut()
            .map(|(_, expires)| expires)
            .chain(state.challenged_to.iter_mut().map(|(_, expires)| expires))
        {
            let duration = expiry
                .checked_sub(cast_ms)
                .filter(|d| *d > 0 && *d <= u64::from(u32::MAX))
                .ok_or(Error::InvalidPayload)?;
            *expiry = due_ms.checked_add(duration).ok_or(Error::Bounds)?;
        }
        Ok(effect)
    }
}
#[derive(Debug, Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub(crate) enum TimerPayload {
    Strike(SavedStrike),
    ChainHit(SavedChainHit),
    MarkerExpiry(SavedMarker),
    FieldEvaluate(SavedField),
    FieldExpiry(SavedField),
    Familiar(super::companion_lifecycle::SavedFamiliarTimer),
    NativeOwnerEffect(SavedNativeOwnerEffect),
    OrdinaryCombat(super::ordinary_timer::SavedOrdinaryCombat),
}
impl TimerPayload {
    pub(crate) fn target_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        if let Self::OrdinaryCombat(saved) = self {
            return saved.current_target(runtime);
        }
        if let Self::ChainHit(saved) = self {
            return runtime.contains_live_creature(saved.target)
                && runtime
                    .player_control_facts(
                        saved.binding.caster,
                        saved.binding.command.game_session_id(),
                    )
                    .is_ok();
        }
        if let Self::NativeOwnerEffect(saved) = self {
            return runtime
                .player_control_facts(
                    saved.binding.caster,
                    saved.binding.command.game_session_id(),
                )
                .is_ok()
                && runtime
                    .creature_combat_facts(saved.effect.target, saved.binding.cast_at.get() / 1000)
                    .is_ok_and(|facts| facts.health > 0);
        }
        if let Self::Familiar(saved) = self {
            return runtime
                .player_control_facts(
                    saved.binding.caster,
                    saved.binding.command.game_session_id(),
                )
                .is_ok()
                && runtime
                    .companion_snapshot(saved.creature)
                    .is_ok_and(|snapshot| {
                        snapshot.state.master
                            == Some(crate::foundation::CompanionMaster {
                                actor: saved.binding.caster,
                                session: saved.binding.command.game_session_id(),
                            })
                            && snapshot.state.policy.is_familiar
                            && snapshot.state.lifecycle_epoch == saved.lifecycle_epoch
                    });
        }
        self.target().is_none_or(|target| {
            runtime
                .player_control_facts(target, self.binding().command.game_session_id())
                .is_ok()
        })
    }
    pub(crate) fn binding(&self) -> CastMetadata<'_> {
        match self {
            Self::Strike(v) => v.binding.metadata(),
            Self::ChainHit(v) => v.binding.metadata(),
            Self::MarkerExpiry(v) => v.binding.metadata(),
            Self::FieldEvaluate(v) | Self::FieldExpiry(v) => v.binding.metadata(),
            Self::Familiar(v) => v.binding.metadata(),
            Self::NativeOwnerEffect(v) => v.binding.metadata(),
            Self::OrdinaryCombat(v) => v.binding.metadata(),
        }
    }
    fn family(&self) -> SpellTimerFamily {
        match self {
            Self::Strike(_) => SpellTimerFamily::Strike,
            Self::ChainHit(_) => SpellTimerFamily::ChainHit,
            Self::MarkerExpiry(_) => SpellTimerFamily::MarkerExpiry,
            Self::FieldEvaluate(_) => SpellTimerFamily::FieldEvaluate,
            Self::FieldExpiry(_) => SpellTimerFamily::FieldExpiry,
            Self::Familiar(_) => SpellTimerFamily::Familiar,
            Self::NativeOwnerEffect(_) => SpellTimerFamily::NativeOwnerEffect,
            Self::OrdinaryCombat(_) => SpellTimerFamily::ChainHit,
        }
    }
    /// World/presentation and owned Item mutation need the shared staged compositor.
    /// A physical-only batch must never claim to have performed these effects.
    pub(crate) fn requires_world_owner(&self) -> bool {
        matches!(
            self,
            Self::MarkerExpiry(_) | Self::FieldEvaluate(_) | Self::FieldExpiry(_)
        )
    }
    fn target(&self) -> Option<ExactActorRef> {
        if let Self::Familiar(saved) = self {
            return Some(saved.creature);
        }
        if let Self::NativeOwnerEffect(saved) = self {
            return Some(saved.effect.target);
        }
        if let Self::OrdinaryCombat(saved) = self {
            return Some(saved.target);
        }
        if let Self::ChainHit(saved) = self {
            return Some(saved.target);
        }
        if matches!(self, Self::MarkerExpiry(_) | Self::FieldExpiry(_)) {
            None
        } else {
            Some(self.binding().caster)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScheduleRequest {
    pub(crate) occurrence: SpellTimerOccurrence,
    pub(crate) due: SemanticTimeMicros,
    pub(crate) payload: TimerPayload,
}
#[derive(Debug)]
pub(crate) enum Error {
    Timer(OwnerTimerError),
    Batch(super::combat_batch::Error),
    StaleOwner,
    StaleReservation,
    Bounds,
    InvalidPayload,
    SubstitutedBatch,
    Allocation,
    WorldItemCompositorUnavailable,
    Companion(super::companion_lifecycle::CompanionExecutionError),
}
impl From<OwnerTimerError> for Error {
    fn from(e: OwnerTimerError) -> Self {
        Self::Timer(e)
    }
}

#[derive(Debug, Clone)]
struct Pending {
    occurrence: SpellTimerOccurrence,
    due: SemanticTimeMicros,
    payload: TimerPayload,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeAiTimerSource {
    pub(crate) caster: ExactActorRef,
    pub(crate) attacker: CharacterId,
    pub(crate) command: CommandRef,
    pub(crate) spell: CompiledNativeSpell,
}
#[derive(Debug)]
pub(crate) struct TimerReservation {
    expected_revision: u64,
    next_revision: u64,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    stamp: RuntimeWorkStamp,
    lane: OwnerTimerLane<SpellTimerFamily, OwnerSpellTimerKey>,
    payloads: Vec<Pending>,
    lifecycle_payloads: Vec<LifecyclePending>,
    ready: VecDeque<FiredTimer<SpellTimerFamily, OwnerSpellTimerKey>>,
    added: Vec<SpellTimerOccurrence>,
}
#[derive(Debug)]
pub(crate) struct FamiliarTimerInstallPreflight {
    reservation: TimerReservation,
}
#[derive(Debug)]
pub(crate) struct ValidatedFamiliarTimerInstall {
    reservation: TimerReservation,
}
impl FamiliarTimerInstallPreflight {
    /// The same successful physical owner must mint this sealed receipt.
    pub(crate) fn finalize(
        self,
        receipt: &super::companion_lifecycle::FamiliarApplyReceipt,
    ) -> Result<ValidatedFamiliarTimerInstall, Error> {
        for occurrence in &self.reservation.added {
            let pending = self
                .reservation
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            let TimerPayload::Familiar(saved) = &pending.payload else {
                return Err(Error::InvalidPayload);
            };
            if !receipt.matches_saved(saved) {
                return Err(Error::SubstitutedBatch);
            }
        }
        Ok(ValidatedFamiliarTimerInstall {
            reservation: self.reservation,
        })
    }
}
/// A reservation qualified before the physical batch commits. The caller must
/// retain the same exclusive timer owner turn until installation; this proof
/// contains data only, never a cloned scope authority.
#[derive(Debug)]
pub(crate) struct ValidatedTimerInstall {
    reservation: TimerReservation,
}
impl ValidatedTimerInstall {
    pub(crate) fn validate_current(
        &self,
        owner: &SpellTimerOwner,
        fence: &ScopeRuntimeFence,
        batch: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        owner.check_reservation(fence, self.reservation.stamp, &self.reservation, batch)
    }

    /// Rebase a retained original cast before its next potentially committing
    /// SQL attempt. Unrelated current timers/ready inputs are retained; source
    /// payloads, occurrences and original deadlines are never reconstructed.
    pub(crate) fn refresh_current(
        &mut self,
        owner: &SpellTimerOwner,
        fence: &ScopeRuntimeFence,
        batch: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        if self.reservation.expected_revision == owner.revision {
            return self.validate_current(owner, fence, batch);
        }
        owner.check_reservation_binding(fence, self.reservation.stamp, &self.reservation, batch)?;
        let mut requests = Vec::new();
        requests
            .try_reserve(self.reservation.added.len())
            .map_err(|_| Error::Allocation)?;
        for occurrence in &self.reservation.added {
            let original = self
                .reservation
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            requests.push(ScheduleRequest {
                occurrence: *occurrence,
                due: original.due,
                payload: original.payload.clone(),
            });
        }
        let reservation = owner.schedule_reservation(fence, self.reservation.stamp, requests)?;
        owner.check_reservation(fence, reservation.stamp, &reservation, batch)?;
        // Nothing changes in this retained proof unless the complete new
        // reservation and current owner/batch qualification succeeded.
        self.reservation = reservation;
        Ok(())
    }
}
/// The actual owner lane plus immutable payloads; `ready` is its already normalized
/// fired-input queue, never an independent timer scheduler.
#[derive(Debug)]
pub(crate) struct SpellTimerOwner {
    lane: OwnerTimerLane<SpellTimerFamily, OwnerSpellTimerKey>,
    payloads: Vec<Pending>,
    lifecycle_payloads: Vec<LifecyclePending>,
    ready: VecDeque<FiredTimer<SpellTimerFamily, OwnerSpellTimerKey>>,
    // Failed owner commit retains the exact typed effects and sealed token. A
    // retry must never reroll damage or manufacture a newer admission sequence.
    prepared_due: Option<(SpellTimerOccurrence, OwnerCombatBatch)>,
    revision: u64,
}
#[derive(Debug)]
pub(crate) struct FireReceipt {
    pub(crate) occurrence: SpellTimerOccurrence,
    pub(crate) batch: CombatBatchReceipt,
}
#[derive(Debug, Default)]
pub(crate) struct FireReport {
    pub(crate) receipts: Vec<FireReceipt>,
    pub(crate) dropped: Vec<SpellTimerOccurrence>,
    pub(crate) lifecycle_dropped: Vec<FamiliarLifecycleOccurrence>,
    pub(crate) lifecycle_blocked: Option<(FamiliarLifecycleOccurrence, Error)>,
    pub(crate) familiar_receipts: Vec<super::companion_lifecycle::FamiliarTimerReceipt>,
    pub(crate) blocked: Option<(SpellTimerOccurrence, Error)>,
}

impl SpellTimerOwner {
    /// Actual lane metadata only; reading a deadline grants no callback authority.
    pub(crate) fn next_due_time(&self) -> Option<SemanticTimeMicros> {
        self.payloads
            .iter()
            .map(|p| p.due)
            .chain(self.lifecycle_payloads.iter().map(|p| p.due))
            .min()
    }
    /// Composition rechecks its fresh SQL/source/lease facts for a retained due
    /// batch before draining. It never evaluates another effect or consumes RNG.
    pub(crate) fn validate_prepared_due(
        &self,
        mut qualify: impl FnMut(&TimerPayload, &OwnerCombatBatch) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let Some((occurrence, batch)) = &self.prepared_due else {
            return Ok(());
        };
        let payload = self
            .payloads
            .iter()
            .find(|p| p.occurrence == *occurrence)
            .ok_or(Error::StaleOwner)?;
        qualify(&payload.payload, batch)
    }
    /// Immutable due-source proposals, bounded by the actual owner callback
    /// budget. A fresh DB/session/physical owner must qualify every proposal.
    pub(crate) fn due_native_ai_sources(
        &self,
        now: SemanticTimeMicros,
    ) -> Vec<NativeAiTimerSource> {
        let mut pending: Vec<_> = self
            .payloads
            .iter()
            .filter(|p| p.due <= now && matches!(p.payload, TimerPayload::NativeOwnerEffect(_)))
            .collect();
        pending.sort_by_key(|p| p.due);
        pending
            .into_iter()
            .take(MAX_DUE_PER_CYCLE)
            .filter_map(|p| {
                let TimerPayload::NativeOwnerEffect(saved) = &p.payload else {
                    return None;
                };
                let binding = &saved.binding;
                Some(NativeAiTimerSource {
                    caster: binding.caster,
                    attacker: binding.attacker,
                    command: binding.command,
                    spell: binding.spell.clone(),
                })
            })
            .collect()
    }
    pub(crate) fn due_ordinary_payloads(
        &self,
        now: SemanticTimeMicros,
    ) -> Vec<super::ordinary_timer::SavedOrdinaryCombat> {
        let mut pending: Vec<_> = self
            .payloads
            .iter()
            .filter(|p| p.due <= now && matches!(p.payload, TimerPayload::OrdinaryCombat(_)))
            .collect();
        pending.sort_by_key(|p| p.due);
        pending
            .into_iter()
            .take(MAX_DUE_PER_CYCLE)
            .filter_map(|p| {
                let TimerPayload::OrdinaryCombat(saved) = &p.payload else {
                    return None;
                };
                Some(saved.clone())
            })
            .collect()
    }
    /// A current lane entry, rather than a caller's cloned source data, qualifies
    /// the sealed due-only SQL reader. This method never constructs SQL authority.
    pub(crate) fn contains_due_ordinary_source(
        &self,
        runtime: &ChannelRuntimeV1,
        saved: &super::ordinary_timer::SavedOrdinaryCombat,
        now: SemanticTimeMicros,
    ) -> bool {
        self.scope()
            == RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
            && self.generation() == runtime.binding().scope_generation()
            && runtime.owner_fence().is_ok()
            && self.payloads.iter().any(|pending| {
                let TimerPayload::OrdinaryCombat(current) = &pending.payload else {
                    return false;
                };
                pending.due <= now
                    && pending.occurrence.command == saved.binding.command
                    && pending.occurrence.phase == saved.phase
                    && current.binding == saved.binding
                    && current.target == saved.target
                    && current.step == saved.step
                    && current.from == saved.from
                    && current.sub_ordinal == saved.sub_ordinal
                    && current.delay_ms == saved.delay_ms
                    && saved.validate(pending.occurrence, pending.due).is_ok()
            })
    }
    /// Data-only retention under the held actual Channel owner turn, before a
    /// potentially committing await. A saved value never issues current authority.
    pub(crate) fn retain_ordinary_numeric(
        &mut self,
        runtime: &ChannelRuntimeV1,
        occurrence: SpellTimerOccurrence,
        resolved: super::ordinary_timer::ResolvedOrdinaryCombat,
    ) -> Result<(), Error> {
        if self.scope()
            != RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
            || self.generation() != runtime.binding().scope_generation()
            || runtime.owner_fence().is_err()
        {
            return Err(Error::StaleOwner);
        }
        let next = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        let pending = self
            .payloads
            .iter_mut()
            .find(|p| p.occurrence == occurrence)
            .ok_or(Error::StaleOwner)?;
        let TimerPayload::OrdinaryCombat(saved) = &mut pending.payload else {
            return Err(Error::InvalidPayload);
        };
        if !saved.current_target(runtime)
            || resolved.batch.caster != saved.binding.caster
            || resolved.batch.attacker != saved.binding.attacker
            || resolved.batch.command != saved.binding.command
            || resolved.batch.occurrence != saved.binding.occurrence.clone().into()
            || resolved.batch.current_lease_generation == 0
            || resolved.batch.anchor.is_some()
            || resolved.batch.deferred.is_some()
            || resolved.batch.effects.len() != 1
            || !saved.validates_effect(&resolved.batch.effects[0])
        {
            return Err(Error::InvalidPayload);
        }
        if let Some(retained) = &saved.resolved {
            return if retained.batch == resolved.batch
                && std::sync::Arc::ptr_eq(&retained.presentation, &resolved.presentation)
            {
                Ok(())
            } else {
                Err(Error::SubstitutedBatch)
            };
        }
        saved.resolved = Some(resolved);
        self.revision = next;
        Ok(())
    }
    pub(crate) fn prepared_due_occurrence(&self) -> Option<SpellTimerOccurrence> {
        self.prepared_due
            .as_ref()
            .map(|(occurrence, _)| *occurrence)
    }
    pub(crate) fn registered_familiar(
        &self,
        creature: ExactActorRef,
        epoch: u64,
    ) -> Option<&super::companion_lifecycle::SavedFamiliarTimer> {
        self.payloads
            .iter()
            .find_map(|pending| match &pending.payload {
                TimerPayload::Familiar(saved)
                    if saved.creature == creature && saved.lifecycle_epoch == epoch =>
                {
                    Some(saved)
                }
                _ => None,
            })
    }
    pub(crate) fn scope(&self) -> RuntimeScopeRefV1 {
        self.lane.scope()
    }
    pub(crate) fn generation(&self) -> ScopeOwnershipGeneration {
        self.lane.generation()
    }
    /// Death/cancellation consumes only this exact lifecycle's saved events.
    pub(crate) fn cancel_lifecycle(
        &mut self,
        command: CommandRef,
        creature: ExactActorRef,
        epoch: u64,
    ) -> Result<usize, Error> {
        let occurrences:Vec<_> = self.payloads.iter().filter_map(|pending|
            matches!(&pending.payload,TimerPayload::Familiar(saved)
                if pending.occurrence.command == command && saved.creature == creature && saved.lifecycle_epoch == epoch)
                .then_some(pending.occurrence)).collect();
        if occurrences.is_empty() {
            return Ok(0);
        }
        let next_revision = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        for occurrence in &occurrences {
            self.lane.cancel(
                SpellTimerFamily::Familiar,
                OwnerSpellTimerKey::Cast(*occurrence),
            );
        }
        self.payloads
            .retain(|p| !occurrences.contains(&p.occurrence));
        self.ready.retain(
            |p| !matches!(p.occurrence, OwnerSpellTimerKey::Cast(o) if occurrences.contains(&o)),
        );
        if self
            .prepared_due
            .as_ref()
            .is_some_and(|(o, _)| occurrences.contains(o))
        {
            self.prepared_due = None;
        }
        self.revision = next_revision;
        Ok(occurrences.len())
    }
    pub(crate) fn new(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Self, Error> {
        let families = [
            SpellTimerFamily::Strike,
            SpellTimerFamily::ChainHit,
            SpellTimerFamily::MarkerExpiry,
            SpellTimerFamily::FieldEvaluate,
            SpellTimerFamily::FieldExpiry,
            SpellTimerFamily::Familiar,
            SpellTimerFamily::NativeOwnerEffect,
            SpellTimerFamily::FamiliarLifecycle,
        ];
        let lane = OwnerTimerLane::for_generation(
            scope,
            generation,
            families
                .into_iter()
                .map(|f| {
                    (
                        f,
                        FamilyPolicy {
                            max_pending: PENDING_PER_KEY,
                            catch_up: f.catch_up(),
                        },
                    )
                })
                .collect::<Vec<_>>(),
        )?;
        Ok(Self {
            lane,
            payloads: vec![],
            lifecycle_payloads: vec![],
            ready: VecDeque::new(),
            prepared_due: None,
            revision: 0,
        })
    }
    fn current(&self, fence: &ScopeRuntimeFence, stamp: RuntimeWorkStamp) -> bool {
        fence.is_current_for_scope(self.lane.scope(), self.lane.generation())
            && fence.accepts_stamp(stamp)
    }
    pub(crate) fn pending_len(&self) -> usize {
        self.payloads.len() + self.lifecycle_payloads.len()
    }
    /// All fallible scheduling occurs on a clone of actual lane data before cost,
    /// damage, Item or presentation state can commit. The authority fence is borrowed.
    pub(crate) fn schedule_reservation(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        requests: Vec<ScheduleRequest>,
    ) -> Result<TimerReservation, Error> {
        if !self.current(fence, stamp) {
            return Err(Error::StaleOwner);
        }
        if requests.len() > MAX_DUE_PER_CYCLE
            || self
                .payloads
                .len()
                .checked_add(self.lifecycle_payloads.len())
                .and_then(|n| n.checked_add(requests.len()))
                .is_none_or(|n| n > MAX_PENDING_TOTAL)
        {
            return Err(Error::Bounds);
        }
        let next_revision = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        let mut lane = self.lane.clone();
        let mut payloads = self.payloads.clone();
        payloads
            .try_reserve(requests.len())
            .map_err(|_| Error::Allocation)?;
        let mut added = Vec::new();
        added
            .try_reserve(requests.len())
            .map_err(|_| Error::Allocation)?;
        for request in requests {
            validate_request(&request)?;
            if payloads.iter().any(|p| p.occurrence == request.occurrence) {
                return Err(Error::InvalidPayload);
            }
            lane.schedule(
                fence,
                stamp,
                request.payload.family(),
                OwnerSpellTimerKey::Cast(request.occurrence),
                request.payload.target(),
                request.due,
            )?;
            added.push(request.occurrence);
            payloads.push(Pending {
                occurrence: request.occurrence,
                due: request.due,
                payload: request.payload,
            });
        }
        Ok(TimerReservation {
            expected_revision: self.revision,
            next_revision,
            scope: lane.scope(),
            generation: lane.generation(),
            stamp,
            lane,
            payloads,
            lifecycle_payloads: self.lifecycle_payloads.clone(),
            ready: self.ready.clone(),
            added,
        })
    }
    fn check_reservation(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        r: &TimerReservation,
        b: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        self.check_reservation_binding(fence, stamp, r, b)?;
        if r.expected_revision != self.revision {
            return Err(Error::StaleReservation);
        }
        Ok(())
    }
    fn check_reservation_binding(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        r: &TimerReservation,
        b: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        if !self.current(fence, stamp)
            || r.scope != self.lane.scope()
            || r.generation != self.lane.generation()
            || r.stamp != stamp
        {
            return Err(Error::StaleOwner);
        }
        for occurrence in &r.added {
            let pending = r
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            let binding = pending.payload.binding();
            if occurrence.command != b.command
                || binding.command != b.command
                || binding.caster != b.caster
                || binding.attacker != b.attacker
                || crate::foundation::runtime_actor_spell_types::SpellOccurrenceBinding::from(
                    (*binding.occurrence).clone(),
                ) != b.occurrence
                || binding.parent_binding != &b.binding
            {
                return Err(Error::SubstitutedBatch);
            }
            if pending.payload.requires_world_owner() {
                return Err(Error::WorldItemCompositorUnavailable);
            }
            if matches!(pending.payload, TimerPayload::Familiar(_)) {
                return Err(Error::InvalidPayload);
            }
        }
        Ok(())
    }
    fn install(&mut self, r: TimerReservation) {
        self.lane = r.lane;
        self.payloads = r.payloads;
        self.lifecycle_payloads = r.lifecycle_payloads;
        self.ready = r.ready;
        self.revision = r.next_revision;
    }
    pub(crate) fn preflight_install(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        batch: &OwnerCombatBatch,
        reservation: TimerReservation,
    ) -> Result<ValidatedTimerInstall, Error> {
        self.check_reservation(fence, stamp, &reservation, batch)?;
        Ok(ValidatedTimerInstall { reservation })
    }
    /// Infallible replacement after an applied combined owner receipt. Keep the
    /// exclusive owner turn from preflight through this call.
    pub(crate) fn install_preflighted(&mut self, proof: ValidatedTimerInstall) {
        self.install(proof.reservation);
    }
    /// Required before the durable familiar writer or physical spawn. This
    /// reservation can contain only familiar events, on this unchanged lane.
    pub(crate) fn validate_familiar_reservation(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        r: &TimerReservation,
    ) -> Result<(), Error> {
        if !self.current(fence, stamp)
            || r.scope != self.scope()
            || r.generation != self.generation()
            || r.stamp != stamp
        {
            return Err(Error::StaleOwner);
        }
        if r.expected_revision != self.revision {
            return Err(Error::StaleReservation);
        }
        for occurrence in &r.added {
            let payload = r
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            if !matches!(payload.payload, TimerPayload::Familiar(_)) {
                return Err(Error::InvalidPayload);
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_familiar_install(
        &self,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        reservation: TimerReservation,
        prepared: &super::companion_lifecycle::PreparedFamiliar,
    ) -> Result<FamiliarTimerInstallPreflight, Error> {
        self.validate_familiar_reservation(fence, stamp, &reservation)?;
        let canonical = prepared
            .schedules(prepared.binding().cast_at, 0)
            .map_err(Error::Companion)?;
        if canonical.len()!=reservation.added.len() || canonical.iter().any(|schedule| {
            reservation.payloads.iter().filter(|p|reservation.added.contains(&p.occurrence)
                && p.due==schedule.due
                && matches!(&p.payload,TimerPayload::Familiar(saved) if saved.event==schedule.saved.event)).count()!=1
        }) {return Err(Error::SubstitutedBatch);}
        for occurrence in &reservation.added {
            let pending = reservation
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            let TimerPayload::Familiar(saved) = &pending.payload else {
                return Err(Error::InvalidPayload);
            };
            if !prepared.matches_saved(saved) {
                return Err(Error::SubstitutedBatch);
            }
        }
        Ok(FamiliarTimerInstallPreflight { reservation })
    }
    /// Caller has installed the same prepared familiar and paid player successor
    /// successfully while retaining this exact uninterrupted owner turn. Every
    /// timer check/allocation preceded those writes; this replacement cannot fail.
    pub(crate) fn install_familiar_preflighted(&mut self, proof: ValidatedFamiliarTimerInstall) {
        self.install(proof.reservation);
    }
    /// The proof is minted only by the actual successful familiar installer.
    /// The caller retains the exclusive owner turn across both installations.
    pub(crate) fn install_familiar_reservation(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        r: TimerReservation,
        proof: &super::companion_lifecycle::FamiliarApplyReceipt,
    ) -> Result<(), Error> {
        self.validate_familiar_reservation(fence, stamp, &r)?;
        for occurrence in &r.added {
            let pending = r
                .payloads
                .iter()
                .find(|p| p.occurrence == *occurrence)
                .ok_or(Error::InvalidPayload)?;
            let TimerPayload::Familiar(saved) = &pending.payload else {
                return Err(Error::InvalidPayload);
            };
            if !proof.matches_saved(saved) || !pending.payload.target_current(runtime) {
                return Err(Error::SubstitutedBatch);
            }
        }
        self.install(r);
        Ok(())
    }
    /// Caller holds the actual owner lock across this entire call and the actual
    /// player-vitals owner's independent anchor preflight and infallible replacement.
    pub(crate) fn commit_cast(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        batch: &OwnerCombatBatch,
        reservation: TimerReservation,
    ) -> Result<CombatBatchReceipt, Error> {
        self.check_reservation(fence, stamp, &reservation, batch)?;
        let staged = runtime.stage_spell_batch(batch).map_err(Error::Batch)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(Error::Batch)?;
        // A reconciled command replay cannot re-install already fired timers.
        // Every allocation, authority/version check and cap check preceded mutation.
        if receipt.applied {
            self.install(reservation);
        }
        Ok(receipt)
    }
    /// Due callbacks build effects from independently fresh actual owner facts.
    /// This helper commits those effects through the real physical atomic batch.
    /// A failed event retains its normalized input; earlier committed events remain.
    pub(crate) fn fire_due(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        clock: &impl OwnerClock,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        build: impl FnMut(
            &ChannelRuntimeV1,
            &TimerPayload,
            SpellTimerOccurrence,
            SemanticTimeMicros,
            RuntimeWorkStamp,
        ) -> Result<OwnerCombatBatch, Error>,
    ) -> Result<FireReport, Error> {
        self.fire_due_impl(runtime, clock, Some(fence), stamp, build)
    }
    /// Production route borrows the actual runtime fence only for each check or
    /// mint, releases that borrow, then commits through the same real runtime.
    pub(crate) fn fire_due_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        clock: &impl OwnerClock,
        stamp: RuntimeWorkStamp,
        build: impl FnMut(
            &ChannelRuntimeV1,
            &TimerPayload,
            SpellTimerOccurrence,
            SemanticTimeMicros,
            RuntimeWorkStamp,
        ) -> Result<OwnerCombatBatch, Error>,
    ) -> Result<FireReport, Error> {
        self.fire_due_impl(runtime, clock, None, stamp, build)
    }
    fn fire_due_impl(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        clock: &impl OwnerClock,
        fence: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        mut build: impl FnMut(
            &ChannelRuntimeV1,
            &TimerPayload,
            SpellTimerOccurrence,
            SemanticTimeMicros,
            RuntimeWorkStamp,
        ) -> Result<OwnerCombatBatch, Error>,
    ) -> Result<FireReport, Error> {
        if !self.current(current_timer_fence(runtime, fence)?, stamp) {
            return Err(Error::StaleOwner);
        }
        let sequence_base = stamp
            .ordinal()
            .get()
            .checked_mul((MAX_DUE_PER_CYCLE as u64) + 1)
            .ok_or(Error::Bounds)?;
        let next_revision = self.revision.checked_add(1).ok_or(Error::Bounds)?;
        let mut lane = self.lane.clone();
        let mut ready = self.ready.clone();
        let mut payloads = self.payloads.clone();
        let mut report = FireReport::default();
        for pending in &self.payloads {
            if pending.payload.target().is_some() {
                let controlled = pending.payload.target_current(runtime);
                if !controlled {
                    lane.cancel(
                        pending.payload.family(),
                        OwnerSpellTimerKey::Cast(pending.occurrence),
                    );
                    ready.retain(|r| r.occurrence != OwnerSpellTimerKey::Cast(pending.occurrence));
                    payloads.retain(|p| p.occurrence != pending.occurrence);
                    report.dropped.push(pending.occurrence);
                }
            }
        }
        let mut lifecycle_payloads = self.lifecycle_payloads.clone();
        for pending in &self.lifecycle_payloads {
            if !lifecycle_target_current(runtime, &pending.saved) {
                lane.cancel(
                    SpellTimerFamily::FamiliarLifecycle,
                    OwnerSpellTimerKey::Lifecycle(pending.occurrence),
                );
                ready.retain(|r| r.occurrence != OwnerSpellTimerKey::Lifecycle(pending.occurrence));
                lifecycle_payloads.retain(|p| p.occurrence != pending.occurrence);
                report.lifecycle_dropped.push(pending.occurrence);
            }
        }
        let due = lane.drain_due(clock, current_timer_fence(runtime, fence)?, |target| {
            self.payloads
                .iter()
                .any(|p| p.payload.target() == Some(target) && p.payload.target_current(runtime))
                || self.lifecycle_payloads.iter().any(|p| {
                    p.saved.creature == target && lifecycle_target_current(runtime, &p.saved)
                })
        });
        ready
            .try_reserve(due.len())
            .map_err(|_| Error::Allocation)?;
        ready.extend(due);
        self.lane = lane;
        self.ready = ready;
        self.payloads = payloads;
        self.lifecycle_payloads = lifecycle_payloads;
        if self
            .prepared_due
            .as_ref()
            .is_some_and(|(o, _)| !self.payloads.iter().any(|p| p.occurrence == *o))
        {
            self.prepared_due = None;
        }
        self.revision = next_revision;
        report
            .receipts
            .try_reserve(MAX_DUE_PER_CYCLE)
            .map_err(|_| Error::Allocation)?;
        report
            .familiar_receipts
            .try_reserve(MAX_DUE_PER_CYCLE)
            .map_err(|_| Error::Allocation)?;
        for event_index in 0..MAX_DUE_PER_CYCLE {
            let Some(fired) = self.ready.front().copied() else {
                break;
            };
            let occurrence = match fired.occurrence {
                OwnerSpellTimerKey::Cast(occurrence) => occurrence,
                OwnerSpellTimerKey::Lifecycle(occurrence) => {
                    let Some(pending) = self
                        .lifecycle_payloads
                        .iter()
                        .find(|p| p.occurrence == occurrence)
                    else {
                        self.ready.pop_front();
                        report.lifecycle_dropped.push(occurrence);
                        continue;
                    };
                    let staged = match super::companion_lifecycle::stage_lifecycle_familiar_timer(
                        runtime,
                        &pending.saved,
                    ) {
                        Ok(Some(staged)) => staged,
                        Ok(None) => {
                            self.ready.pop_front();
                            self.lifecycle_payloads
                                .retain(|p| p.occurrence != occurrence);
                            report.lifecycle_dropped.push(occurrence);
                            continue;
                        }
                        Err(error) => {
                            report.lifecycle_blocked = Some((occurrence, Error::Companion(error)));
                            break;
                        }
                    };
                    match super::companion_lifecycle::commit_familiar_timer(runtime, staged) {
                        Ok(receipt) => {
                            self.ready.pop_front();
                            self.lifecycle_payloads
                                .retain(|p| p.occurrence != occurrence);
                            report.familiar_receipts.push(receipt);
                            continue;
                        }
                        Err(error) => {
                            report.lifecycle_blocked = Some((occurrence, Error::Companion(error)));
                            break;
                        }
                    }
                }
            };
            let Some(pending) = self.payloads.iter().find(|p| p.occurrence == occurrence) else {
                self.ready.pop_front();
                report.dropped.push(occurrence);
                continue;
            };
            if pending.payload.requires_world_owner() {
                report.blocked = Some((occurrence, Error::WorldItemCompositorUnavailable));
                break;
            }
            if let TimerPayload::Familiar(saved) = &pending.payload {
                let staged = match super::companion_lifecycle::stage_familiar_timer(runtime, saved)
                {
                    Ok(Some(staged)) => staged,
                    Ok(None) => {
                        self.ready.pop_front();
                        self.payloads.retain(|p| p.occurrence != occurrence);
                        report.dropped.push(occurrence);
                        continue;
                    }
                    Err(error) => {
                        report.blocked = Some((occurrence, Error::Companion(error)));
                        break;
                    }
                };
                match super::companion_lifecycle::commit_familiar_timer(runtime, staged) {
                    Ok(receipt) => {
                        self.ready.pop_front();
                        self.payloads.retain(|p| p.occurrence != occurrence);
                        report.familiar_receipts.push(receipt);
                        continue;
                    }
                    Err(error) => {
                        report.blocked = Some((occurrence, Error::Companion(error)));
                        break;
                    }
                }
            }
            let batch = if let Some((prepared_occurrence, batch)) = &self.prepared_due {
                if *prepared_occurrence != occurrence {
                    report.blocked = Some((occurrence, Error::InvalidPayload));
                    break;
                }
                batch.clone()
            } else {
                let mut batch =
                    match build(runtime, &pending.payload, occurrence, clock.now(), stamp) {
                        Ok(batch) => batch,
                        Err(e) => {
                            report.blocked = Some((occurrence, e));
                            break;
                        }
                    };
                let binding = pending.payload.binding();
                if batch.caster != binding.caster
                    || batch.attacker != binding.attacker
                    || batch.command != binding.command
                    || batch.occurrence != (*binding.occurrence).clone().into()
                    || batch.anchor.is_some()
                    || batch.current_lease_generation == 0
                    || match &pending.payload {
                        TimerPayload::OrdinaryCombat(saved) => {
                            saved.resolved.as_ref().is_none_or(|resolved| {
                                resolved.batch != batch
                                    || batch.now_ms > clock.now().get() / 1000
                                    || batch.now_ms < pending.due.get() / 1000
                            })
                        }
                        _ => batch.now_ms != clock.now().get() / 1000,
                    }
                    || batch.effects.len() > super::combat_batch::MAX_EFFECTS
                    || batch.deferred.is_some()
                {
                    report.blocked = Some((occurrence, Error::SubstitutedBatch));
                    break;
                }
                let admission_sequence = sequence_base
                    .checked_add(event_index as u64)
                    .ok_or(Error::Bounds)?;
                let token = match DeferredCommitAuthority::from_due(
                    current_timer_fence(runtime, fence)?,
                    self.lane.scope(),
                    self.lane.generation(),
                    stamp,
                    occurrence.phase.checked_add(1).ok_or(Error::Bounds)?,
                    admission_sequence,
                    binding.parent_binding.clone(),
                    &batch,
                ) {
                    Ok(token) => token,
                    Err(error) => {
                        report.blocked = Some((occurrence, Error::Batch(error)));
                        break;
                    }
                };
                batch.deferred = Some(token);
                self.prepared_due = Some((occurrence, batch.clone()));
                batch
            };
            let receipt = match runtime
                .stage_spell_batch(&batch)
                .and_then(|s| runtime.commit_spell_batch(s))
            {
                Ok(receipt) => receipt,
                Err(e) => {
                    report.blocked = Some((occurrence, Error::Batch(e)));
                    break;
                }
            };
            self.ready.pop_front();
            self.prepared_due = None;
            self.payloads.retain(|p| p.occurrence != occurrence);
            report.receipts.push(FireReceipt {
                occurrence,
                batch: receipt,
            });
        }
        Ok(report)
    }
}

fn current_timer_fence<'a>(
    runtime: &'a ChannelRuntimeV1,
    provided: Option<&'a ScopeRuntimeFence>,
) -> Result<&'a ScopeRuntimeFence, Error> {
    provided.map_or_else(|| runtime.owner_fence().map_err(|_| Error::StaleOwner), Ok)
}

fn validate_request(r: &ScheduleRequest) -> Result<(), Error> {
    let binding = r.payload.binding();
    if r.occurrence.command != binding.command
        || r.occurrence.phase >= 256
        || r.due < binding.cast_at
        || binding.parent_binding.len() > super::combat_batch::MAX_BINDING_BYTES
        || binding.parent_binding.is_empty()
    {
        return Err(Error::InvalidPayload);
    }
    match &r.payload {
        TimerPayload::Strike(v) => {
            let due = v.scheduled.due_ms.checked_mul(1000).ok_or(Error::Bounds)?;
            if due != r.due.get()
                || v.scheduled.position != binding.cast_position
                || binding.cast_snapshot.as_ref().map(|s| &s.identity)
                    != Some(&v.scheduled.expected_caster)
            {
                return Err(Error::InvalidPayload);
            }
        }
        TimerPayload::OrdinaryCombat(saved) => {
            saved.validate(r.occurrence, r.due)?;
        }
        TimerPayload::ChainHit(v) => {
            if v.magnitude <= 0 || v.effect_ordinal >= 256 {
                return Err(Error::InvalidPayload);
            }
        }
        TimerPayload::MarkerExpiry(v) => {
            if v.instance == [0; 16]
                || v.asset_binding.is_empty()
                || v.position != binding.cast_position
            {
                return Err(Error::InvalidPayload);
            }
        }
        TimerPayload::Familiar(saved) => {
            if saved.lifecycle_epoch == 0 {
                return Err(Error::InvalidPayload);
            }
        }
        TimerPayload::NativeOwnerEffect(saved) => {
            if saved.effect.sub_ordinal >= super::combat_batch::MAX_EFFECTS as u16 {
                return Err(Error::InvalidPayload);
            }
            let super::combat_batch::OwnerCombatChange::MonsterAi(state) = &saved.effect.change
            else {
                return Err(Error::InvalidPayload);
            };
            if state.forced_distance.is_none() && state.challenged_to.is_none() {
                return Err(Error::InvalidPayload);
            }
            if state
                .challenged_to
                .is_some_and(|(target, _)| target != binding.caster)
            {
                return Err(Error::InvalidPayload);
            }
            saved.at_due(r.due)?;
        }
        TimerPayload::FieldEvaluate(v) | TimerPayload::FieldExpiry(v) => {
            if v.instances.len() > 9
                || v.positions.len() > 9
                || v.expires_at < binding.cast_at
                || v.instances.contains(&[0; 16])
                || v.instances.iter().collect::<BTreeSet<_>>().len() != v.instances.len()
                || v.positions.iter().collect::<BTreeSet<_>>().len() != v.positions.len()
            {
                return Err(Error::InvalidPayload);
            }
            if matches!(r.payload, TimerPayload::FieldExpiry(_)) && r.due != v.expires_at {
                return Err(Error::InvalidPayload);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "delayed_execution_tests.rs"]
mod tests;

#[path = "delayed_execution_lifecycle.rs"]
mod lifecycle;
use lifecycle::lifecycle_target_current;
