use super::*;

impl ChannelRuntimeV1 {
    /// CREATURE-AI-1: the health of live creature `actor`.
    pub fn live_creature_health(&self, actor: ExactActorRef) -> Option<u64> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                generation, health, ..
            } if *generation == actor.0.actor_local_generation.0 && *health > 0 => {
                u64::try_from(*health).ok()
            }
            _ => None,
        }
    }

    /// Test only: sets a live creature's health directly.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_test_creature_health(
        &mut self,
        actor: ExactActorRef,
        value: i64,
    ) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied { health, .. } => {
                *health = value;
                Ok(())
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
}

// Main reconciliation packet Foundation-1: source heal only, no lifecycle takeover.
// Append inside physical Foundation carrier. Loaded typed membership qualification is separate.
#[derive(Debug, Clone)]
pub struct CreatureSelfHealRegistration {
    world: WorldId,
    activation: u64,
    pub(super) server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    target: Box<str>,
    ability: Box<str>,
    pub(super) max_health: i64,
    pub(super) minimum: i64,
    pub(super) maximum: i64,
}

#[derive(Debug, Default)]
pub struct CreatureSelfHealLedger {
    /// By creature and defence Ability (Codex 4243039220).
    pub(super) entries: ActorMemoTable<Box<str>, CreatureSelfHealEntry>,
}

#[derive(Debug)]
pub(super) struct CreatureSelfHealEntry {
    pub(super) sequence: u64,
    draw: i64,
    result: OwnerDamageResult,
}

