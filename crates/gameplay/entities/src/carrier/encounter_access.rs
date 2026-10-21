//! Generic owner operations used by encounter rules in the runtime layer.
use super::*;

pub type EncounterHealProjection = (Vec<(ExactActorRef, String, u64, u64)>, Vec<usize>);

/// Runtime-owned encounter state. The carrier calls these at the existing damage,
/// heal and corpse-publication boundaries, while holding the same owner borrow.
pub trait EncounterHooks: std::fmt::Debug + Send + Sync + std::any::Any {
    fn clone_hooks(&self) -> Box<dyn EncounterHooks>;
    fn eq_hooks(&self, other: &dyn EncounterHooks) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn contains(&self, actor: ExactActorRef) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn damage(
        &self,
        carrier: &mut ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail: bool,
        pin: Option<&ChannelContentPin>,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Option<Result<OwnerDamageResult, CarrierError>>;
    fn projected_death(&mut self, actor: ExactActorRef, death: CreatureDeathOccurrenceKey);
    fn expand_heals(
        &self,
        runtime: &ChannelRuntimeV1,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError>;
}

impl Clone for Box<dyn EncounterHooks> {
    fn clone(&self) -> Self {
        self.clone_hooks()
    }
}
impl PartialEq for dyn EncounterHooks {
    fn eq(&self, other: &Self) -> bool {
        self.eq_hooks(other)
    }
}
impl Eq for dyn EncounterHooks {}

impl ChannelActorCarrier {
    pub fn encounter_native_summon_role(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.validate_summon_actor(continuity, actor)?;
        Ok(self.native_summons.child(actor).map(|link| link.parent))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_encounter_single_damage(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail: bool,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let (index, next, result) = self.prepare_creature_damage_inner_bounded(
            continuity,
            actor,
            command,
            attacker,
            fail,
            max_sub_ordinal,
            false,
            now_ms,
        )?;
        if let Some(next) = next {
            self.slots[index] = next;
        }
        Ok(result)
    }

    pub fn encounter_slots(&self) -> &[Slot] {
        &self.slots
    }
    pub fn encounter_actor_index(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
    ) -> Result<usize, CarrierError> {
        self.validate_ref(continuity, actor)
    }
    pub fn encounter_scope(&self) -> (WorldId, ChannelId, ScopeOwnershipGeneration) {
        (self.world_id, self.channel_id, self.scope_generation)
    }
    pub fn encounter_is_native_summon(&self, actor: ExactActorRef) -> bool {
        self.is_native_summon(actor)
    }

    /// Stage all physical slots before publication. The rule owner rechecks its
    /// group and result invariants after staging; no write precedes that check.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_encounter_damage_batch<const N: usize, E: From<CarrierError>>(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actors: [ExactActorRef; N],
        keys: [&[u8]; N],
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail: bool,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
        validate: impl FnOnce(&Self, &[usize; N], &[OwnerDamageResult; N]) -> Result<(), E>,
    ) -> Result<[OwnerDamageResult; N], E> {
        let mut indexes = [0; N];
        let mut staged: [Option<Slot>; N] = std::array::from_fn(|_| None);
        let mut results = [OwnerDamageResult {
            applied: false,
            health_before: 0,
            health_after: 0,
        }; N];
        for i in 0..N {
            let (index, next, result) = self.prepare_creature_damage_inner_bounded(
                continuity,
                actors[i].0,
                OwnerDamageCommand {
                    target: keys[i],
                    occurrence: command.occurrence,
                    binding: command.binding,
                    damage: command.damage,
                },
                attacker,
                fail,
                max_sub_ordinal,
                false,
                now_ms,
            )?;
            indexes[i] = index;
            staged[i] = next;
            results[i] = result;
        }
        validate(self, &indexes, &results)?;
        for (index, next) in indexes.into_iter().zip(staged) {
            if let Some(next) = next {
                self.slots[index] = next;
            }
        }
        Ok(results)
    }
}

impl ChannelRuntimeV1 {
    pub fn encounter_owner_scope(&self) -> (WorldId, ChannelId, ScopeOwnershipGeneration) {
        (
            self.continuity.world_id,
            self.continuity.channel_id,
            self.continuity.current_generation,
        )
    }
    pub fn encounter_parts(&self) -> (&ChannelActorCarrier, &NamespaceContinuityGuard) {
        (&self.carrier, &self.continuity)
    }

    pub fn encounter_parts_mut(&mut self) -> (&mut ChannelActorCarrier, &NamespaceContinuityGuard) {
        (&mut self.carrier, &self.continuity)
    }

    /// As [`Self::encounter_parts_mut`], plus the pinned Content for owner checks made during the
    /// write. The pin is `Clone` only in test builds, so production borrows it.
    pub fn encounter_parts_with_content_mut(
        &mut self,
    ) -> (
        &mut ChannelActorCarrier,
        &NamespaceContinuityGuard,
        &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    ) {
        (&mut self.carrier, &self.continuity, &self.content)
    }

    pub fn encounter_hooks(&self) -> Option<&dyn EncounterHooks> {
        self.carrier.encounter_hooks.as_deref()
    }

    /// Install the runtime's hooks at encounter registration, before its first
    /// actor mutation. Temporarily splitting the state keeps the owner exclusive.
    pub fn with_encounter_hooks<T: EncounterHooks + Default, R, E: From<CarrierError>>(
        &mut self,
        operation: impl FnOnce(&mut Self, &mut T) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut hooks = self
            .carrier
            .encounter_hooks
            .take()
            .unwrap_or_else(|| Box::<T>::default());
        let result = if let Some(state) = hooks.as_any_mut().downcast_mut::<T>() {
            operation(self, state)
        } else {
            Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict.into())
        };
        self.carrier.encounter_hooks = Some(hooks);
        result
    }

    pub(super) fn expand_encounter_heals(
        &self,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError> {
        if let Some(hooks) = self.encounter_hooks() {
            return hooks.expand_heals(self, requests);
        }
        let mut expanded = Vec::new();
        expanded.try_reserve(64).map_err(|_| {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed
        })?;
        let mut map = Vec::new();
        map.try_reserve(requests.len()).map_err(|_| {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed
        })?;
        for (n, (actor, key, max, draw)) in requests.iter().enumerate() {
            if *max == 0 || *max > i64::MAX as u64 || *draw > i64::MAX as u64 {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::InvalidCreatureHealth,
                );
            }
            if requests[..n].iter().any(|r| r.0 == *actor) {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                );
            }
            if expanded.len() >= 64 {
                return Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::AllocationFailed,
                );
            }
            map.push(expanded.len());
            expanded.push((*actor, key.clone(), *max, *draw));
        }
        Ok((expanded, map))
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_encounter_fixture_actor(
        &mut self,
        state: ActorState,
        key: &str,
        health: i64,
        at: Option<LocalPosition>,
    ) -> Result<ActorRef, CarrierError> {
        let actor = self
            .carrier
            .admit_creature(&self.continuity, state, key, health)?;
        if let Some(at) = at {
            self.carrier.initialize_position(
                &self.continuity,
                actor,
                self.pinned_position_context(),
                at,
            )?;
        }
        Ok(actor)
    }

