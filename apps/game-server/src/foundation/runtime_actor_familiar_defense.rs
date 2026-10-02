//! Source self-defense runs on the actual familiar slot, not a player cast or
//! a second HP/cache owner. Its private receipt is the presentation authority.
use super::runtime_actor_companion::{
    CompanionSnapshot, CompanionState, FamiliarDefenseClock, FamiliarSelfHealDefense,
};
use super::*;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct PreparedFamiliarDefense {
    before: CompanionSnapshot,
    slot: Box<Slot>,
    next: CompanionState,
    health_after: i64,
    stamp: super::super::RuntimeWorkStamp,
    policy: FamiliarSelfHealDefense,
    successful: bool,
}
#[derive(Debug)]
pub(crate) struct FamiliarDefenseReceipt {
    actor: ExactActorRef,
    position: MovementLocalPosition,
    profile_digest: [u8; 32],
    content_digest: [u8; 32],
    ordinal: u64,
    health_before: i64,
    health_after: i64,
    effect: Option<String>,
}
impl FamiliarDefenseReceipt {
    pub(crate) fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub(crate) fn profile_digest(&self) -> [u8; 32] {
        self.profile_digest
    }
    pub(crate) fn content_digest(&self) -> [u8; 32] {
        self.content_digest
    }
    pub(crate) fn ordinal(&self) -> u64 {
        self.ordinal
    }
    pub(crate) fn health_before(&self) -> i64 {
        self.health_before
    }
    pub(crate) fn health_after(&self) -> i64 {
        self.health_after
    }
    pub(crate) fn effect(&self) -> Option<&str> {
        self.effect.as_deref()
    }
}
impl PreparedFamiliarDefense {
    pub(crate) fn expected(&self) -> &CompanionSnapshot {
        &self.before
    }
    pub(crate) fn profile(&self) -> &FamiliarSelfHealDefense {
        &self.policy
    }
    pub(crate) fn ordinal(&self) -> u64 {
        self.next
            .familiar_defense
            .as_ref()
            .map_or(0, |clock| clock.ordinal)
    }
    pub(crate) fn successful(&self) -> bool {
        self.successful
    }
}
fn due(
    snapshot: &CompanionSnapshot,
    policy: &FamiliarSelfHealDefense,
    now_us: u64,
) -> Result<Option<(u64, u64)>, CarrierError> {
    let born = snapshot
        .state
        .created_at_us
        .ok_or(CarrierError::PlanConflict)?;
    if now_us < born {
        return Err(CarrierError::PlanConflict);
    }
    let interval = u64::from(policy.interval_ms)
        .checked_mul(1000)
        .ok_or(CarrierError::PlanConflict)?;
    let (next_due, ordinal) = match &snapshot.state.familiar_defense {
        Some(clock) if clock.profile_digest == policy.profile_digest => {
            (clock.next_due_us, clock.ordinal)
        }
        Some(_) => return Err(CarrierError::PlanConflict),
        None => (
            born.checked_add(interval)
                .ok_or(CarrierError::PlanConflict)?,
            0,
        ),
    };
    Ok((now_us >= next_due).then_some((next_due, ordinal)))
}
impl ChannelRuntimeV1 {
    fn qualify_familiar_defense(
        &self,
        actor: ExactActorRef,
        policy: &FamiliarSelfHealDefense,
        now_unix: i64,
    ) -> Result<CompanionSnapshot, CarrierError> {
        if self.actor_spell_reserved(actor)
            || policy.source_digest != self.content_pin().server_artifact_digest()
            || policy.profile_digest == [0; 32]
            || policy.interval_ms == 0
            || policy.chance_percent > 100
            || policy.heal_min <= 0
            || policy.heal_min != policy.heal_max
            || policy.effect != "CONST_ME_MAGIC_GREEN"
        {
            return Err(CarrierError::PlanConflict);
        }
        let snapshot = self.companion_snapshot(actor)?;
        let master = snapshot
            .state
            .master
            .as_ref()
            .ok_or(CarrierError::PlanConflict)?;
        if !snapshot.state.policy.is_familiar
            || snapshot.state.policy.definition_key != policy.creature_key
            || snapshot.state.policy.definition_revision != policy.creature_revision
            || snapshot.health <= 0
            || snapshot.maximum_health != snapshot.state.policy.maximum_health
            || snapshot.health > snapshot.maximum_health
            || snapshot.state.lifecycle_epoch == 0
            || snapshot
                .state
                .expires_at_unix
                .is_some_and(|until| until <= now_unix)
            || !self
                .player_control_facts(master.actor, master.session)
                .is_ok_and(|f| f.control_loss.is_none())
            || self.actor_spell_reserved(master.actor)
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(snapshot)
    }
    /// Qualification/capacity checks may precede RNG in the same owner turn.
    pub(crate) fn familiar_defense_due(
        &self,
        actor: ExactActorRef,
        policy: &FamiliarSelfHealDefense,
        now_us: u64,
        now_unix: i64,
    ) -> Result<bool, CarrierError> {
        let snapshot = self.qualify_familiar_defense(actor, policy, now_unix)?;
        Ok(due(&snapshot, policy, now_us)?.is_some())
    }
    pub(crate) fn prepare_familiar_defense(
        &mut self,
        actor: ExactActorRef,
        policy: &FamiliarSelfHealDefense,
        now_us: u64,
        now_unix: i64,
    ) -> Result<Option<PreparedFamiliarDefense>, CarrierError> {
        let before = self.qualify_familiar_defense(actor, policy, now_unix)?;
        let Some((source_due, ordinal)) = due(&before, policy, now_us)? else {
            return Ok(None);
        };
        let next_ordinal = ordinal.checked_add(1).ok_or(CarrierError::PlanConflict)?;
        // Source onThinkDefense resets the single due spell's accumulated ticks;
        // one delayed owner cycle casts at most once, never an offline catch-up.
        let next_due = now_us
            .checked_add(u64::from(policy.interval_ms) * 1000)
            .ok_or(CarrierError::PlanConflict)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let slot = Box::new(self.carrier.slots[index].clone());
        let mut bytes = Sha256::new();
        bytes.update(b"oteryn:familiar-self-defense:v1\0");
        bytes.update(actor.placement_identity());
        bytes.update(policy.profile_digest);
        bytes.update(before.state.lifecycle_epoch.to_be_bytes());
        bytes.update(ordinal.to_be_bytes());
        bytes.update(source_due.to_be_bytes());
        let hash = bytes.finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        let root = GameplayDecisionRoot::from_bytes(policy.source_digest);
        let sample = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "familiar.defense.chance",
            0,
        )
        .map_err(|_| CarrierError::PlanConflict)?;
        let roll = 1 + u8::try_from((u128::from(sample) * 100) >> 64)
            .map_err(|_| CarrierError::PlanConflict)?;
        let successful = roll <= policy.chance_percent;
        // Canonical selected source vectors have a fixed positive heal. An
        // unowned variable range is refused before any physical mutation.
        if policy.heal_min != policy.heal_max {
            return Err(CarrierError::PlanConflict);
        }
        let health_after = if successful {
            before
                .health
                .checked_add(policy.heal_min)
                .ok_or(CarrierError::DamageOverflow)?
                .min(before.maximum_health)
        } else {
            before.health
        };
        let mut next = before.state.clone();
        next.familiar_defense = Some(FamiliarDefenseClock {
            profile_digest: policy.profile_digest,
            next_due_us: next_due,
            ordinal: next_ordinal,
        });
        let stamp = self.issue_owner_work()?;
        Ok(Some(PreparedFamiliarDefense {
            before,
            slot,
            next,
            health_after,
            stamp,
            policy: policy.clone(),
            successful,
        }))
    }
    pub(crate) fn commit_familiar_defense(
        &mut self,
        prepared: PreparedFamiliarDefense,
    ) -> Result<FamiliarDefenseReceipt, CarrierError> {
        if !self.owner_fence()?.accepts_stamp(prepared.stamp)
            || self.actor_spell_reserved(prepared.before.actor)
        {
            return Err(CarrierError::PlanConflict);
        }
        let current = self.companion_snapshot(prepared.before.actor)?;
        let master = current
            .state
            .master
            .as_ref()
            .ok_or(CarrierError::PlanConflict)?;
        if current != prepared.before
            || !self
                .player_control_facts(master.actor, master.session)
                .is_ok_and(|f| f.control_loss.is_none())
            || self.actor_spell_reserved(master.actor)
        {
            return Err(CarrierError::PlanConflict);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.before.actor.0)?;
        if self.carrier.slots[index] != *prepared.slot {
            return Err(CarrierError::PlanConflict);
        }
        let mut next_slot = *prepared.slot;
        let Slot::CreatureOccupied {
            health,
            companion: Some(state),
            ..
        } = &mut next_slot
        else {
            return Err(CarrierError::PlanConflict);
        };
        *health = prepared.health_after;
        *state = Box::new(prepared.next);
        let ordinal = state
            .familiar_defense
            .as_ref()
            .ok_or(CarrierError::PlanConflict)?
            .ordinal;
        let receipt = FamiliarDefenseReceipt {
            actor: prepared.before.actor,
            position: prepared.before.position,
            profile_digest: prepared.policy.profile_digest,
            content_digest: prepared.policy.source_digest,
            ordinal,
            health_before: prepared.before.health,
            health_after: prepared.health_after,
            effect: prepared.successful.then_some(prepared.policy.effect),
        };
        // Actual existing slot owns both HP and the defense clock. This is the
        // sole mutation; all proof/arithmetic checks precede it.
        self.carrier.slots[index] = next_slot;
        Ok(receipt)
    }
}
