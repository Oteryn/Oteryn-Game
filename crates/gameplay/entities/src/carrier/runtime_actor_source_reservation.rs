//! Commandless owner events reserve the actual fixed slots, without inventing a cast command.
use super::runtime_actor_spell::ActorCombatState;
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, Slot};
use std::sync::Arc;
pub mod source_reservation_seal {
    pub trait Sealed {}
}
pub trait SourceActorReservationProof: source_reservation_seal::Sealed {
    fn owner(&self) -> (ExactActorRef, GameSessionId);
    fn event_identity(&self) -> [u8; 16];
    fn source_binding_digest(&self) -> [u8; 32];
    fn affected_actors(&self) -> &[ExactActorRef];
    /// Independently current physical/source binding, excluding commercial re-grants.
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool;
}
/// A world callback owns an actual current scope, never a fabricated player/session.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceReservationOwner {
    Character(ExactActorRef, GameSessionId),
    Death(ExactActorRef),
    Scope(super::RuntimeScopeRefV1, super::ScopeOwnershipGeneration),
}

pub trait ScopeCreatureReservationProof: source_reservation_seal::Sealed {
    fn scope(&self) -> (super::RuntimeScopeRefV1, super::ScopeOwnershipGeneration);
    fn event_identity(&self) -> [u8; 16];
    fn source_binding_digest(&self) -> [u8; 32];
    fn affected_creatures(&self) -> &[ExactActorRef];
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool;
}