    pub fn encounter_actor_index(&self, actor: ExactActorRef) -> Result<usize, CarrierError> {
        self.carrier.validate_ref(&self.continuity, actor.0)
    }

    pub fn encounter_slot(&self, index: usize) -> &Slot {
        &self.carrier.slots[index]
    }

    pub fn encounter_slots(&self) -> &[Slot] {
        &self.carrier.slots
    }

    pub fn encounter_player_controlled(&self, index: usize) -> bool {
        matches!(&self.carrier.slots[index], Slot::Occupied { lifecycle, .. } if lifecycle.control_loss.is_none())
    }

    pub fn read_encounter_position(
        &self,
        actor: ExactActorRef,
    ) -> Result<PositionSnapshot, CarrierError> {
        self.carrier.read_position(&self.continuity, actor.0)
    }

    pub fn encounter_position_context(&self) -> PreProductionPositionContext {
        self.pinned_position_context()
    }

    pub fn encounter_death_is_current(&self, key: CreatureDeathOccurrenceKey) -> bool {
        self.carrier.validate_ref(&self.continuity, key.0).is_ok()
            && self
                .carrier
                .corpse_projections
                .iter()
                .any(|p| p.occurrence.death_key() == key)
    }

    pub fn remove_encounter_actor(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }

    /// Admission and position initialization under the owner's actual pin. A failed
    /// initialization retires the just-admitted generation before returning its error.
    pub fn admit_encounter_actor(
        &mut self,
        key: &str,
        health: i64,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.pinned_position_context();
        let created = self
            .carrier
            .admit_creature(&self.continuity, ActorState(0), key, health)?;
        if let Err(error) = self.carrier.initialize_position(
            &self.continuity,
            created,
            context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        ) {
            self.carrier.remove(&self.continuity, created)?;
            return Err(error);
        }
        Ok(ExactActorRef(created))
    }