impl ChannelRuntimeV1 {
    /// Composition loader supplies an exact current Creature->defense Ability->Heal Effect->Range
    /// chain. Native registration fences the immutable definition facts to this activation.
    /// Never call this from network input or infer membership merely from an artifact digest.
    pub fn bind_creature_self_heal(
        &self,
        target: &str,
        ability: &str,
        max_health: u64,
        minimum: u64,
        maximum: u64,
    ) -> Result<CreatureSelfHealRegistration, CarrierError> {
        if !target.starts_with("oteryn:creature.")
            || !ability.starts_with("oteryn:ability.")
            || max_health == 0
            || minimum > maximum
            || maximum > i64::MAX as u64
            || max_health > i64::MAX as u64
        {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        Ok(CreatureSelfHealRegistration {
            world: self.content.world_id(),
            activation: self.content.activation_sequence(),
            server: self.content.server_artifact_digest(),
            client: self.content.client_artifact_digest(),
            frame: self.content.frame_binding_digest(),
            map: self.content.map_revision_digest(),
            target: target.into(),
            ability: ability.into(),
            max_health: max_health as i64,
            minimum: minimum as i64,
            maximum: maximum as i64,
        })
    }
    /// Actual native creature HP writer, no Effect fixture map. Ordered think/cast sequence per (caster, defense ability). Independent defenses may
    /// execute in the same Think, including Think0; A+B+retryA cannot reapply A.
    pub fn commit_creature_self_heal(
        &mut self,
        ledger: &mut CreatureSelfHealLedger,
        registration: &CreatureSelfHealRegistration,
        actor: ExactActorRef,
        sequence: u64,
        server_draw: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if registration.world != self.content.world_id()
            || registration.activation != self.content.activation_sequence()
            || registration.server != self.content.server_artifact_digest()
            || registration.client != self.content.client_artifact_digest()
            || registration.frame != self.content.frame_binding_digest()
            || registration.map != self.content.map_revision_digest()
        {
            return Err(CarrierError::PositionContextMismatch);
        }
        if !(registration.minimum..=registration.maximum).contains(&server_draw) {
            return Err(CarrierError::InvalidDamage);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let before = match &self.carrier.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation != actor.0.actor_local_generation.0 =>
            {
                return Err(CarrierError::StaleActorGeneration);
            }
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == registration.target.as_bytes() && *health > 0 => {
                *health
            }
            Slot::CreatureOccupied {
                target_identity, ..
            } if target_identity.as_ref() != registration.target.as_bytes() => {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            _ => return Err(CarrierError::CreatureNotActionable),
        };
        if before > registration.max_health {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        // The live actor's slot drops its former occupant's entries; the other dead actors'
        // entries go only when the ledger is full (Codex 4243039220).
        ledger.entries.evict_stale_at(actor);
        let known = match ledger.entries.get(actor, &*registration.ability) {
            Some(entry) => {
                if sequence < entry.sequence {
                    return Err(CarrierError::StaleAttackerSequence);
                }
                if sequence == entry.sequence {
                    if entry.draw != server_draw {
                        return Err(CarrierError::PlanConflict);
                    }
                    return Ok(OwnerDamageResult {
                        applied: false,
                        ..entry.result
                    });
                }
                true
            }
            None => false,
        };
        if !known {
            if ledger.entries.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                ledger.entries.prune(|e| self.contains_live_creature(e));
            }
            if ledger.entries.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
                || ledger.entries.actor_len(actor) >= CREATUREAI0_DEFENCES_PER_CREATURE_MAX
            {
                return Err(CarrierError::AllocationFailed);
            }
        }
        // Existing native authoring admits at most8 defenses; AI active actors ceiling64.
        // Aggregate current-owner ScopeRuntimeFence is required at the composition call.
        // Allocate source binding before native HP write, including replay state replacement.
        let ability = registration.ability.clone();
        let after = before
            .saturating_add(server_draw)
            .min(registration.max_health);
        let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        *health = after;
        let result = OwnerDamageResult {
            applied: true,
            health_before: before,
            health_after: after,
        };
        let entry = CreatureSelfHealEntry {
            sequence,
            draw: server_draw,
            result,
        };
        ledger.entries.insert(actor, ability, entry);
        Ok(result)
    }
    pub fn current_live_creature_identity(
        &self,
        actor: ExactActorRef,
    ) -> Result<&[u8], CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                ..
            } if *generation == actor.0.actor_local_generation.0 && *health > 0 => {
                Ok(target_identity.as_ref())
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }
    /// Source-qualified bounded draws from this physical Channel owner; exact current pin,
    /// all actor generations/identity/HP and output storage preflight before publication.
    /// Existing CreatureCommitState including its canonical conditions is preserved intact.
    pub fn commit_source_creature_heal_batch(
        &mut self,
        content: [u8; 32],
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<Vec<OwnerDamageResult>, CarrierError> {
        if content != self.content.server_artifact_digest() {
            return Err(CarrierError::PositionContextMismatch);
        }
        if requests.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        let original_len = requests.len();
        let (expanded, original_map) = self.expand_encounter_heals(requests)?;
        let requests = expanded.as_slice();
        let mut original_results = Vec::new();
        original_results
            .try_reserve(original_len)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut prepared = Vec::new();
        prepared
            .try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut results = Vec::new();
        results
            .try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (actor, key, max, draw) in requests {
            if *max == 0 || *max > i64::MAX as u64 || *draw > i64::MAX as u64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
            self.carrier.assert_slot_spell_unreserved(index)?;
            if prepared.iter().any(|(old, _)| *old == index) {
                return Err(CarrierError::PlanConflict);
            }
            let mut next = self.carrier.slots[index].clone();
            let Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                ..
            } = &mut next
            else {
                return Err(CarrierError::NotCreature);
            };
            if *generation != actor.0.actor_local_generation.0 {
                return Err(CarrierError::StaleActorGeneration);
            }
            if target_identity.as_ref() != key.as_bytes() || *health <= 0 {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            if *health > *max as i64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            let before = *health;
            let after = before.saturating_add(*draw as i64).min(*max as i64);
            *health = after;
            prepared.push((index, next));
            results.push(OwnerDamageResult {
                applied: true,
                health_before: before,
                health_after: after,
            });
        }
        for index in original_map {
            original_results.push(*results.get(index).ok_or(CarrierError::PlanConflict)?);
        }
        // All fallible work, including original result projection and canonical condition clones, precedes HP publication.
        for (index, next) in prepared {
            self.carrier.slots[index] = next;
        }
        Ok(original_results)
    }
    /// Same native physical HP owner used by the subsequent source-qualified heal batch.
    pub fn read_source_creature_health(
        &self,
        actor: ExactActorRef,
        key: &str,
        max: u64,
    ) -> Result<u64, CarrierError> {
        if max == 0 || max > i64::MAX as u64 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation != actor.0.actor_local_generation.0 =>
            {
                Err(CarrierError::StaleActorGeneration)
            }
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == key.as_bytes()
                && *health > 0
                && *health <= max as i64 =>
            {
                Ok(*health as u64)
            }
            _ => Err(CarrierError::CreatureTargetMismatch),
        }
    }
}

// Main reconciliation Foundation-2: intrinsic native summon lifecycle.
impl ChannelRuntimeV1 {
    pub fn matches_live_creature_identity(&self, actor: ExactActorRef, target: &[u8]) -> bool {
        self.current_live_creature_identity(actor)
            .is_ok_and(|identity| identity == target)
    }
}