/// Only retained, source-qualified registered GlobalEvent work implements this
/// port. It carries no Player or Creature ownership identity.
pub trait SourceGlobalReservationProof: source_reservation_seal::Sealed {
    fn registration(
        &self,
    ) -> &oteryn_gameplay_content::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent;
    fn work_stamp(&self) -> oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
    fn event_binding(&self) -> [u8; 32];
    fn execution_micros(&self) -> u64;
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceReservationIdentity {
    owner: SourceReservationOwner,

    event: [u8; 16],
    source: [u8; 32],
    death_cursor: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct SourceActorReservation {
    identity: Arc<SourceReservationIdentity>,
    originals: Vec<(usize, Slot)>,
    installed: bool,
}
/// Canonical copied-death action reservation. Its owner is the real BEGIN
/// incarnation/cursor, without inventing a Player session for a NULL death.
#[derive(Debug, Clone)]
pub struct SourceDeathActionReservation {
    inner: SourceActorReservation,
    begin: super::native_source_death::SourceCreatureDeathSeal,
    index: usize,
}
fn state(slot: &Slot) -> Result<&ActorCombatState, CarrierError> {
    match slot {
        Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
            Ok(spell_combat)
        }
        _ => Err(CarrierError::PlanConflict),
    }
}
fn state_mut(slot: &mut Slot) -> Result<&mut ActorCombatState, CarrierError> {
    match slot {
        Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
            Ok(spell_combat)
        }
        _ => Err(CarrierError::PlanConflict),
    }
}
fn identity(proof: &impl SourceActorReservationProof) -> SourceReservationIdentity {
    let (actor, session) = proof.owner();
    SourceReservationIdentity {
        owner: SourceReservationOwner::Character(actor, session),

        event: proof.event_identity(),
        source: proof.source_binding_digest(),
        death_cursor: None,
    }
}
impl ChannelRuntimeV1 {
    /// Reserve the actual callback-time target cohort before detached physical
    /// SQL. Unknown outcomes must retain this same reservation and proof.
    pub fn reserve_source_global_action(
        &mut self,
        proof: &impl SourceGlobalReservationProof,
        affected: &[ExactActorRef],
    ) -> Result<SourceActorReservation, CarrierError> {
        self.owner_fence()?;
        if !proof.current_for(self)
            || proof.registration().content() != self.content_pin().server_artifact_digest()
            || !self.owner_fence()?.accepts_stamp(proof.work_stamp())
            || proof.event_binding() == [0; 32]
            || affected.len() > self.carrier.slots.len()
        {
            return Err(CarrierError::PlanConflict);
        }
        for actor in affected {
            self.carrier.validate_ref(&self.continuity, actor.0)?;
            self.assert_actor_spell_unreserved(*actor)?;
        }
        // Neutral until the real registered callback target reaches this owner.
        Err(CarrierError::PlanConflict)
    }
    #[allow(
        clippy::expect_used,
        reason = "all fixed-slot kinds are validated before atomic reservation publication"
    )]
    pub fn reserve_source_death_action(
        &mut self,
        begin: &super::native_source_death::SourceCreatureDeathSeal,
        event: &oteryn_gameplay_content::content::QualifiedNativeDeathEvent,
        affected: &[ExactActorRef],
    ) -> Result<SourceDeathActionReservation, CarrierError> {
        self.owner_fence()?;
        let index = self
            .source_creature_death_event_index(begin)?
            .ok_or(CarrierError::PlanConflict)?;
        if begin.events().get(index) != Some(event) || affected.len() > self.carrier.slots.len() {
            return Err(CarrierError::PlanConflict);
        }
        let mut originals = Vec::new();
        originals
            .try_reserve_exact(
                affected
                    .len()
                    .checked_add(1)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            )
            .map_err(|_| CarrierError::AllocationFailed)?;
        let source_actor = begin.actor();
        for actor in std::iter::once(&source_actor).chain(affected) {
            let slot = self.carrier.validate_ref(&self.continuity, actor.0)?;
            self.assert_actor_spell_unreserved(*actor)?;
            state(&self.carrier.slots[slot])?;
            if !originals.iter().any(|(old, _)| *old == slot) {
                originals.push((slot, self.carrier.slots[slot].clone()));
            }
        }
        let identity = Arc::new(SourceReservationIdentity {
            owner: SourceReservationOwner::Death(source_actor),
            event: source_actor.placement_identity(),
            source: begin.content_digest(),
            death_cursor: Some(index),
        });
        // All allocation/validation precedes publication to the same fixed-slot
        // pending_source gates used by Player-owned source reservations.
        for (slot, original) in &mut originals {
            state_mut(&mut self.carrier.slots[*slot])
                .expect("prevalidated source slot")
                .pending_source = Some(Arc::clone(&identity));
            state_mut(original)
                .expect("prevalidated source original")
                .pending_source = Some(Arc::clone(&identity));
        }
        Ok(SourceDeathActionReservation {
            inner: SourceActorReservation {
                identity,
                originals,
                installed: true,
            },
            begin: begin.clone(),
            index,
        })
    }
    pub fn validate_source_death_action_reservation(
        &self,
        reservation: &SourceDeathActionReservation,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if self.source_creature_death_event_index(&reservation.begin)? != Some(reservation.index)
            || reservation.inner.identity.owner
                != SourceReservationOwner::Death(reservation.begin.actor())
            || reservation.inner.identity.death_cursor != Some(reservation.index)
            || reservation.inner.identity.source != reservation.begin.content_digest()
            || !reservation.inner.installed
        {
            return Err(CarrierError::PlanConflict);
        }
        for (slot, original) in &reservation.inner.originals {
            if self.carrier.slots.get(*slot) != Some(original) {
                return Err(CarrierError::PlanConflict);
            }
            let state = state(original)?;
            if state.pending_owner.is_some()
                || state.pending_source.as_ref() != Some(&reservation.inner.identity)
            {
                return Err(CarrierError::PlanConflict);
            }
        }
        Ok(())
    }
    #[allow(
        clippy::expect_used,
        reason = "release validates all retained fixed-slot originals before publication"
    )]
    pub fn release_source_death_action_reservation(
        &mut self,
        reservation: SourceDeathActionReservation,
    ) -> Result<(), CarrierError> {
        self.validate_source_death_action_reservation(&reservation)?;
        for (slot, _) in reservation.inner.originals {
            state_mut(&mut self.carrier.slots[slot])
                .expect("validated source slot")
                .pending_source = None;
        }
        Ok(())
    }

    pub fn prepare_source_actor_reservation(
        &self,
        proof: &impl SourceActorReservationProof,
    ) -> Result<SourceActorReservation, CarrierError> {
        self.owner_fence()?;
        let (actor, session) = proof.owner();
        if !proof.current_for(self)
            || proof.affected_actors().len() > self.carrier.slots.len()
            || proof.event_identity() == [0; 16]
            || proof.source_binding_digest() == [0; 32]
            || self
                .player_control_facts(actor, session)?
                .control_loss
                .is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        let mut originals = Vec::new();
        originals
            .try_reserve_exact(proof.affected_actors().len() + 1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for affected in std::iter::once(&actor).chain(proof.affected_actors()) {
            let index = self.carrier.validate_ref(&self.continuity, affected.0)?;
            self.assert_actor_spell_unreserved(*affected)?;
            if originals.iter().any(|(i, _)| *i == index) {
                continue;
            }
            originals.push((index, self.carrier.slots[index].clone()));
        }
        Ok(SourceActorReservation {
            identity: Arc::new(identity(proof)),
            originals,
            installed: false,
        })
    }
    pub fn validate_source_actor_reservation(
        &self,
        prepared: &SourceActorReservation,
        proof: &impl SourceActorReservationProof,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !proof.current_for(self) || *prepared.identity != identity(proof) {
            return Err(CarrierError::PlanConflict);
        }
        for (index, original) in &prepared.originals {
            if self.carrier.slots.get(*index) != Some(original) {
                return Err(CarrierError::PlanConflict);
            }
            let s = state(original)?;
            if s.pending_owner.is_some()
                || (prepared.installed && s.pending_source.as_ref() != Some(&prepared.identity))
                || (!prepared.installed && s.pending_source.is_some())
            {
                return Err(CarrierError::PlanConflict);
            }
        }
        Ok(())
    }
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub fn reserve_source_actors(
        &mut self,
        prepared: &mut SourceActorReservation,
        proof: &impl SourceActorReservationProof,
    ) -> Result<(), CarrierError> {
        self.validate_source_actor_reservation(prepared, proof)?;
        if prepared.installed {
            return Ok(());
        }
        for (index, original) in &mut prepared.originals {
            state_mut(&mut self.carrier.slots[*index])
                .expect("validated actual source slot")
                .pending_source = Some(Arc::clone(&prepared.identity));
            state_mut(original)
                .expect("validated original source slot")
                .pending_source = Some(Arc::clone(&prepared.identity));
        }
        prepared.installed = true;
        Ok(())
    }
    /// The registered compositor calls this only after genuine COMMIT or proven
    /// rollback, followed immediately by source installation in the same owner turn.
    /// Unknown COMMIT retains this reservation; dropping its data never releases it.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub fn release_source_actors_after_observed_outcome(
        &mut self,
        prepared: SourceActorReservation,
        proof: &impl SourceActorReservationProof,
    ) -> Result<(), CarrierError> {
        self.validate_source_actor_reservation(&prepared, proof)?;
        if !prepared.installed {
            return Err(CarrierError::PlanConflict);
        }
        for (index, _) in prepared.originals {
            state_mut(&mut self.carrier.slots[index])
                .expect("validated actual source slot")
                .pending_source = None;
        }
        Ok(())
    }
}