    pub fn assert_encounter_slot_unreserved(&self, index: usize) -> Result<(), CarrierError> {
        self.carrier.assert_slot_spell_unreserved(index)
    }

    /// The caller finishes all rule checks and allocations before consuming a prey.
    /// Removal remains the native administrative path, including intrinsic children.
    pub fn commit_encounter_consumption(
        &mut self,
        prey: ExactActorRef,
        consumer: ExactActorRef,
        health_after: i64,
    ) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, consumer.0)?;
        if !matches!(&self.carrier.slots[index], Slot::CreatureOccupied { generation, .. }
            if *generation == consumer.actor_local_generation())
        {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration,
            );
        }
        self.carrier.remove(&self.continuity, prey.0)?;
        if let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[index] {
            *health = health_after;
        } else {
            unreachable!("same owner work item preserves consumer slot")
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, PartialEq, Eq)]
pub struct EncounterFixtureSnapshot(Box<[Slot]>);

#[cfg(any(test, feature = "test-support"))]
impl ChannelRuntimeV1 {
    pub fn advance_encounter_fixture_actor_generation(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        if let Slot::CreatureOccupied { generation, .. } = &mut self.carrier.slots[index] {
            *generation += 1;
            Ok(())
        } else {
            Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature)
        }
    }

    pub fn encounter_fixture_summon_progress(&self) -> (usize, u64) {
        (
            self.carrier.native_summons.len(),
            self.carrier.native_summons.retire_cursor,
        )
    }

    pub fn set_encounter_fixture_retire_cursor(&mut self, cursor: u64) {
        self.carrier.native_summons.retire_cursor = cursor;
    }
    pub fn replace_encounter_fixture_content(&mut self, content: ChannelContentPin) {
        self.content = content;
    }
    #[allow(clippy::expect_used)]
    pub fn read_encounter_fixture_health(&self, actor: ExactActorRef) -> i64 {
        let index = self
            .encounter_actor_index(actor)
            .expect("valid native fixture operation must succeed");
        match self.encounter_slot(index) {
            Slot::CreatureOccupied { health, .. } => Some(*health),
            _ => None,
        }
        .expect("fixture requires native creature")
    }
    pub fn encounter_fixture_snapshot(&self) -> EncounterFixtureSnapshot {
        EncounterFixtureSnapshot(self.carrier.slots.clone())
    }

    pub fn encounter_fixture_death_counts(&self) -> (usize, usize) {
        (
            self.carrier.corpse_projections.len(),
            self.carrier.death_reward_occurrences.len(),
        )
    }

    pub fn encounter_fixture_advance(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<(), CarrierError> {
        self.continuity
            .advance(PreProductionContinuityGrant::fixture(
                self.carrier.world_id,
                self.carrier.channel_id,
                generation,
            ))
    }
}

#[cfg(any(test, feature = "test-support"))]
impl CreatureSelfHealRegistration {
    pub fn encounter_fixture_bounds(&self) -> (i64, i64, i64) {
        (self.max_health, self.minimum, self.maximum)
    }
    pub fn encounter_fixture_change_server(&mut self) {
        self.server[0] ^= 1;
    }
}

#[cfg(any(test, feature = "test-support"))]
impl CreatureSelfHealLedger {
    pub fn encounter_fixture_entries_len(&self) -> usize {
        self.entries.len()
    }
    pub fn encounter_fixture_sequence(&self, actor: ExactActorRef, key: &str) -> Option<u64> {
        self.entries.get(actor, key).map(|entry| entry.sequence)
    }
}
