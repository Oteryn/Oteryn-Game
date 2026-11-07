//! Closed accepted second-life override, owned by actual native creature health.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::QualifiedMagnoliaSecondLife;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MagnoliaLife {
    First,
    QueuedSecond,
    Second,
}
impl ChannelRuntimeV1 {
    /// The immutable catalogue still has52000. Only the already sealed native
    /// Magnolia actor may read its accepted encounter-local60000 override.
    pub(super) fn magnolia_policy_matches(
        &self,
        actor: ExactActorRef,
        expected: &oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy,
        actual: &oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy,
    ) -> Result<bool, CarrierError> {
        let Some(proof) = self
            .companion_policies
            .as_ref()
            .and_then(|table| table.magnolia_second_life())
        else {
            return Ok(false);
        };
        if !matches!(
            self.binding.runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || proof.artifact_digest() != self.content_pin().server_artifact_digest()
            || expected != proof.policy()
        {
            return Ok(false);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            committed,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Ok(false);
        };
        let maximum = match committed.magnolia {
            Some(MagnoliaLife::First) => 52_000,
            Some(MagnoliaLife::Second) => 60_000,
            _ => return Ok(false),
        };
        let mut current = expected.clone();
        current.maximum_health = maximum;
        Ok(spell_combat.maximum_health == maximum && actual == &current)
    }

    /// A real first-life root hit remains retained contribution, while its
    /// qualified queued heal has already prevented the death callback.
    pub fn magnolia_prevented_death(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        matches!(&self.carrier.slots[index],Slot::CreatureOccupied{health,committed,..}
            if *health>0 && committed.magnolia==Some(MagnoliaLife::Second)
                && committed.magnolia_prevented_ordinal.is_some() && committed.lethal().is_none())
    }
    /// Qualified source binding is required before the real encounter starts.
    /// No second Creature identity, client choice, persistent quest flag or fake
    /// health bar is introduced by the phase.
    pub fn bind_magnolia_second_life(
        &mut self,
        actor: ExactActorRef,
        source: &QualifiedMagnoliaSecondLife,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || self.content_pin().server_artifact_digest() != source.artifact_digest()
        {
            return Err(CarrierError::PlanConflict);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            target_identity,
            committed,
            spell_combat,
            companion: Some(companion),
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if target_identity.as_ref() != source.creature_key().as_bytes()
            || companion.master.is_some()
            || companion.policy.as_ref() != source.policy()
            || companion.policy.definition_key != source.creature_key()
            || companion.policy.maximum_health != source.first_health()
            || spell_combat.maximum_health != source.first_health()
            || *health != source.first_health()
            || !committed.entries.is_empty()
            || committed.magnolia.is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        committed.magnolia = Some(MagnoliaLife::First);
        Ok(())
    }
}

/// Drain the accepted counter-reached/full-heal action after the complete root
/// health change, on the same staged slot. No principal/reward callback is used.
pub(super) fn drain_slot(slot: &mut Slot) -> Result<bool, CarrierError> {
    let Slot::CreatureOccupied {
        health,
        committed,
        spell_combat,
        companion: Some(source),
        ..
    } = slot
    else {
        return Ok(false);
    };
    drain_health(committed, spell_combat, source, health)
}
fn drain_health(
    committed: &mut CreatureCommitState,
    combat: &super::runtime_actor_spell::ActorCombatState,
    source: &super::runtime_actor_companion::CompanionState,
    health: &mut i64,
) -> Result<bool, CarrierError> {
    if committed.magnolia != Some(MagnoliaLife::QueuedSecond) {
        return Ok(false);
    }
    if source.policy.definition_key != "oteryn:creature.the_moonsnow_magnolia"
        || source.policy.maximum_health != 60_000
        || combat.maximum_health != 60_000
        || *health != 0
        || committed.magnolia_prevented_ordinal.is_none()
    {
        return Err(CarrierError::PlanConflict);
    }
    *health = combat.maximum_health;
    committed.magnolia = Some(MagnoliaLife::Second);
    Ok(true)
}

/// One finite phase transition, shared by direct hits and the real periodic
/// health/store owner. The caller stages the real HP drain before drain_slot.
pub(super) fn stage_lethal(
    committed: &mut CreatureCommitState,
    combat: &mut super::runtime_actor_spell::ActorCombatState,
    source: Option<&mut super::runtime_actor_companion::CompanionState>,
    ordinal: u64,
    before: i64,
    after: i64,
) -> Result<bool, CarrierError> {
    if before <= 0 || after != 0 || committed.magnolia != Some(MagnoliaLife::First) {
        return Ok(false);
    }
    let source = source.ok_or(CarrierError::NotCreature)?;
    if source.policy.definition_key != "oteryn:creature.the_moonsnow_magnolia"
        || source.policy.maximum_health != 52_000
        || combat.maximum_health != 52_000
    {
        return Err(CarrierError::PlanConflict);
    }
    let mut policy = (*source.policy).clone();
    policy.maximum_health = 60_000;
    source.policy = std::sync::Arc::new(policy);
    combat.maximum_health = 60_000;
    committed.magnolia = Some(MagnoliaLife::QueuedSecond);
    committed.magnolia_prevented_ordinal = Some(ordinal);
    Ok(true)
}
/// A bounded local plan for the already qualified actor. Each due condition
/// pulse is its own health root: its queued heal precedes the next due pulse.
/// The entire cursor/HP/receipt successor still installs atomically once.
#[derive(Debug)]
pub(super) struct PeriodicPhase {
    staged: Slot,
    life_before: i64,
    next: u64,
    prevented: Option<(u64, i64)>,
}
impl PeriodicPhase {
    pub(super) fn from_slot(slot: &Slot) -> Result<Option<Self>, CarrierError> {
        let Slot::CreatureOccupied {
            health,
            committed,
            spell_combat,
            companion: Some(source),
            ..
        } = slot
        else {
            return Ok(None);
        };
        let maximum = match committed.magnolia {
            None => return Ok(None),
            Some(MagnoliaLife::First) => 52_000,
            Some(MagnoliaLife::Second) => 60_000,
            Some(MagnoliaLife::QueuedSecond) => return Err(CarrierError::PlanConflict),
        };
        if source.policy.definition_key != "oteryn:creature.the_moonsnow_magnolia"
            || source.policy.maximum_health != maximum
            || spell_combat.maximum_health != maximum
            || *health <= 0
            || *health > maximum
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(Some(Self {
            staged: slot.clone(),
            life_before: *health,
            next: committed.next_ordinal,
            prevented: None,
        }))
    }
    pub(super) fn maximum(&self) -> i64 {
        match &self.staged {
            Slot::CreatureOccupied { spell_combat, .. } => spell_combat.maximum_health,
            _ => 0,
        }
    }
    pub(super) fn after_hit(
        &mut self,
        source: &mut super::runtime_actor_companion::CompanionState,
        before: i64,
        health: &mut i64,
    ) -> Result<(), CarrierError> {
        let Slot::CreatureOccupied {
            committed,
            spell_combat,
            ..
        } = &mut self.staged
        else {
            return Err(CarrierError::NotCreature);
        };
        if stage_lethal(
            committed,
            spell_combat,
            Some(source),
            self.next,
            before,
            *health,
        )? {
            self.prevented = Some((self.next, self.life_before));
            self.next = self
                .next
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?;
            drain_health(committed, spell_combat, source, health)?;
            self.life_before = *health;
        }
        Ok(())
    }
    pub(super) fn finish(
        mut self,
        source: super::runtime_actor_companion::CompanionState,
        health_after: i64,
        had_damage: bool,
        binding: &[u8],
    ) -> Result<Slot, CarrierError> {
        let Slot::CreatureOccupied {
            health,
            companion,
            committed,
            ..
        } = &mut self.staged
        else {
            return Err(CarrierError::NotCreature);
        };
        let count = usize::from(self.prevented.is_some()) + usize::from(health_after == 0);
        if committed
            .entries
            .len()
            .checked_add(count)
            .is_none_or(|len| len > COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX)
        {
            return Err(CarrierError::DamageReceiptCapacityExceeded);
        }
        committed
            .entries
            .try_reserve(count)
            .map_err(|_| CarrierError::CommitAllocationFailed)?;
        let record = |ordinal, before, suffix: &[u8]| -> Result<OwnerCommitRecord, CarrierError> {
            let mut actual = binding.to_vec();
            actual.extend_from_slice(suffix);
            Ok(OwnerCommitRecord {
                binding: copy_bounded_binding(&actual)?,
                damage: before,
                result: OwnerDamageResult {
                    applied: true,
                    health_before: before,
                    health_after: 0,
                },
                ordinal,
                origin: None,
                reflection: None,
            })
        };
        if let Some((ordinal, before)) = self.prevented {
            committed
                .entries
                .push(record(ordinal, before, b"/magnolia-prevented")?);
        }
        if health_after == 0 {
            committed
                .entries
                .push(record(self.next, self.life_before, b"/magnolia-final")?);
            self.next = self
                .next
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?;
            committed.conditions.die();
        } else if had_damage && self.prevented.is_none() {
            self.next = self
                .next
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        }
        committed.next_ordinal = self.next;
        *health = health_after;
        *companion = Some(Box::new(source));
        Ok(self.staged)
    }
}