fn scope_identity(proof: &impl ScopeCreatureReservationProof) -> SourceReservationIdentity {
    let (scope, generation) = proof.scope();
    SourceReservationIdentity {
        owner: SourceReservationOwner::Scope(scope, generation),
        event: proof.event_identity(),
        source: proof.source_binding_digest(),
        death_cursor: None,
    }
}

impl ChannelRuntimeV1 {
    /// Crystal movements_extraction_spot.lua:26-49 is a world-owned monster
    /// callback. Reserve the actual source slot while its durable result is
    /// uncertain; ordinary movement/combat/removal already honor pending_source.
    pub fn prepare_scope_creature_reservation(
        &self,
        proof: &impl ScopeCreatureReservationProof,
    ) -> Result<SourceActorReservation, CarrierError> {
        let (scope, generation) = proof.scope();
        if !self.owner_fence()?.is_current_for_scope(scope, generation)
            || !proof.current_for(self)
            || proof.affected_creatures().is_empty()
            || proof.affected_creatures().len() > 256
            || proof.event_identity() == [0; 16]
            || proof.source_binding_digest() == [0; 32]
        {
            return Err(CarrierError::PlanConflict);
        }
        let identity = Arc::new(scope_identity(proof));
        let mut originals = Vec::new();
        originals
            .try_reserve_exact(proof.affected_creatures().len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut installed = None;
        for actor in proof.affected_creatures() {
            let index = self
                .carrier
                .validate_summon_actor(&self.continuity, *actor)?;
            if originals.iter().any(|(i, _)| *i == index)
                || self.carrier.native_summons.has_children(*actor)
            {
                return Err(CarrierError::PlanConflict);
            }
            let combat = state(&self.carrier.slots[index])?;
            if combat.pending_owner.is_some() {
                return Err(CarrierError::PlanConflict);
            }
            let present = match &combat.pending_source {
                None => false,
                Some(pending) if **pending == *identity => true,
                Some(_) => return Err(CarrierError::PlanConflict),
            };
            if installed.is_some_and(|before| before != present) {
                return Err(CarrierError::PlanConflict);
            }
            installed = Some(present);
            originals.push((index, self.carrier.slots[index].clone()));
        }
        // An exact frozen identity can be reconstructed after a projection poll
        // loses local data. A different event, source or owner cannot resume it.
        Ok(SourceActorReservation {
            identity,
            originals,
            installed: installed.unwrap_or(false),
        })
    }

    pub fn validate_scope_creature_reservation(
        &self,
        prepared: &SourceActorReservation,
        proof: &impl ScopeCreatureReservationProof,
    ) -> Result<(), CarrierError> {
        let (scope, generation) = proof.scope();
        if !self.owner_fence()?.is_current_for_scope(scope, generation)
            || !proof.current_for(self)
            || *prepared.identity != scope_identity(proof)
        {
            return Err(CarrierError::PlanConflict);
        }
        for (index, original) in &prepared.originals {
            if self.carrier.slots.get(*index) != Some(original) {
                return Err(CarrierError::PlanConflict);
            }
            let combat = state(original)?;
            if combat.pending_owner.is_some()
                || (prepared.installed
                    && combat.pending_source.as_ref() != Some(&prepared.identity))
                || (!prepared.installed && combat.pending_source.is_some())
            {
                return Err(CarrierError::PlanConflict);
            }
        }
        Ok(())
    }

    #[allow(
        clippy::expect_used,
        reason = "validated current exact source slots cannot change during this owner turn"
    )]
    pub fn reserve_scope_creatures(
        &mut self,
        prepared: &mut SourceActorReservation,
        proof: &impl ScopeCreatureReservationProof,
    ) -> Result<(), CarrierError> {
        self.validate_scope_creature_reservation(prepared, proof)?;
        if prepared.installed {
            return Ok(());
        }
        for (index, original) in &mut prepared.originals {
            state_mut(&mut self.carrier.slots[*index])
                .expect("validated scope source slot")
                .pending_source = Some(Arc::clone(&prepared.identity));
            state_mut(original)
                .expect("validated scope source predecessor")
                .pending_source = Some(Arc::clone(&prepared.identity));
        }
        prepared.installed = true;
        Ok(())
    }

    #[allow(
        clippy::expect_used,
        reason = "validated current exact source slots cannot change during this owner turn"
    )]
    pub fn release_scope_creatures_after_observed_outcome(
        &mut self,
        prepared: SourceActorReservation,
        proof: &impl ScopeCreatureReservationProof,
    ) -> Result<(), CarrierError> {
        self.validate_scope_creature_reservation(&prepared, proof)?;
        if !prepared.installed {
            return Err(CarrierError::PlanConflict);
        }
        for (index, _) in prepared.originals {
            state_mut(&mut self.carrier.slots[index])
                .expect("validated scope source slot")
                .pending_source = None;
        }
        Ok(())
    }

    #[allow(
        clippy::expect_used,
        reason = "all fallible current-source and exact-slot checks precede releasing the pending source"
    )]
    pub fn remove_scope_creature_after_observed_outcome(
        &mut self,
        prepared: SourceActorReservation,
        proof: &impl ScopeCreatureReservationProof,
        actor: ExactActorRef,
        key: &str,
        content: [u8; 32],
    ) -> Result<(), CarrierError> {
        self.validate_scope_creature_reservation(&prepared, proof)?;
        if !prepared.installed
            || !proof.affected_creatures().contains(&actor)
            || self.content_pin().server_artifact_digest() != content
            || !self.matches_live_creature_identity(actor, key.as_bytes())
            || self.carrier.native_summons.has_children(actor)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.carrier
            .validate_summon_actor(&self.continuity, actor)?;
        self.release_scope_creatures_after_observed_outcome(prepared, proof)?;
        self.remove_native_masterless_subject(actor, key, content)
            .expect("validated leaf source removal");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::ControlLossMark;
    use crate::MovementLocalPosition;
    use crate::MovementPositionSnapshot;
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    use oteryn_gameplay_foundation::foundation::ChannelId;
    use oteryn_gameplay_foundation::foundation::NodeId;
    use oteryn_gameplay_foundation::foundation::WorldId;
    fn id(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }
    struct Proof {
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementPositionSnapshot,
        affected: Vec<ExactActorRef>,
        event: [u8; 16],
    }
    impl source_reservation_seal::Sealed for Proof {}
    impl SourceActorReservationProof for Proof {
        fn owner(&self) -> (ExactActorRef, GameSessionId) {
            (self.actor, self.session)
        }
        fn event_identity(&self) -> [u8; 16] {
            self.event
        }
        fn source_binding_digest(&self) -> [u8; 32] {
            [8; 32]
        }
        fn affected_actors(&self) -> &[ExactActorRef] {
            &self.affected
        }
        fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
            runtime.owner_fence().is_ok()
                && runtime.read_actor_position(self.actor) == Ok(self.position)
                && runtime
                    .player_control_facts(self.actor, self.session)
                    .is_ok_and(|p| p.control_loss.is_none())
        }
    }
    #[test]
    fn commandless_original_reserves_real_slots_and_only_exact_prewrite_rollback_releases() {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        let position = match runtime.initialize_first_entry_position(actor).unwrap() {
            super::super::FirstEntryPosition::Initialized(p)
            | super::super::FirstEntryPosition::Reconciled(p) => p,
        };
        let target = runtime
            .admit_test_creature(MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 0,
            })
            .unwrap();
        let unrelated = runtime
            .admit_test_creature(MovementLocalPosition {
                x: 2,
                y: 0,
                floor: 0,
            })
            .unwrap();
        let proof = Proof {
            actor,
            session,
            position,
            affected: vec![target],
            event: id(5),
        };
        let mut reserved = runtime.prepare_source_actor_reservation(&proof).unwrap();
        runtime
            .reserve_source_actors(&mut reserved, &proof)
            .unwrap();
        runtime
            .reserve_source_actors(&mut reserved, &proof)
            .unwrap();
        assert!(runtime.actor_spell_reserved(actor));
        assert!(runtime.actor_spell_reserved(target));
        assert!(!runtime.actor_spell_reserved(unrelated));
        assert_eq!(
            runtime.remove_test_actor(target),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(
            runtime.record_control_loss(
                actor,
                session,
                ControlLossMark {
                    epoch: 1,
                    grace_deadline: 100
                }
            ),
            Err(CarrierError::PlanConflict)
        );
        runtime.remove_test_actor(unrelated).unwrap();
        runtime
            .validate_source_actor_reservation(&reserved, &proof)
            .unwrap();
        let foreign = Proof {
            actor,
            session,
            position,
            affected: vec![target],
            event: id(6),
        };
        assert_eq!(
            runtime.validate_source_actor_reservation(&reserved, &foreign),
            Err(CarrierError::PlanConflict)
        );
        // No SQL operation was issued in this test: this cancellation is proven pre-write.
        runtime
            .release_source_actors_after_observed_outcome(reserved, &proof)
            .unwrap();
        assert!(!runtime.actor_spell_reserved(actor));
        assert!(!runtime.actor_spell_reserved(target));
        runtime.remove_test_actor(target).unwrap();
    }
    #[test]
    fn source_cohort_reservation_is_bounded_by_owned_slots_instead_of_256_spectators() {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            300,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        let position = match runtime.initialize_first_entry_position(actor).unwrap() {
            super::super::FirstEntryPosition::Initialized(p)
            | super::super::FirstEntryPosition::Reconciled(p) => p,
        };
        // This is a fixed-slot owner regression, not an admission of source monsters.
        // A source caller independently qualifies every actual affected actor.
        let affected = (0..257)
            .map(|x| {
                runtime
                    .admit_test_creature(MovementLocalPosition { x, y: 1, floor: 0 })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let proof = Proof {
            actor,
            session,
            position,
            affected,
            event: id(5),
        };
        let prepared = runtime.prepare_source_actor_reservation(&proof);
        assert!(
            prepared.is_ok(),
            "actual owned source cohorts have no 256-spectator gameplay cap: {prepared:?}"
        );
        let mut reservation = prepared.unwrap();
        runtime
            .reserve_source_actors(&mut reservation, &proof)
            .unwrap();
        assert!(
            proof
                .affected
                .iter()
                .all(|a| runtime.actor_spell_reserved(*a))
        );
        runtime
            .validate_source_actor_reservation(&reservation, &proof)
            .unwrap();
        // No SQL has been issued: this is a proven pre-write cancellation.
        runtime
            .release_source_actors_after_observed_outcome(reservation, &proof)
            .unwrap();
        assert!(
            proof
                .affected
                .iter()
                .all(|a| !runtime.actor_spell_reserved(*a))
        );
    }
}

#[cfg(test)]
mod scope_tests {
    #![allow(clippy::unwrap_used)]
    use super::super::{NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome};
    use super::*;
    use crate::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags, MovementLocalPosition,
        MovementPositionSnapshot,
    };
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    use oteryn_gameplay_foundation::foundation::{
        ChannelId, NodeId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
    };
    fn id(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }
    struct Proof {
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        actors: [ExactActorRef; 1],
        position: MovementPositionSnapshot,
        event: [u8; 16],
    }
    impl source_reservation_seal::Sealed for Proof {}
    impl ScopeCreatureReservationProof for Proof {
        fn scope(&self) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration) {
            (self.scope, self.generation)
        }
        fn event_identity(&self) -> [u8; 16] {
            self.event
        }
        fn source_binding_digest(&self) -> [u8; 32] {
            [9; 32]
        }
        fn affected_creatures(&self) -> &[ExactActorRef] {
            &self.actors
        }
        fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
            runtime.read_actor_position(self.actors[0]) == Ok(self.position)
        }
    }
    fn fixture() -> (ChannelRuntimeV1, Proof) {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let policy = CompiledCreaturePolicy {
            definition_key: "creature:rat".into(),
            definition_revision: "test-1".into(),
            display_name: "rat".into(),
            maximum_health: 20,
            forge_difficulty: None,
            base_speed: 220,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: Vec::new(),
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
            defense: Some(1),
            armor: Some(0),
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: Vec::new(),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };
        r.install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                r.content_pin().server_artifact_digest(),
                vec![policy],
            )
            .unwrap(),
        )
        .unwrap();
        let spec = NativeRaidSpawnSpec::qualified(
            "creature:rat",
            "test-1",
            r.content_pin().server_artifact_digest(),
            false,
        )
        .unwrap();
        let stamp = r.issue_owner_work().unwrap();
        let placed = r
            .realize_raid_spawn(
                stamp,
                RaidSpawnIdentity {
                    firing_id: id(7),
                    wave: 0,
                    entry: 0,
                    index: 0,
                },
                &spec,
                Some(MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                }),
                |_, _| Some(true),
            )
            .unwrap();
        let actor = match placed.outcome {
            RaidSpawnOutcome::Placed(actor) => Some(actor),
            _ => None,
        };
        assert!(actor.is_some(), "native raid fixture must place a creature");
        let actor = actor.unwrap();
        let p = Proof {
            scope: r.binding().runtime_scope(),
            generation: r.binding().scope_generation(),
            actors: [actor],
            position: r.read_actor_position(actor).unwrap(),
            event: id(8),
        };
        (r, p)
    }
    #[test]
    fn w05d_scope_source_unknown_outcome_blocks_real_movement_and_resumes_exact_frozen_identity() {
        let (mut r, p) = fixture();
        let actor = p.actors[0];
        let mut prepared = r.prepare_scope_creature_reservation(&p).unwrap();
        r.reserve_scope_creatures(&mut prepared, &p).unwrap();
        assert!(r.actor_spell_reserved(actor));
        let health_before = r.companion_snapshot(actor).unwrap().health;
        assert!(health_before > 0);
        assert!(
            r.commit_monster_lab_damage(actor, b"scope-pending-lethal", health_before)
                .is_err()
        );
        assert_eq!(r.companion_snapshot(actor).unwrap().health, health_before);
        assert!(r.contains_live_creature(actor));
        assert_eq!(
            r.borrow_movement_position().commit_creature_relocation(
                actor,
                p.position,
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0
                }
            ),
            Err(CarrierError::PlanConflict)
        );
        drop(prepared); // Unknown COMMIT is not a rollback or a release.
        assert!(r.actor_spell_reserved(actor));
        let mut resumed = r.prepare_scope_creature_reservation(&p).unwrap();
        r.reserve_scope_creatures(&mut resumed, &p).unwrap();
        let foreign = Proof {
            scope: p.scope,
            generation: p.generation,
            actors: p.actors,
            position: p.position,
            event: id(9),
        };
        assert!(r.prepare_scope_creature_reservation(&foreign).is_err());
        r.release_scope_creatures_after_observed_outcome(resumed, &p)
            .unwrap();
        assert!(!r.actor_spell_reserved(actor));
        assert!(
            r.borrow_movement_position()
                .commit_creature_relocation(
                    actor,
                    p.position,
                    MovementLocalPosition {
                        x: 2,
                        y: 0,
                        floor: 0
                    }
                )
                .is_ok()
        );
        r.commit_monster_lab_damage(actor, b"scope-observed-lethal", health_before)
            .unwrap();
        assert!(!r.contains_live_creature(actor));
    }
    #[test]
    fn w05d_scope_source_requires_the_actual_current_scope_fence_before_reservation() {
        let (r, mut p) = fixture();
        p.generation = ScopeOwnershipGeneration::new(2).unwrap();
        assert!(r.prepare_scope_creature_reservation(&p).is_err());
        assert!(!r.actor_spell_reserved(p.actors[0]));
        p.generation = r.binding().scope_generation();
        p.scope = RuntimeScopeRefV1::Channel {
            world_id: WorldId::decode(&id(1)).unwrap(),
            channel_id: ChannelId::decode(&id(8)).unwrap(),
        };
        assert!(r.prepare_scope_creature_reservation(&p).is_err());
        assert!(!r.actor_spell_reserved(p.actors[0]));
    }
}
